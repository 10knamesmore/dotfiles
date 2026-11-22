import {
  replaceControlCharacters,
  stripTerminalSequences,
  visibleWidth,
  wrapTextWithAnsi,
} from "@earendil-works/pi-tui";

interface StyleSpan {
  row: number;
  start: number;
  end: number;
  fg: string | null;
  bg: string | null;
}

export interface Frame {
  lines: string[];
  styles?: StyleSpan[];
}

type TextRun = { text: string; sgr: string };

const graphemes = new Intl.Segmenter(undefined, { granularity: "grapheme" });
const RESET = "\x1b[0m";

function terminalText(text: string): string {
  return replaceControlCharacters(text.replace(/\t/gu, "   ")).replace(/[\u2028\u2029]/gu, " ");
}

function colorSgr(color: string | null, channel: 38 | 48): string {
  if (!color) return "";
  const rgb = Number.parseInt(color.slice(1), 16);
  return `\x1b[${channel};2;${(rgb >> 16) & 255};${(rgb >> 8) & 255};${rgb & 255}m`;
}

function rowRuns(line: string, spans: readonly StyleSpan[]): TextRun[] {
  if (!spans.length) return [{ text: terminalText(line), sgr: "" }];

  // The CLI emits sorted, nonoverlapping UTF-8 byte ranges.
  const bytes = Buffer.from(line);
  const runs: TextRun[] = [];
  let offset = 0;
  const add = (start: number, end: number, sgr: string) => {
    runs.push({ text: terminalText(bytes.subarray(start, end).toString()), sgr });
  };
  for (const { start, end, fg, bg } of spans) {
    if (offset < start) add(offset, start, "");
    add(start, end, colorSgr(fg, 38) + colorSgr(bg, 48));
    offset = end;
  }
  if (offset < bytes.length) add(offset, bytes.length, "");
  return runs;
}

function paint(text: string, sgr: string): string {
  return text && sgr ? sgr + text + RESET : text;
}

function wrapLosslessly(runs: readonly TextRun[], width: number): string[] {
  const rows: string[] = [];
  let row = "";
  let columns = 0;
  for (const { text, sgr } of runs) {
    let chunk = "";
    const append = (glyph: string, cells: number) => {
      if (columns + cells > width) {
        rows.push(row + paint(chunk, sgr));
        row = "";
        columns = 0;
        chunk = "";
      }
      chunk += glyph;
      columns += cells;
    };
    for (const { segment } of graphemes.segment(text)) {
      const cells = visibleWidth(segment);
      if (cells <= width) {
        append(segment, cells);
      } else {
        // A grapheme wider than the viewport remains readable as its code points.
        for (const codePoint of segment) {
          for (const char of `\\u{${codePoint.codePointAt(0)!.toString(16)}}`) append(char, 1);
        }
      }
    }
    row += paint(chunk, sgr);
  }
  rows.push(row);
  return rows;
}

function renderRow(runs: readonly TextRun[], width: number): string[] {
  const line = runs.map(({ text, sgr }) => paint(text, sgr)).join("");
  if (visibleWidth(line) <= width) return [line];
  if (width > 1) {
    const wrapped = wrapTextWithAnsi(line, width);
    // Pi's word wrap may drop layout spaces or return a grapheme wider than the viewport.
    if (
      stripTerminalSequences(wrapped.join("")) === stripTerminalSequences(line) &&
      wrapped.every((row) => visibleWidth(row) <= width)
    ) {
      return wrapped.map((row) => (row.includes("\x1b") && !row.endsWith(RESET) ? row + RESET : row));
    }
  }
  return wrapLosslessly(runs, width);
}

export function renderFrame(frame: Frame, width: number, color: boolean): string[] {
  const styles = (color && frame.styles) || [];
  let next = 0;
  return frame.lines.flatMap((line, row) => {
    const first = next;
    while (styles[next]?.row === row) next++;
    return renderRow(rowRuns(line, styles.slice(first, next)), width);
  });
}

export function renderText(text: string, width: number): string[] {
  return text.split(/\r\n|\r|\n/u).flatMap((line) => renderRow(rowRuns(line, []), width));
}

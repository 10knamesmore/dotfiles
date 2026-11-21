import { getLanguageFromPath, highlightCode, type Theme } from "@earendil-works/pi-coding-agent";
import {
  replaceControlCharacters,
  stripTerminalSequences,
  truncateToWidth,
  wrapTextWithAnsi,
  type Component,
} from "@earendil-works/pi-tui";
import { GUTTER_DIVIDER, lineNumberWidth, type SourceRange } from "./numbered-text.js";

export function renderNumberedSource(
  source: string,
  path: string,
  range: SourceRange,
  theme: Theme,
  notice: Component | undefined,
): Component {
  const clean = stripTerminalSequences(source)
    .replace(/\r/g, "")
    .replace(/\t/g, "   ")
    .split("\n")
    .map(replaceControlCharacters);
  const language = getLanguageFromPath(path);
  const highlighted = language
    ? highlightCode(clean.join("\n"), language)
    : clean.map((line) => theme.fg("toolOutput", line));
  const numberWidth = lineNumberWidth(range);
  const gutterWidth = numberWidth + GUTTER_DIVIDER.length;
  const continuation = theme.fg("dim", " ".repeat(numberWidth) + GUTTER_DIVIDER);
  let cached: { width: number; lines: string[] } | undefined;

  return {
    render(width) {
      if (cached?.width === width) return cached.lines;
      const showGutter = width - gutterWidth >= 2;
      const sourceWidth = showGutter ? width - gutterWidth : width;
      const lines = [""];
      for (const [index, line] of highlighted.entries()) {
        const wrapped =
          sourceWidth < 2 ? [truncateToWidth(line, sourceWidth, "…")] : wrapTextWithAnsi(line, sourceWidth);
        if (showGutter) {
          const number = String(range.firstLine + index).padStart(numberWidth);
          lines.push(
            ...wrapped.map((segment, row) =>
              row === 0 ? theme.fg("dim", number + GUTTER_DIVIDER) + segment : continuation + segment,
            ),
          );
        } else {
          lines.push(...wrapped);
        }
      }
      if (notice) lines.push(...notice.render(width));
      cached = { width, lines };
      return lines;
    },
    invalidate() {
      cached = undefined;
      notice?.invalidate();
    },
  };
}

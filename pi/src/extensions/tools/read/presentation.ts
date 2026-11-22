import { getLanguageFromPath, highlightCode, type Theme } from "@earendil-works/pi-coding-agent";
import {
  hyperlink,
  replaceControlCharacters,
  stripTerminalSequences,
  Text,
  truncateToWidth,
  wrapTextWithAnsi,
  type Component,
} from "@earendil-works/pi-tui";
import { GUTTER_DIVIDER, lineNumberWidth, type SourceRange } from "./numbered-text.js";

// Pi formats both regular and compact read headers through Text.setText().
export class ReadCallHeader extends Text {
  private title = "";

  override setText(title: string): void {
    this.title = title;
    super.setText(title);
  }

  linkToLine(line: number, sourceUrl?: string): void {
    const fileLink = /\x1b\]8;;(file:[^\x1b\x07]*)\x1b\\/.exec(this.title);
    const url = sourceUrl ?? fileLink?.[1];
    if (!url) return;

    const target = `${url}#L${line}`;
    if (fileLink) {
      const prefix = this.title.slice(0, fileLink.index);
      const pathAndRange = this.title.slice(fileLink.index + fileLink[0].length).replace("\x1b]8;;\x1b\\", "");
      super.setText(prefix + hyperlink(pathAndRange, target));
    } else {
      // Compact skill, resource and docs headers end in a parenthesized expand hint.
      const hint = this.title.lastIndexOf(" (");
      super.setText(hyperlink(this.title.slice(0, hint), target) + this.title.slice(hint));
    }
  }
}

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

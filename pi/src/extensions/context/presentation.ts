import type { Theme, ThemeColor } from "@earendil-works/pi-coding-agent";
import {
  mixColors,
  stripTerminalSequences,
  truncateToWidth,
  visibleWidth,
  wrapTextWithAnsi,
  type Color,
  type Component,
} from "@earendil-works/pi-tui";
import type { ReportRow, ReportTone } from "./report.js";
import type { SourceDocument } from "./source-layout.js";

export type PageContent =
  { kind: "report"; rows: ReportRow[] } | ({ kind: "source" } & SourceDocument);

const valueColors: Record<ReportTone, ThemeColor> = {
  value: "text",
  number: "syntaxNumber",
  success: "success",
  warning: "warning",
  error: "error",
};
const changeColors = { added: "success", removed: "error", changed: "warning" } as const;
const changeMarkers = { added: "+", removed: "−", changed: "~" } as const;

export class ContextAppearance {
  public surface!: Color;
  public section!: Color;
  public selected!: Color;
  public stripe!: Color;
  private frameWidth = 0;
  private framedRows = new Map<string, string>();

  public constructor(public readonly theme: Theme) {
    this.refresh();
  }

  public refresh(): void {
    this.surface = this.theme.colors.toolPendingBg;
    this.section = mixColors(this.surface, this.theme.colors.accent, 0.16);
    this.selected = mixColors(this.surface, this.theme.colors.accent, 0.3);
    this.stripe = mixColors(this.surface, this.theme.colors.text, 0.045);
    this.framedRows.clear();
  }

  public fill(text: string, width: number, background = this.surface): string {
    const clipped = truncateToWidth(text, width);
    const padding = " ".repeat(Math.max(0, width - visibleWidth(clipped)));
    return this.theme.style(clipped + padding, { fg: "text", bg: background });
  }

  public panel(body: string[], width: number, title: string, help: string): string[] {
    const inner = Math.max(1, width - 4);
    if (this.frameWidth !== width) this.framedRows.clear();
    this.frameWidth = width;
    const nextFramedRows = new Map<string, string>();
    const frame = (line: string) => {
      const rendered =
        this.framedRows.get(line) ??
        this.fill(`${this.theme.fg("border", "│")} ${this.fill(line, inner)} ${this.theme.fg("border", "│")}`, width);
      nextFramedRows.set(line, rendered);
      return rendered;
    };
    const edge = (left: string, right: string) =>
      this.fill(this.theme.fg("border", `${left}${"─".repeat(Math.max(0, width - 2))}${right}`), width);
    const lines = [
      edge("┌", "┐"),
      frame(
        this.fill(this.theme.style(stripTerminalSequences(title), { fg: "accent", bold: true }), inner, this.selected),
      ),
      ...body.map(frame),
      frame(this.fill(this.theme.fg("muted", help), inner, this.section)),
      edge("└", "┘"),
    ];
    this.framedRows = nextFramedRows;
    return lines;
  }
}

export class ContextReport implements Component {
  private cachedWidth = 0;
  private cachedLines: string[] | undefined;

  public constructor(
    private readonly rows: ReportRow[],
    private readonly appearance: ContextAppearance,
  ) {}

  public render(width: number): string[] {
    if (this.cachedLines && this.cachedWidth === width) return this.cachedLines;
    this.cachedWidth = width;
    this.cachedLines = this.reportLines(this.rows, width);
    return this.cachedLines;
  }

  public invalidate(): void {
    this.cachedLines = undefined;
    this.appearance.refresh();
  }

  private reportLines(rows: ReportRow[], width: number): string[] {
    const { theme } = this.appearance;
    const labelWidth = Math.min(22, Math.floor(width * 0.34));
    const lines: string[] = [];
    let fieldIndex = 0;
    for (const row of rows) {
      if (row.kind === "heading") {
        if (lines.length) lines.push(this.appearance.fill("", width));
        fieldIndex = 0;
        const title = theme.style(`▌ ${stripTerminalSequences(row.text)}`, { fg: "accent", bold: true });
        for (const line of wrapTextWithAnsi(title, width)) {
          lines.push(this.appearance.fill(line, width, this.appearance.section));
        }
      } else if (row.kind === "field") {
        const background = fieldIndex++ % 2 === 0 ? this.appearance.stripe : this.appearance.surface;
        const label = stripTerminalSequences(row.label);
        const value = theme.style(stripTerminalSequences(row.value), { fg: valueColors[row.tone], bold: true });
        const detail = row.detail ? `  ${theme.fg("muted", stripTerminalSequences(row.detail))}` : "";
        const inline = visibleWidth(label) <= labelWidth;
        if (!inline) {
          for (const line of wrapTextWithAnsi(theme.fg("muted", label), Math.max(1, width - 2))) {
            lines.push(this.appearance.fill(` ${line}`, width, background));
          }
        }
        const indent = inline ? labelWidth + 2 : 2;
        const valueLines = wrapTextWithAnsi(value + detail, Math.max(1, width - indent));
        valueLines.forEach((line, index) => {
          const prefix =
            inline && index === 0
              ? ` ${theme.fg("muted", label)}${" ".repeat(labelWidth - visibleWidth(label) + 1)}`
              : " ".repeat(indent);
          lines.push(this.appearance.fill(prefix + line, width, background));
        });
      } else if (row.kind === "change") {
        const color = changeColors[row.operation];
        const background = mixColors(this.appearance.surface, theme.colors[color], 0.1);
        const text = theme.fg(color, `${changeMarkers[row.operation]} ${stripTerminalSequences(row.text)}`);
        for (const line of wrapTextWithAnsi(text, Math.max(1, width - 2))) {
          lines.push(this.appearance.fill(` ${line}`, width, background));
        }
      } else {
        for (const line of wrapTextWithAnsi(
          theme.fg("muted", stripTerminalSequences(row.text)),
          Math.max(1, width - 2),
        )) {
          lines.push(this.appearance.fill(` ${line}`, width));
        }
      }
    }
    return lines;
  }
}

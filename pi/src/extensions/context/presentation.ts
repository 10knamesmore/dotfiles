import type { Theme } from "@earendil-works/pi-coding-agent";
import {
  mixColors,
  stripTerminalSequences,
  truncateToWidth,
  visibleWidth,
  wrapTextWithAnsi,
  type Color,
  type Component,
} from "@earendil-works/pi-tui";
import type { ReportSection, TableCell, TableColumn } from "./report.js";
import type { SourceDocument } from "./source-layout.js";

export type PageContent = { kind: "report"; sections: ReportSection[] } | ({ kind: "source" } & SourceDocument);

const columnDivider = " │ ";

export function tableLayout(width: number, columns: readonly TableColumn[]) {
  let remainingWidth = Math.max(columns.length, width - (columns.length - 1) * columnDivider.length);
  let remainingWeight = columns.reduce((sum, column) => sum + column.weight, 0);
  return columns.map((column, index) => {
    const available = remainingWidth - (columns.length - index - 1);
    const width = Math.max(1, Math.min(available, Math.floor((remainingWidth * column.weight) / remainingWeight)));
    remainingWidth -= width;
    remainingWeight -= column.weight;
    return { ...column, width };
  });
}

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

  public tableRow(
    layout: ReturnType<typeof tableLayout>,
    cells: readonly TableCell[],
    width: number,
    background = this.surface,
    wrap = false,
  ): string[] {
    const columns = layout.map((column, index) => {
      const cell = cells[index] ?? "";
      const { text, color, bold } = typeof cell === "string" ? { text: cell, color: undefined, bold: undefined } : cell;
      const clean = stripTerminalSequences(text);
      const lines = wrap ? wrapTextWithAnsi(clean, column.width) : [clean];
      return { ...column, lines, color, bold };
    });
    const divider = this.theme.fg("border", columnDivider);
    const height = Math.max(1, ...columns.map(({ lines }) => lines.length));
    return Array.from({ length: height }, (_, row) => {
      const line = columns.map((column) => {
        const text = truncateToWidth(column.lines[row] ?? "", column.width, "…");
        const padding = " ".repeat(column.width - visibleWidth(text));
        const aligned = column.align === "right" ? padding + text : text + padding;
        return this.theme.style(aligned, { fg: column.color ?? "text", bold: column.bold });
      });
      return this.fill(line.join(divider), width, background);
    });
  }

  public panel(body: string[], width: number, selection?: string): string[] {
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
    const edge = (left: string, right: string, counter = "") => {
      const inset = counter && width >= counter.length + 4 ? ` ${counter} ` : "";
      const rule = "─".repeat(Math.max(0, width - 2 - inset.length));
      const border = inset
        ? this.theme.fg("border", left + rule) + this.theme.fg("syntaxNumber", inset) + this.theme.fg("border", right)
        : this.theme.fg("border", left + rule + right);
      return this.fill(border, width);
    };
    const lines = [edge("╭", "╮"), ...body.map(frame), edge("╰", "╯", selection)];
    this.framedRows = nextFramedRows;
    return lines;
  }
}

export class ContextReport implements Component {
  private cachedWidth = 0;
  private cachedLines: string[] | undefined;

  public constructor(
    private readonly sections: ReportSection[],
    private readonly appearance: ContextAppearance,
  ) {}

  public render(width: number): string[] {
    if (this.cachedLines && this.cachedWidth === width) return this.cachedLines;
    this.cachedWidth = width;
    this.cachedLines = this.reportLines(width);
    return this.cachedLines;
  }

  public invalidate(): void {
    this.cachedLines = undefined;
    this.appearance.refresh();
  }

  private reportLines(width: number): string[] {
    const { theme } = this.appearance;
    const lines: string[] = [];
    for (const section of this.sections) {
      if (lines.length) lines.push(this.appearance.fill("", width));
      const title = theme.style(section.title, { fg: "accent", bold: true });
      lines.push(this.appearance.fill(title, width, this.appearance.selected));
      if (section.rows.length) {
        const layout = tableLayout(width, section.columns);
        const headings = section.columns.map(({ heading }): TableCell => ({
          text: heading,
          color: "muted",
          bold: true,
        }));
        lines.push(...this.appearance.tableRow(layout, headings, width, this.appearance.section));
        section.rows.forEach((row, index) => {
          const background = index % 2 === 0 ? this.appearance.stripe : this.appearance.surface;
          lines.push(...this.appearance.tableRow(layout, row, width, background, true));
        });
      }
      if (section.note) {
        for (const line of wrapTextWithAnsi(theme.fg("muted", section.note), width)) {
          lines.push(this.appearance.fill(line, width));
        }
      }
    }
    return lines;
  }
}

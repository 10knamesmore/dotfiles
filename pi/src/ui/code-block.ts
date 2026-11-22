import { copyToClipboard, type Theme } from "@earendil-works/pi-coding-agent";
import {
  Container,
  mixColors,
  replaceControlCharacters,
  rgbColor,
  stripTerminalSequences,
  truncateToWidth,
  visibleWidth,
  wrapTextWithAnsi,
  type Component,
  type TuiMouseEvent,
  type TuiMouseEventResult,
} from "@earendil-works/pi-tui";
import { ACTION_STYLES, COPIED_LABEL, COPY_LABEL, FEEDBACK_DURATION_MS } from "./action-feedback.ts";

/** Apply Pi's Markdown margin without measuring or padding over Kitty image placements. */
export class PaddedCodeBlock extends Container {
  private cached?: { source: readonly string[]; prefix: string; lines: readonly string[] };

  constructor(
    component: Component,
    private readonly paddingX: number,
    private readonly paint?: (line: string) => string,
  ) {
    super();
    this.addChild(component);
  }

  override render(width: number): readonly string[] {
    const inner = Math.max(1, width - 2 * this.paddingX);
    const source = super.render(inner);
    const prefix = " ".repeat(Math.min(this.paddingX, Math.max(0, width - inner)));
    if (!prefix && !this.paint) return source;
    if (source !== this.cached?.source || prefix !== this.cached.prefix) {
      this.cached = { source, prefix, lines: source.map((line) => prefix + (this.paint ? this.paint(line) : line)) };
    }
    return this.cached.lines;
  }

  override handleMouse(event: TuiMouseEvent): ReturnType<Container["handleMouse"]> {
    const width = Math.max(1, event.width - 2 * this.paddingX);
    const x = event.x - Math.min(this.paddingX, Math.max(0, event.width - width));
    if (x < 0 || x >= width) return undefined;
    return super.handleMouse({ ...event, x, width });
  }
}

const bodyInset = (width: number) => (width >= 5 ? 2 : width >= 3 ? 1 : 0);

export function codeBlockBodyWidth(width: number): number {
  return Math.max(1, width - 2 * bodyInset(width));
}

/** Clears the previous frame's hover when the pointer moves directly between blocks. */
export interface CodeBlockPointer {
  leave?: () => void;
}

export interface CodeBlockTarget {
  id: string;
  rect: { x: number; y: number; w: number; h: number };
  cells?: readonly (readonly [number, number])[];
}

function targetAt<T extends CodeBlockTarget>(
  targets: readonly T[],
  x: number,
  y: number,
  enabled?: (target: T) => boolean,
): T | undefined {
  let nearest: T | undefined;
  let smallestArea = Infinity;
  for (const target of targets) {
    if (enabled && !enabled(target)) continue;
    if (target.cells?.some(([column, row]) => column === x && row === y)) return target;
    const rect = target.rect;
    if (x < rect.x || x >= rect.x + rect.w || y < rect.y || y >= rect.y + rect.h) continue;
    const area = rect.w * rect.h;
    if (area < smallestArea) {
      nearest = target;
      smallestArea = area;
    }
  }
  return nearest;
}

export interface CodeBlockControl extends Omit<CodeBlockTarget, "cells"> {
  kind: "source" | "copy" | "fold";
  label: string;
  control: true;
}

type OpenView = "rendered" | "source";
type CodeBlockView = OpenView | "hidden";

export interface CodeBlockBodyAppearance<T extends CodeBlockTarget> {
  hovered: T | undefined;
  control: CodeBlockControl | undefined;
}

interface BodyLines {
  lines: readonly string[];
  /** These rows already fill the body width, including cursor advances over terminal images. */
  imageRows?: ReadonlySet<number>;
}

type CopyText = string | (() => string | undefined);
export type CodeBlockBody<T extends CodeBlockTarget> =
  | (BodyLines & { copyText?: CopyText })
  | { targets: readonly T[]; draw(appearance: CodeBlockBodyAppearance<T>): BodyLines; copyText: CopyText };

interface FrameOptions<T extends CodeBlockTarget> {
  title: string;
  theme: Theme;
  source(): string;
  renderBody(width: number): CodeBlockBody<T>;
  requestRender(): void;
  pointer: CodeBlockPointer;
  copy?: typeof copyToClipboard;
  interaction?: {
    enabled(target: T): boolean;
    activate(target: T, shift: boolean): void;
    viewChanged(): void;
    viewNotice?(): string | undefined;
  };
}

interface ControlLabel {
  kind: CodeBlockControl["kind"];
  label: string;
  text: string;
  short: string;
}

function headerLayout(width: number, controls: readonly ControlLabel[]) {
  const room = Math.max(1, width - 2);
  const labels = controls.map((control) => ({
    ...control,
    text: room < 5 ? control.short : truncateToWidth(control.text, room),
  }));
  const total = labels.reduce((sum, control) => sum + visibleWidth(control.text) + 1, -1);
  const inline = room >= total + 10;
  const start = Math.min(1, width - room);
  let x = inline ? width - total - 1 : start;
  let y = inline ? 0 : 1;
  const targets = labels.map(({ kind, label, text }) => {
    const w = visibleWidth(text);
    if (x + w > start + room) {
      x = start;
      y++;
    }
    const result = { id: `frame:${kind}`, kind, label, control: true as const, rect: { x, y, w, h: 1 }, text };
    x += w + 1;
    return result;
  });
  return { targets, titleWidth: inline ? room - total - 1 : width - 2, height: y + 1 };
}

const borderRule = (width: number) => "─".repeat(Math.max(0, width));

function renderHeader(
  width: number,
  title: string,
  header: ReturnType<typeof headerLayout>,
  border: (text: string) => string,
  style: (control: CodeBlockControl, text: string) => string,
): string[] {
  const name = truncateToWidth(`─ ${title} `, Math.max(0, header.titleWidth), "");
  return Array.from({ length: header.height }, (_, y) => {
    const side = width >= 3;
    let row = y === 0 ? border("╭" + name) : side ? border("│") : "";
    let x = visibleWidth(row);
    for (const target of header.targets) {
      if (target.rect.y !== y) continue;
      const gap = target.rect.x - x;
      row += y === 0 ? border(borderRule(gap)) : " ".repeat(gap);
      row += style(target, target.text);
      x = target.rect.x + target.rect.w;
    }
    if (y === 0) return width > 1 ? row + border(borderRule(width - x - 1) + "╮") : row;
    return side ? row + " ".repeat(width - x - 1) + border("│") : row;
  });
}

function isControl<T extends CodeBlockTarget>(target: T | CodeBlockControl): target is CodeBlockControl {
  return "control" in target && target.control === true;
}

export class CodeBlockFrame<T extends CodeBlockTarget = never> {
  private openView: OpenView = "rendered";
  private hidden = false;
  private copyFeedback: { view: OpenView; copied: boolean } | undefined;
  private copyTimer: ReturnType<typeof setTimeout> | undefined;
  private copyAttempt = 0;
  private hovered: string | null = null;
  private width: number | undefined;
  private body: CodeBlockBody<T> | undefined;
  private bodyTop = 0;
  private bodyTargets: readonly T[] = [];
  private controls: CodeBlockControl[] = [];
  private lines: string[] | undefined;
  private disposed = false;

  constructor(private readonly options: FrameOptions<T>) {}

  private readonly leave = () => {
    this.options.pointer.leave = undefined;
    this.hovered = null;
    this.redraw();
  };

  invalidate(): void {
    this.body = undefined;
    this.lines = undefined;
  }

  private redraw(): void {
    this.lines = undefined;
    if (!this.disposed) this.options.requestRender();
  }

  private get view(): CodeBlockView {
    return this.hidden ? "hidden" : this.openView;
  }

  private renderedBody(width: number): CodeBlockBody<T> {
    return (this.body ??= this.options.renderBody(width));
  }

  copyRenderedText(): string | undefined {
    if (this.width === undefined || this.disposed) return undefined;
    try {
      const body = this.renderedBody(codeBlockBodyWidth(this.width));
      if (typeof body.copyText === "function") return body.copyText();
      return body.copyText ?? ("lines" in body ? body.lines.map(stripTerminalSequences).join("\n") : undefined);
    } catch {
      return undefined;
    }
  }

  render(width: number): string[] {
    if (this.disposed || width <= 0) return [];
    if (this.width !== width) {
      this.invalidate();
      this.width = width;
    }
    if (this.lines) return this.lines;
    try {
      return (this.lines = this.draw(width));
    } catch (error) {
      this.controls = [];
      this.bodyTargets = [];
      const message = String(error instanceof Error ? error.message : error)
        .replace(/\s+/g, " ")
        .slice(0, 120);
      return [
        truncateToWidth(this.options.theme.fg("error", `! Could not draw ${this.options.title}: ${message}`), width),
      ];
    }
  }

  private draw(width: number): string[] {
    const { theme, title } = this.options;
    const available = codeBlockBodyWidth(width);
    const view = this.view;
    const feedback = this.copyFeedback?.view === view ? this.copyFeedback : undefined;
    const borderColor = mixColors(theme.colors.muted, rgbColor(0, 0, 0), 0.4, "srgb");
    const border = (text: string) => theme.style(text, { fg: borderColor });
    const inset = bodyInset(width);
    const left = inset ? border(inset === 2 ? "│ " : "│") : "";
    const right = inset ? border(inset === 2 ? " │" : "│") : "";
    const bodyRow = (text: string, image = false) =>
      left + (image ? text : truncateToWidth(text, available, "…", true)) + right;
    const labels: ControlLabel[] = [
      {
        kind: "source",
        label: view === "source" ? "show rendered block" : "view source",
        text: view === "source" ? "-[rendered]-" : "-[source]-",
        short: view === "source" ? "R" : "S",
      },
    ];
    if (view !== "hidden")
      labels.push({
        kind: "copy",
        label: view === "source" ? "copy source" : "copy rendered block as text",
        text: feedback?.copied ? COPIED_LABEL : COPY_LABEL,
        short: feedback?.copied ? "✓" : "C",
      });
    labels.push({
      kind: "fold",
      label: view === "hidden" ? "show block" : "hide block",
      text: view === "hidden" ? "-[show]-" : "-[hide]-",
      short: view === "hidden" ? "+" : "-",
    });
    const header = headerLayout(width, labels);
    this.bodyTop = header.height;
    this.controls = header.targets;
    const style = (control: CodeBlockControl, text: string) => {
      if (control.kind === "copy" && feedback?.copied) return theme.style(text, ACTION_STYLES.success);
      const active = control.id === this.hovered;
      return theme.style(text, ACTION_STYLES[active ? "active" : "idle"]);
    };
    const lines = renderHeader(width, theme.fg("borderMuted", title), header, border, style);
    this.bodyTargets = [];
    if (view !== "hidden") {
      const body = view === "source" ? { lines: this.sourceLines(available) } : this.renderedBody(available);
      let drawing: BodyLines;
      if ("draw" in body) {
        this.bodyTargets = body.targets;
        drawing = body.draw({
          hovered: body.targets.find((target) => target.id === this.hovered),
          control: this.controls.find((target) => target.id === this.hovered),
        });
      } else {
        drawing = body;
      }
      drawing.lines.forEach((line, row) => lines.push(bodyRow(line, drawing.imageRows?.has(row))));
    }
    if (view !== "rendered") {
      const notice = this.options.interaction?.viewNotice?.();
      if (notice) lines.push(bodyRow(notice));
    }
    if (feedback && !feedback.copied) {
      lines.push(bodyRow(theme.fg("error", "Could not write to the clipboard.")));
    }
    lines.push(border(width > 1 ? `╰${borderRule(width - 2)}╯` : "╰"));
    return lines;
  }

  private sourceLines(width: number): string[] {
    const { source, theme } = this.options;
    return source()
      .split(/\r\n|\n|\r/)
      .flatMap((line) => {
        const code = replaceControlCharacters(line.replaceAll("\t", "  "));
        return wrapTextWithAnsi(theme.fg("mdCode", code), width);
      });
  }

  private hit(x: number, y: number): T | CodeBlockControl | undefined {
    if (y < this.bodyTop) return targetAt(this.controls, x, y);
    const width = this.width!;
    x -= bodyInset(width);
    if (x < 0 || x >= codeBlockBodyWidth(width)) return undefined;
    return targetAt(this.bodyTargets, x, y - this.bodyTop, this.options.interaction?.enabled);
  }

  handleMouse(event: TuiMouseEvent): TuiMouseEventResult | undefined {
    if (this.disposed || this.width === undefined) return undefined;
    try {
      const target = this.hit(event.x, event.y);
      if (event.type === "move" || event.type === "press") {
        const pointer = this.options.pointer;
        if (pointer.leave !== this.leave) pointer.leave?.();
        pointer.leave = target ? this.leave : undefined;
        const id = target?.id ?? null;
        if (id !== this.hovered) {
          this.hovered = id;
          this.redraw();
        }
      }
      if (!target) return undefined;
      if (event.type === "move") return { handled: true, render: false };
      if (event.button !== "left") return undefined;
      if (event.type === "press") return { handled: true, render: false };
      if (event.type === "click") {
        this.activate(target, event.shift ?? false);
        this.redraw();
        return { handled: true, render: true };
      }
    } catch {
      return undefined;
    }
    return undefined;
  }

  private activate(target: T | CodeBlockControl, shift: boolean): void {
    if (!isControl(target)) {
      this.options.interaction?.activate(target, shift);
      return;
    }
    switch (target.kind) {
      case "copy":
        void this.copyActiveText();
        break;
      case "fold":
        this.hidden = !this.hidden;
        break;
      case "source":
        this.openView = this.hidden || this.openView === "rendered" ? "source" : "rendered";
        this.hidden = false;
        break;
    }
    if (target.kind !== "copy") {
      this.hovered = null;
      this.controls = [];
      this.bodyTargets = [];
      this.options.interaction?.viewChanged();
    }
  }

  private async copyActiveText(): Promise<void> {
    if (this.hidden) return;
    const view = this.openView;
    const text = view === "source" ? this.options.source() : this.copyRenderedText();
    if (text === undefined) return;
    const attempt = ++this.copyAttempt;
    let copied = false;
    try {
      await (this.options.copy ?? copyToClipboard)(text);
      copied = true;
    } catch {
      // Display the result of the latest attempt below the block.
    }
    if (this.disposed || attempt !== this.copyAttempt) return;
    clearTimeout(this.copyTimer);
    this.copyFeedback = { view, copied };
    this.copyTimer = setTimeout(() => {
      this.copyFeedback = undefined;
      this.redraw();
    }, FEEDBACK_DURATION_MS);
    this.copyTimer.unref();
    this.redraw();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    clearTimeout(this.copyTimer);
    if (this.options.pointer.leave === this.leave) this.options.pointer.leave = undefined;
    this.invalidate();
    this.bodyTargets = [];
    this.controls = [];
  }
}

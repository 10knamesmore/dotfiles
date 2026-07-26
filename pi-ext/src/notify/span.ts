/**
 * 行内文本片段（span）的样式与三段布局。
 *
 * 一行可以是裸字符串（整行默认样式），也可以是若干 span 混排；span 除自身样式外
 * 还带 align，把同一行分成左/中/右三段，段内按原顺序连排。
 */

import { visibleWidth } from "@earendil-works/pi-tui";

import { type Modifiers, type Rgb, paint, parseFg } from "./theme.ts";

export interface Span extends Modifiers {
  text: string;
  /** 角色名（text/subtext/overlay/accent/red/yellow/green/peach）或 `#rrggbb`。 */
  fg?: string;
  align?: "left" | "center" | "right";
}

/** 一行内容：整行默认样式的裸字符串，或按 span 混排。 */
export type Line = string | (string | Span)[];

/** 同段之间的最小间隙（字符），量宽时按段界数累加，保证三段默认不重叠。 */
export const GROUP_GAP = 2;

function toSpan(item: string | Span): Span {
  return typeof item === "string" ? { text: item } : item;
}

export function toSpans(line: Line): Span[] {
  return (typeof line === "string" ? [line] : line).map(toSpan);
}

/** 单个 span 着色：无 fg 时落到语境默认色（标题是级别色、body 是正文色）。 */
export function renderSpan(span: Span, defaultFg: Rgb): string {
  return paint(span.text, span.fg ? parseFg(span.fg) : defaultFg, span);
}

interface GroupWidths {
  left: number;
  center: number;
  right: number;
}

function groupWidths(spans: Span[]): GroupWidths {
  const w: GroupWidths = { left: 0, center: 0, right: 0 };
  for (const span of spans) w[span.align ?? "left"] += visibleWidth(span.text);
  return w;
}

/**
 * 一行的最小所需宽：各段宽之和 + 非空段间隙 × 段界数。
 * 卡片按它量宽，因此正常情况下三段不会相互挤压。
 */
export function lineWidth(line: Line): number {
  const w = groupWidths(toSpans(line));
  const groups = Number(w.left > 0) + Number(w.center > 0) + Number(w.right > 0);
  return w.left + w.center + w.right + GROUP_GAP * Math.max(0, groups - 1);
}

/** 一个显示格：已着色的字符、被宽字符占据的续格、或空。 */
type Cell = { text: string } | "cont" | null;

/**
 * 把一段 spans 从 `start` 列写进 cells，逐字符按显示宽度推进。
 * 后写的覆盖先写的——与叠画同语义；宽字符被覆盖半格时另半格退化成空格。
 */
function writeSpans(cells: Cell[], spans: Span[], start: number, defaultFg: Rgb): void {
  let col = start;
  for (const span of spans) {
    for (const ch of span.text) {
      const w = visibleWidth(ch);
      if (col + w > cells.length) return;
      if (col >= 0) {
        cells[col] = { text: paint(ch, span.fg ? parseFg(span.fg) : defaultFg, span) };
        for (let i = 1; i < w; i++) cells[col + i] = "cont";
      }
      col += w;
    }
  }
}

/**
 * 按 align 把一行 spans 排进 width：左段贴左、中段居中于整行、右段贴右。
 *
 * 产出恰好占满 width 的一行（宿主 TUI 要求每行不得超过 width）。
 */
export function layoutLine(line: Line, width: number, defaultFg: Rgb): string {
  if (width <= 0) return "";

  const spans = toSpans(line);
  const cells: Cell[] = new Array(width).fill(null);
  const w = groupWidths(spans);
  const pick = (align: Span["align"]) => spans.filter((s) => (s.align ?? "left") === align);

  writeSpans(cells, pick("left"), 0, defaultFg);
  writeSpans(cells, pick("center"), Math.floor((width - w.center) / 2), defaultFg);
  writeSpans(cells, pick("right"), width - w.right, defaultFg);

  let out = "";
  for (const cell of cells) {
    if (cell === "cont") continue;
    out += cell === null ? " " : cell.text;
  }
  return out;
}

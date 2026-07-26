/**
 * 卡片画框：圆角边框 + 标题 + 多行 body + 底边提示 + ttl 倒计时蔓延。
 *
 * 纯渲染、无状态、不碰宿主——只读通知（card.ts）与交互审批（ask.ts）共用同一套
 * 外观，改边框样式只需动这里一处。
 */

import { visibleWidth } from "@earendil-works/pi-tui";

import { type Line, type Span, layoutLine, lineWidth, renderSpan, toSpans } from "./span.ts";
import { PALETTE, type Rgb, lerpColor, paint } from "./theme.ts";

export type CardKind = "info" | "warn" | "error";

export interface FrameOptions {
  /** 画进上边框的标题（不含级别符号，符号按 kind 补）。 */
  title?: Line;
  /** 每项一行；裸字符串里的 \n 拆成多行。 */
  body: Line[];
  kind?: CardKind;
  /** 底边右下角的提示（关闭键 / 操作键）；缺省不画。 */
  closeHint?: string;
}

const BORDER = { tl: "╭", tr: "╮", bl: "╰", br: "╯", h: "─", v: "│" };

/** 级别 → 边框 / 标题色。 */
export function accentOf(kind: CardKind | undefined): Rgb {
  if (kind === "error") return PALETTE.red;
  if (kind === "warn") return PALETTE.yellow;
  return PALETTE.accent;
}

/** 级别 → 角色名（写进 span 的 fg 用；与 accentOf 同源）。 */
export function accentNameOf(kind: CardKind | undefined): string {
  if (kind === "error") return "red";
  if (kind === "warn") return "yellow";
  return "accent";
}

/** 级别 → 标题前缀符号（info 无符号）。 */
export function symbolOf(kind: CardKind | undefined): string {
  if (kind === "error") return "✗ ";
  if (kind === "warn") return "⚠ ";
  return "";
}

/** 展开 body：裸字符串里的换行拆成独立行，span 数组原样保留。 */
export function expandBody(body: Line[]): Line[] {
  const out: Line[] = [];
  for (const line of body) {
    if (typeof line === "string" && line.includes("\n")) out.push(...line.split("\n"));
    else out.push(line);
  }
  return out;
}

/**
 * 完全展开时的总宽：按标题 / body / 底边提示取最宽。
 * 标题与提示自带前后各一空格（+2）；body 之外再加边框 2 + 左右 padding 2。
 */
export function frameWidth(opts: FrameOptions, body: Line[]): number {
  const title = (opts.title ? lineWidth(opts.title) : 0) + visibleWidth(symbolOf(opts.kind));
  const bodyW = body.reduce((max, line) => Math.max(max, lineWidth(line)), 0);
  const hint = visibleWidth(opts.closeHint ?? "");
  return Math.max(Math.max(title, hint) + 2, bodyW) + 4;
}

/**
 * 带 ttl 的边框倒计时：暗色自左上角沿两条路径同时蔓延——顺时针经顶边 → 右边、
 * 逆时针经左边 → 底边——于右下角汇合熄灭，扫一眼边框即知剩余时长。
 *
 * 前沿不是硬边，而是一条占单路径 1/4 的渐变带：级别色到暗端的色距大，若只在前沿
 * 单格插值，每格会在 ttl/周长 的瞬间完成整段变色，肉眼就是逐格跳变；空间上拉开
 * 梯度后，每格的变暗被摊到整条带通过的时长。
 *
 * 返回按坐标取边框色的函数（progress 为千分比）。
 */
function makeDecay(width: number, height: number, progress: number, bright: Rgb) {
  const len = width - 1 + (height - 1);
  const band = Math.max(Math.floor((len * 1000) / 4), 2000);
  const front = Math.floor((Math.min(1000, Math.max(0, progress)) * (len * 1000 + band)) / 1000);

  /** 该格在两条路径上的距离：左上角 d=0，右下角 d=len。 */
  const distance = (x: number, y: number): number => {
    if (y === 0) return x; // 顶边（含两上角）：顺时针路径起始段
    if (x === width - 1) return width - 1 + y; // 右边（含右下角）接顶边之后
    if (x === 0) return y; // 左边（含左下角）：逆时针路径起始段
    return height - 1 + x; // 底边内段接左边之后，向右与右下角汇合
  };

  return (x: number, y: number): Rgb => {
    if (progress <= 0) return bright;
    const cov = Math.min(band, Math.max(0, front - distance(x, y) * 1000));
    if (cov === 0) return bright;
    return lerpColor(bright, PALETTE.surface1, cov, band);
  };
}

/**
 * 画一张卡片，按内容量宽并居中于 `avail`。
 *
 * @param progress ttl 消耗千分比；0 = 驻留 / 刚出现，边框常亮。
 */
export function renderFrame(opts: FrameOptions, avail: number, progress = 0): string[] {
  const body = expandBody(opts.body);
  const width = Math.min(frameWidth(opts, body), avail);
  // 边框 2 + padding 2 都放不下就不画框，退化成纯文本
  if (width < 5) return body.map((line) => layoutLine(line, avail, PALETTE.text));

  const accent = accentOf(opts.kind);
  const symbol = symbolOf(opts.kind);
  const inner = width - 4; // 去掉边框与左右 padding
  const height = body.length + 2;
  const colorAt = makeDecay(width, height, progress, accent);
  const edge = (x: number, y: number, ch: string) => paint(ch, colorAt(x, y));

  const lines: string[] = [];

  // 上边框：` 符号 + 标题 spans `，缺省色 = 级别色，span 自带 fg 可覆盖。
  // 标题文字画在边框上但不参与蔓延（只重染边框字形）。
  let top = edge(0, 0, BORDER.tl);
  let col = 1;
  if (opts.title || symbol) {
    const heading: Span[] = [{ text: ` ${symbol}` }, ...toSpans(opts.title ?? []), { text: " " }];
    const headW = heading.reduce((sum, s) => sum + visibleWidth(s.text), 0);
    if (headW <= width - 2) {
      top += heading.map((s) => renderSpan(s, accent)).join("");
      col += headW;
    }
  }
  for (; col < width - 1; col++) top += edge(col, 0, BORDER.h);
  top += edge(width - 1, 0, BORDER.tr);
  lines.push(top);

  body.forEach((line, i) => {
    const y = i + 1;
    lines.push(
      edge(0, y, BORDER.v) +
        " " +
        layoutLine(line, inner, PALETTE.text) +
        " " +
        edge(width - 1, y, BORDER.v),
    );
  });

  // 下边框：提示右对齐，overlay 色，同样不参与蔓延。
  const y = height - 1;
  let bottom = edge(0, y, BORDER.bl);
  const hint = opts.closeHint ? ` ${opts.closeHint} ` : "";
  const hintW = visibleWidth(hint);
  const fits = hintW > 0 && hintW <= width - 2;
  const fill = width - 2 - (fits ? hintW : 0);
  for (let x = 1; x <= fill; x++) bottom += edge(x, y, BORDER.h);
  if (fits) bottom += paint(hint, PALETTE.overlay);
  bottom += edge(width - 1, y, BORDER.br);
  lines.push(bottom);

  // 卡片按内容量宽，剩余空间左右均分使其居中
  const indent = " ".repeat(Math.floor((avail - width) / 2));
  return lines.map((line) => indent + line);
}

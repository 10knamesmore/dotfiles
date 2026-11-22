// Layout helpers for blocks: rows of cells with a fixed width.
import { cell, EMPTY, gWidth, graphemes, textWidth, truncateText, type Cell, type Rect } from "../diagram/canvas.ts";
import type { Block, Target } from "./types.ts";

export { cell, EMPTY };

export function textCells(s: string, role = ""): Cell[] {
  const out: Cell[] = [];
  for (const g of graphemes(s)) {
    const w = gWidth(g);
    if (w === 0) continue;
    out.push(cell(g, role));
    if (w === 2) out.push(cell("", role));
  }
  return out;
}

export function padRow(row: Cell[], w: number): Cell[] {
  if (row.length === w) return row;
  if (row.length > w) {
    const cut = row.slice(0, w);
    if (cut.length && cut[cut.length - 1].ch !== "" && w < row.length && row[w].ch === "") cut[cut.length - 1] = EMPTY;
    return cut;
  }
  return row.concat(Array.from({ length: w - row.length }, () => EMPTY));
}

export const blank = (w: number, h: number): Block => ({
  w,
  rows: Array.from({ length: h }, () => Array.from({ length: w }, () => EMPTY)),
  targets: [],
});

export function fromRows(w: number, rows: Cell[][], targets: Target[] = []): Block {
  return { w, rows: rows.map((r) => padRow(r, w)), targets };
}

export interface Run {
  t: string;
  role: string;
}

/** Word-wrap styled runs to `width` columns. Explicit "\n" starts a new line. */
export function wrapRuns(runs: Run[], width: number): Cell[][] {
  width = Math.max(1, width);
  const lines: Cell[][] = [[]];
  let col = 0;
  const cur = () => lines[lines.length - 1];
  const newline = () => {
    lines.push([]);
    col = 0;
  };
  for (const run of runs) {
    const parts = run.t.split("\n");
    parts.forEach((part, pi) => {
      if (pi > 0) newline();
      // split into words and spaces
      for (const tok of part.match(/\s+|\S+/g) ?? []) {
        const isSpace = /^\s+$/.test(tok);
        const tw = textWidth(tok);
        if (isSpace) {
          if (col === 0 || col + tw > width) continue; // no leading spaces, no trailing overflow
          cur().push(...textCells(tok, run.role));
          col += tw;
          continue;
        }
        if (col + tw > width && col > 0) {
          // trim one trailing space
          while (cur().length && cur()[cur().length - 1].ch === " ") cur().pop();
          newline();
        }
        if (tw <= width) {
          cur().push(...textCells(tok, run.role));
          col += tw;
        } else {
          // break a long word
          for (const g of graphemes(tok)) {
            const gw = gWidth(g);
            if (col + gw > width) newline();
            cur().push(...textCells(g, run.role));
            col += gw;
          }
        }
      }
    });
  }
  while (lines.length > 1 && lines[lines.length - 1].length === 0) lines.pop();
  for (const l of lines) while (l.length && l[l.length - 1].ch === " ") l.pop();
  return lines;
}

export const wrapText = (s: string, width: number, role = ""): Cell[][] => wrapRuns([{ t: s, role }], width);

export function shiftTargets(ts: Target[], dx: number, dy: number): Target[] {
  return ts.map((t) => ({
    ...t,
    rect: { ...t.rect, x: t.rect.x + dx, y: t.rect.y + dy },
    ...(t.cells ? { cells: t.cells.map(([x, y]) => [x + dx, y + dy] as [number, number]) } : {}),
  }));
}

export function vstack(blocks: Block[], gap: number, w?: number): Block {
  const width = w ?? Math.max(0, ...blocks.map((b) => b.w));
  const rows: Cell[][] = [];
  const targets: Target[] = [];
  blocks.forEach((b, i) => {
    if (!b.rows.length) return;
    if (rows.length && gap > 0) for (let g = 0; g < gap; g++) rows.push(Array.from({ length: width }, () => EMPTY));
    targets.push(...shiftTargets(b.targets, 0, rows.length));
    for (const r of b.rows) rows.push(padRow(r, width));
    void i;
  });
  return { w: width, rows, targets };
}

export function hstack(blocks: Block[], gap: number): Block {
  const h = Math.max(0, ...blocks.map((b) => b.rows.length));
  const w = blocks.reduce((a, b, i) => a + b.w + (i ? gap : 0), 0);
  const rows: Cell[][] = Array.from({ length: h }, () => []);
  const targets: Target[] = [];
  let x = 0;
  blocks.forEach((b, i) => {
    if (i) for (const r of rows) for (let g = 0; g < gap; g++) r.push(EMPTY);
    for (let y = 0; y < h; y++) rows[y].push(...(b.rows[y] ?? Array.from({ length: b.w }, () => EMPTY)));
    targets.push(...shiftTargets(b.targets, x, 0));
    x += b.w + gap;
  });
  return { w, rows, targets };
}

export interface BoxOpts {
  title?: string;
  role: string;
  titleRole?: string;
  /** text drawn in the top border on the right */
  tag?: string;
  tagRole?: string;
  padX?: number;
  dashed?: boolean;
}

/** Wrap a block in a rounded box. `inner` is placed with padX columns of padding. Returns the boxed block. */
export function boxed(inner: Block, opts: BoxOpts): Block {
  const padX = opts.padX ?? 1;
  const w = inner.w + 2 + 2 * padX;
  const role = opts.role;
  const hz = opts.dashed ? "╌" : "─";
  const vt = opts.dashed ? "╎" : "│";
  const top: Cell[] = [cell("╭", role)];
  for (let i = 0; i < w - 2; i++) top.push(cell(hz, role));
  top.push(cell("╮", role));
  if (opts.title) {
    const t = truncateText(opts.title, Math.max(1, w - 6 - (opts.tag ? textWidth(opts.tag) + 2 : 0)));
    const cells = textCells(` ${t} `, opts.titleRole ?? role);
    for (let i = 0; i < cells.length && 2 + i < w - 1; i++) top[2 + i] = cells[i];
  }
  if (opts.tag) {
    const cells = textCells(opts.tag, opts.tagRole ?? role);
    const start = w - 1 - cells.length;
    for (let i = 0; i < cells.length; i++) if (start + i > 1) top[start + i] = cells[i];
  }
  const rows: Cell[][] = [top];
  for (const r of inner.rows)
    rows.push([
      cell(vt, role),
      ...Array.from({ length: padX }, () => EMPTY),
      ...padRow(r, inner.w),
      ...Array.from({ length: padX }, () => EMPTY),
      cell(vt, role),
    ]);
  rows.push([cell("╰", role), ...Array.from({ length: w - 2 }, () => cell(hz, role)), cell("╯", role)]);
  return { w, rows, targets: shiftTargets(inner.targets, 1 + padX, 1) };
}

export const rectOf = (x: number, y: number, w: number, h: number): Rect => ({ x, y, w, h });

export function textBlock(s: string, width: number, role = ""): Block {
  const lines = wrapText(s, width, role);
  return fromRows(width, lines);
}

export function lineOf(parts: Array<[string, string]>, w: number): Cell[] {
  const out: Cell[] = [];
  for (const [t, r] of parts) out.push(...textCells(t, r));
  return padRow(out, w);
}

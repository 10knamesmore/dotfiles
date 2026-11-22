// The hover bubble: a one-line tooltip drawn over the cells next to the element, inside the widget's own rows so
// the transcript does not jump when it appears.
import { cell, EMPTY, textWidth, truncateText, type Cell } from "../diagram/canvas.ts";
import { BADGE } from "./core.ts";
import type { Target } from "./types.ts";
import { textCells } from "./util.ts";

export function drawBubble(rows: Cell[][], width: number, t: Target): void {
  const hint = t.tip?.hint?.replace(/\s+/g, " ").trim();
  if (!hint || !rows.length) return;
  const text = truncateText(hint, Math.max(8, Math.min(76, width - 5)));
  const cells: Cell[] = [
    cell(" ", "tip.bubble"),
    cell(BADGE, `tip.bubble.${t.tip?.tone ?? "info"}`),
    cell(" ", "tip.bubble"),
    ...textCells(text, "tip.bubble"),
    cell(" ", "tip.bubble"),
  ];
  const w = Math.min(width, 4 + textWidth(text) + 1);
  const r = t.rect;
  // above the element if there is a row there, else below it, else over its own row to the right
  let y = r.y - 1;
  if (y < 0) y = r.y + r.h;
  let x = Math.max(0, Math.min(r.x, width - w));
  if (y >= rows.length) {
    y = Math.min(r.y, rows.length - 1);
    x = Math.max(0, Math.min(r.x + r.w + 1, width - w));
  }
  const row = rows[y];
  if (!row) return;
  // do not cut a wide character in half at either end of the bubble
  if (row[x]?.ch === "") row[x - 1] = EMPTY;
  const end = x + w;
  if (row[end]?.ch === "") row[end] = EMPTY;
  for (let i = 0; i < w && x + i < row.length; i++) row[x + i] = cells[i] ?? cell(" ", "tip.bubble");
}

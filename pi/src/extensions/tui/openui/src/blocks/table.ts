import { textWidth, truncateText, type Cell, type Rect } from "../diagram/canvas.ts";
import type { Block } from "./types.ts";
import { cell, fromRows, padRow, textCells } from "./util.ts";

export interface TableAnchors {
  rows: Rect[];
  cols: Rect[];
  cells: Rect[][];
}

export interface TableSort {
  column: number;
  direction: "asc" | "desc";
}

const NUMERIC = /^[-+]?[\d.,]+\s*(%|ms|s|kb|mb|gb|tb|x)?$/i;

/** A cell that holds a bare number. Units, thousands separators and other text stay text: "50%" and "5s" are never 50 and 5. */
const PLAIN_NUMBER = /^[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?$/;

const collator = new Intl.Collator(undefined, { numeric: true });

function plainNumber(v: string | number | undefined): number | null {
  if (typeof v === "number") return Number.isFinite(v) ? v : null;
  if (typeof v !== "string") return null;
  const t = v.trim();
  return PLAIN_NUMBER.test(t) ? Number(t) : null;
}

function compareCells(a: string | number | undefined, b: string | number | undefined): number {
  const an = plainNumber(a);
  const bn = plainNumber(b);
  if (an !== null && bn !== null) return an - bn;
  return collator.compare(String(a ?? ""), String(b ?? ""));
}

/** Display row → original row index. Equal cells keep their original order, so a row never swaps with an equal neighbour. */
function sortedOrder(rows: Array<Array<string | number>>, sort?: TableSort): number[] {
  const order = rows.map((_, ri) => ri);
  if (!sort) return order;
  const dir = sort.direction === "desc" ? -1 : 1;
  const at = (ri: number): string | number | undefined => rows[ri]?.[sort.column];
  order.sort((a, b) => {
    const c = compareCells(at(a), at(b));
    return c === 0 ? a - b : dir * c;
  });
  return order;
}

export function tableBlock(
  columns: string[],
  rows: Array<Array<string | number>>,
  width: number,
  sort?: TableSort,
): { block: Block; anchors: TableAnchors; order: number[] } {
  const n = Math.max(1, columns.length);
  const str = (v: string | number) =>
    String(v ?? "")
      .replace(/\s+/g, " ")
      .trim();
  const data = rows.map((r) => columns.map((_, i) => str(r[i] ?? "")));
  const order = sortedOrder(rows, sort);
  const shown = order.map((ri) => data[ri]);
  const arrow = sort ? (sort.direction === "asc" ? " ↑" : " ↓") : "";
  const heads = columns.map((c, i) => str(c) + (i === sort?.column ? arrow : ""));
  const numeric = heads.map((_, i) => shown.length > 0 && shown.every((r) => r[i] === "" || NUMERIC.test(r[i])));
  const nat = heads.map((h, i) => Math.max(textWidth(h), ...shown.map((r) => textWidth(r[i])), 1));
  const avail = Math.max(n, width - (3 * n + 1));
  const w = nat.slice();
  let sum = w.reduce((a, b) => a + b, 0);
  while (sum > avail) {
    let k = 0;
    for (let i = 1; i < n; i++) if (w[i] > w[k]) k = i;
    if (w[k] <= 3) break;
    w[k]--;
    sum--;
  }
  const total = w.reduce((a, b) => a + b, 0) + 3 * n + 1;
  const line = (l: string, m: string, r: string): Cell[] => {
    const out: Cell[] = [cell(l, "table.border")];
    w.forEach((cw, i) => {
      for (let k = 0; k < cw + 2; k++) out.push(cell("─", "table.border"));
      out.push(cell(i === n - 1 ? r : m, "table.border"));
    });
    return out;
  };
  const rowCells = (vals: string[], role: string, align: boolean[]): Cell[] => {
    const out: Cell[] = [cell("│", "table.border")];
    vals.forEach((v, i) => {
      const t = truncateText(v, w[i]);
      const tw = textWidth(t);
      const pad = w[i] - tw;
      out.push(cell(" "));
      if (align[i]) for (let k = 0; k < pad; k++) out.push(cell(" "));
      out.push(...textCells(t, role));
      if (!align[i]) for (let k = 0; k < pad; k++) out.push(cell(" "));
      out.push(cell(" "), cell("│", "table.border"));
    });
    return out;
  };
  const out: Cell[][] = [line("┌", "┬", "┐")];
  out.push(
    rowCells(
      heads,
      "table.head",
      heads.map(() => false),
    ),
  );
  out.push(line("├", "┼", "┤"));
  shown.forEach((r) => out.push(rowCells(r, "", numeric)));
  out.push(line("└", "┴", "┘"));
  const colX: number[] = [];
  let x = 1;
  w.forEach((cw) => {
    colX.push(x + 1);
    x += cw + 3;
  });
  // anchors stay in original row order; their y is where that row is drawn, so a note on row:N still means row N of the data
  const rowY: number[] = [];
  order.forEach((ri, pos) => {
    rowY[ri] = 3 + pos;
  });
  const anchors: TableAnchors = {
    cols: w.map((cw, i) => ({ x: colX[i], y: 1, w: cw, h: 1 })),
    rows: rows.map((_, ri) => ({ x: 0, y: rowY[ri], w: total, h: 1 })),
    cells: rows.map((_, ri) => w.map((cw, i) => ({ x: colX[i], y: rowY[ri], w: cw, h: 1 }))),
  };
  return {
    block: fromRows(
      total,
      out.map((r) => padRow(r, total)),
    ),
    anchors,
    order,
  };
}

function csvCell(v: string | number | undefined): string {
  const s = String(v ?? "");
  return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
}

/** The table as RFC 4180 style CSV: header, CRLF between records, cells kept exactly as given. `order` exports in display order. */
export function tableCsv(columns: string[], rows: Array<Array<string | number>>, order?: number[]): string {
  const lines = [columns.map(csvCell).join(",")];
  for (const ri of order ?? rows.map((_, i) => i)) {
    lines.push(columns.map((_, i) => csvCell(rows[ri]?.[i])).join(","));
  }
  return lines.join("\r\n");
}

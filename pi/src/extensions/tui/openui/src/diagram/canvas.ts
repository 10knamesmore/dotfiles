// Text measurement and a cell canvas. Pure, no dependencies.
import { WIDE } from "./wide.ts";

const seg: { segment(s: string): Iterable<{ segment: string }> } | null =
  typeof Intl !== "undefined" && (Intl as any).Segmenter
    ? new (Intl as any).Segmenter(undefined, { granularity: "grapheme" })
    : null;

export function graphemes(s: string): string[] {
  return seg ? Array.from(seg.segment(s), (x) => x.segment) : Array.from(s);
}

// The code points pi draws two cells wide: the generated table, which src/pi/widths.ts corrects against the running pi when the
// extension loads (the blocks where Unicode adds emoji). Flat [first, last] pairs, ascending.
let table: readonly number[] = WIDE;
export function setWide(t: readonly number[]): void {
  table = t;
}

/** Whether `cp` is in the table of code points pi draws two cells wide (binary search over [first, last] pairs). */
function isWide(cp: number): boolean {
  let lo = 0;
  let hi = table.length / 2 - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (cp < table[mid * 2]) hi = mid - 1;
    else if (cp > table[mid * 2 + 1]) lo = mid + 1;
    else return true;
  }
  return false;
}

/**
 * Columns one code point takes. It must agree with the terminal, or everything to its right on the row slides and the borders
 * come out of line: wide characters (CJK, and the emoji and symbols models like: U+2705 U+274C U+26A1 U+1F680) are two, taken from pi's own width
 * function through src/diagram/wide.ts; marks and controls are none.
 */
function cpWidth(cp: number): number {
  if (cp === 0 || cp < 32 || (cp >= 0x7f && cp < 0xa0)) return 0;
  if (
    (cp >= 0x300 && cp <= 0x36f) ||
    (cp >= 0x200b && cp <= 0x200f) ||
    (cp >= 0xfe00 && cp <= 0xfe0f) ||
    (cp >= 0x20d0 && cp <= 0x20ff)
  )
    return 0;
  return isWide(cp) ? 2 : 1;
}

/** Display width of one grapheme cluster (0, 1 or 2 columns). */
export function gWidth(g: string): number {
  let w = 0;
  let sawVs16 = false;
  for (const ch of g) {
    const cp = ch.codePointAt(0)!;
    if (cp === 0xfe0f) sawVs16 = true;
    w = Math.max(w, cpWidth(cp));
  }
  return sawVs16 ? 2 : w;
}

export function textWidth(s: string): number {
  let w = 0;
  for (const g of graphemes(s)) w += gWidth(g);
  return w;
}

/** Cut `s` to at most `max` columns, ending in an ellipsis when something was cut. */
export function truncateText(s: string, max: number, ell = "…"): string {
  if (max <= 0) return "";
  if (textWidth(s) <= max) return s;
  const ew = textWidth(ell);
  if (max <= ew) return graphemes(ell).slice(0, max).join("");
  let out = "";
  let w = 0;
  for (const g of graphemes(s)) {
    const gw = gWidth(g);
    if (w + gw > max - ew) break;
    out += g;
    w += gw;
  }
  return out + ell;
}

/** Replace control characters and newlines so a label is always one line. */
export function oneLine(s: string): string {
  return String(s ?? "")
    .replace(/[\u0000-\u001f\u007f]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export interface Cell {
  readonly ch: string; // "" marks the second column of a wide character
  readonly role: string;
}

/**
 * Cells are interned and frozen. A widget is mostly blanks and box lines, and one object per terminal cell cost 160 KB per
 * widget (300 widgets in a transcript held 47 MB); shared cells cost a pointer each. Nothing may change a cell: build a new
 * one with `cell()` instead. Freezing makes a mistake throw at once instead of restyling every widget that shares the cell.
 */
const pool = new Map<string, Map<string, Cell>>();
let pooled = 0;
const POOL_MAX = 50_000;
export function cell(ch: string, role = ""): Cell {
  let byCh = pool.get(role);
  if (!byCh) pool.set(role, (byCh = new Map()));
  let c = byCh.get(ch);
  if (!c) {
    c = Object.freeze({ ch, role });
    if (pooled < POOL_MAX) {
      byCh.set(ch, c);
      pooled++;
    }
  }
  return c;
}
export const EMPTY: Cell = cell(" ");

export class Canvas {
  readonly w: number;
  readonly h: number;
  readonly ch: string[];
  readonly role: string[];
  constructor(w: number, h: number) {
    this.w = w;
    this.h = h;
    this.ch = new Array(w * h).fill(" ");
    this.role = new Array(w * h).fill("");
  }
  inb(x: number, y: number): boolean {
    return x >= 0 && y >= 0 && x < this.w && y < this.h;
  }
  set(x: number, y: number, ch: string, role = ""): void {
    if (!this.inb(x, y)) return;
    this.ch[y * this.w + x] = ch;
    this.role[y * this.w + x] = role;
  }
  getCh(x: number, y: number): string {
    return this.inb(x, y) ? this.ch[y * this.w + x] : " ";
  }
  getRole(x: number, y: number): string {
    return this.inb(x, y) ? this.role[y * this.w + x] : "";
  }
  isBlank(x: number, y: number): boolean {
    return this.inb(x, y) && this.ch[y * this.w + x] === " " && this.role[y * this.w + x] === "";
  }
  /** Write text starting at (x, y). Returns the number of columns written. */
  text(x: number, y: number, s: string, role = ""): number {
    let cx = x;
    for (const g of graphemes(s)) {
      const gw = gWidth(g);
      if (gw === 0) continue;
      this.set(cx, y, g, role);
      if (gw === 2) this.set(cx + 1, y, "", role);
      cx += gw;
    }
    return cx - x;
  }
  rows(): Cell[][] {
    const out: Cell[][] = [];
    for (let y = 0; y < this.h; y++) {
      const row: Cell[] = [];
      for (let x = 0; x < this.w; x++) row.push(cell(this.ch[y * this.w + x], this.role[y * this.w + x]));
      out.push(row);
    }
    return out;
  }
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Palette = (role: string) => ((s: string) => string) | undefined;

/** Turn cell rows into strings, grouping runs of the same role so escapes stay short. */
export function rowsToLines(rows: Cell[][], palette?: Palette, trim = true): string[] {
  return rows.map((full) => {
    let end = full.length;
    while (trim && end > 0 && full[end - 1].ch === " " && full[end - 1].role === "") end--;
    const row = full.slice(0, end);
    let out = "";
    let runRole: string | null = null;
    let run = "";
    const flush = () => {
      if (runRole === null) return;
      const f = palette ? palette(runRole) : undefined;
      out += f ? f(run) : run;
      run = "";
    };
    for (const c of row) {
      if (c.role !== runRole) {
        flush();
        runRole = c.role;
      }
      run += c.ch;
    }
    flush();
    return out;
  });
}

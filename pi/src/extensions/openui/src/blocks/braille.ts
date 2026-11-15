// A dot canvas drawn with braille characters: 2 x 4 dots per cell, square dots in a 1:2 cell.
import { cell, EMPTY, type Cell } from "../diagram/canvas.ts";

const BITS = [
  [0x01, 0x02, 0x04, 0x40],
  [0x08, 0x10, 0x20, 0x80],
];

export class Dots {
  readonly cols: number;
  readonly rows: number;
  readonly w: number;
  readonly h: number;
  readonly bits: Uint8Array;
  readonly owner: Int16Array;
  constructor(cols: number, rows: number) {
    this.cols = cols;
    this.rows = rows;
    this.w = cols * 2;
    this.h = rows * 4;
    this.bits = new Uint8Array(cols * rows);
    this.owner = new Int16Array(cols * rows).fill(-1);
  }
  set(x: number, y: number, owner: number): void {
    x = Math.round(x);
    y = Math.round(y);
    if (x < 0 || y < 0 || x >= this.w || y >= this.h) return;
    const c = (y >> 2) * this.cols + (x >> 1);
    this.bits[c] |= BITS[x & 1][y & 3];
    this.owner[c] = owner;
  }
  line(x0: number, y0: number, x1: number, y1: number, owner: number): void {
    x0 = Math.round(x0);
    y0 = Math.round(y0);
    x1 = Math.round(x1);
    y1 = Math.round(y1);
    const dx = Math.abs(x1 - x0);
    const dy = -Math.abs(y1 - y0);
    const sx = x0 < x1 ? 1 : -1;
    const sy = y0 < y1 ? 1 : -1;
    let err = dx + dy;
    for (let guard = 0; guard < 100000; guard++) {
      this.set(x0, y0, owner);
      if (x0 === x1 && y0 === y1) break;
      const e2 = 2 * err;
      if (e2 >= dy) {
        err += dy;
        x0 += sx;
      }
      if (e2 <= dx) {
        err += dx;
        y0 += sy;
      }
    }
  }
  rows2cells(roleFor: (owner: number) => string): Cell[][] {
    const out: Cell[][] = [];
    for (let r = 0; r < this.rows; r++) {
      const row: Cell[] = [];
      for (let c = 0; c < this.cols; c++) {
        const b = this.bits[r * this.cols + c];
        row.push(b ? cell(String.fromCharCode(0x2800 + b), roleFor(this.owner[r * this.cols + c])) : EMPTY);
      }
      out.push(row);
    }
    return out;
  }
}

export function niceNum(x: number, round: boolean): number {
  if (x <= 0) return 1;
  const exp = Math.floor(Math.log10(x));
  const f = x / 10 ** exp;
  const nf = round ? (f < 1.5 ? 1 : f < 3 ? 2 : f < 7 ? 5 : 10) : f <= 1 ? 1 : f <= 2 ? 2 : f <= 5 ? 5 : 10;
  return nf * 10 ** exp;
}

export function fmtNum(v: number): string {
  if (!Number.isFinite(v)) return "?";
  const a = Math.abs(v);
  if (a >= 1e6) return `${+(v / 1e6).toFixed(1)}M`;
  if (a >= 1e4) return `${+(v / 1e3).toFixed(1)}k`;
  if (Number.isInteger(v)) return String(v);
  return String(+v.toFixed(a < 1 ? 3 : a < 100 ? 2 : 1));
}

/** monotone cubic interpolation (no overshoot) evaluated at x */
export function monotone(xs: number[], ys: number[]): (x: number) => number {
  const n = xs.length;
  if (n === 1) return () => ys[0];
  const d: number[] = [];
  const m: number[] = new Array(n).fill(0);
  for (let i = 0; i < n - 1; i++) d.push((ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i] || 1));
  m[0] = d[0];
  m[n - 1] = d[n - 2];
  for (let i = 1; i < n - 1; i++) m[i] = d[i - 1] * d[i] <= 0 ? 0 : (d[i - 1] + d[i]) / 2;
  for (let i = 0; i < n - 1; i++) {
    if (d[i] === 0) {
      m[i] = 0;
      m[i + 1] = 0;
      continue;
    }
    const a = m[i] / d[i];
    const b = m[i + 1] / d[i];
    const s = a * a + b * b;
    if (s > 9) {
      const t = 3 / Math.sqrt(s);
      m[i] = t * a * d[i];
      m[i + 1] = t * b * d[i];
    }
  }
  return (x: number) => {
    let i = 0;
    while (i < n - 2 && x > xs[i + 1]) i++;
    const h = xs[i + 1] - xs[i] || 1;
    const t = Math.min(1, Math.max(0, (x - xs[i]) / h));
    const t2 = t * t;
    const t3 = t2 * t;
    return (
      (2 * t3 - 3 * t2 + 1) * ys[i] +
      (t3 - 2 * t2 + t) * h * m[i] +
      (-2 * t3 + 3 * t2) * ys[i + 1] +
      (t3 - t2) * h * m[i + 1]
    );
  };
}

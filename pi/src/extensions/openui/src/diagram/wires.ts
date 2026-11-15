// Draw routed wires, end markers and labels onto a Canvas.

import { Canvas, textWidth } from "./canvas.ts";
import { DX, DY, OPP, type Step } from "./router.ts";

export type Mark = "none" | "arrow" | "dot" | "one" | "many";

export interface WireSpec {
  id: number;
  steps: Step[];
  dashed: boolean;
  /** mark at the start box / end box */
  startMark: Mark;
  endMark: Mark;
  label?: string;
  /** short description for a footnote, e.g. "a -> b" */
  desc?: string;
  /** role prefix, e.g. "wire" */
  role: string;
}

const BIT = [1, 2, 4, 8]; // N E S W
const GLYPH: Record<number, string> = {
  0: " ",
  1: "│",
  2: "─",
  3: "╰",
  4: "│",
  5: "│",
  6: "╭",
  7: "├",
  8: "─",
  9: "╯",
  10: "─",
  11: "┴",
  12: "╮",
  13: "┤",
  14: "┬",
  15: "┼",
};
const ARROW = ["▲", "▶", "▼", "◀"];
const JUNCTION = ["┴", "├", "┬", "┤"]; // by outward normal of the box side
/** end glyphs by the direction that points INTO the box: 0 north, 1 east, 2 south, 3 west */
export const ONE_END = ["┬", "┤", "┴", "├"];
export const MANY_END = ["V", "<", "Λ", ">"];

export interface WireDrawResult {
  footnotes: string[];
  /** cells that carry the label text of each wire id, for tests */
  labelCells: Map<number, Array<[number, number]>>;
}

function straightCells(w: WireSpec): Array<{ i: number; horizontal: boolean }> {
  const out: Array<{ i: number; horizontal: boolean }> = [];
  const s = w.steps;
  for (let i = 1; i + 1 < s.length; i++)
    if (s[i].dir === s[i + 1].dir) out.push({ i, horizontal: s[i].dir === 1 || s[i].dir === 3 });
  return out;
}

export function drawWires(c: Canvas, wires: WireSpec[], reserved?: (x: number, y: number) => boolean): WireDrawResult {
  const W = c.w;
  const mask = new Uint8Array(c.w * c.h);
  const solidN = new Uint8Array(c.w * c.h);
  const dashN = new Uint8Array(c.w * c.h);
  const roleAt = new Array<string>(c.w * c.h).fill("");
  const wireCell = new Uint8Array(c.w * c.h);
  for (const w of wires) {
    const n = w.steps.length;
    for (let i = 0; i < n; i++) {
      const s = w.steps[i];
      if (!c.inb(s.x, s.y)) continue;
      const k = s.y * W + s.x;
      mask[k] |= BIT[OPP[s.dir]];
      if (i + 1 < n) mask[k] |= BIT[w.steps[i + 1].dir];
      if (w.dashed) dashN[k]++;
      else solidN[k]++;
      roleAt[k] = w.role;
      wireCell[k] = 1;
    }
  }
  for (let y = 0; y < c.h; y++) {
    for (let x = 0; x < c.w; x++) {
      const k = y * W + x;
      if (!wireCell[k]) continue;
      let m = mask[k];
      if (c.getRole(x, y).startsWith("zone")) {
        const zc = c.getCh(x, y);
        m |= zc === "│" ? 5 : zc === "─" ? 10 : 0;
      }
      let g = GLYPH[m];
      if (dashN[k] > 0 && solidN[k] === 0) {
        if (m === 5 || m === 1 || m === 4) g = "┆";
        else if (m === 10 || m === 2 || m === 8) g = "┄";
      }
      c.set(x, y, g, roleAt[k]);
    }
  }

  // markers and attached ends
  for (const w of wires) {
    const s = w.steps;
    if (!s.length) continue;
    const first = s[0];
    const last = s[s.length - 1];
    const startDirIn = OPP[first.dir]; // pointing into the start box
    const endDirIn = last.dir; // pointing into the end box
    applyEnd(c, first, startDirIn, w.startMark, w.role, true);
    applyEnd(c, last, endDirIn, w.endMark, w.role, false);
  }

  // labels
  const footnotes: string[] = [];
  const labelCells = new Map<number, Array<[number, number]>>();
  const taken = new Set<number>();
  for (const w of wires) {
    if (!w.label) continue;
    const text = w.label;
    const tw = textWidth(text);
    const placed = placeLabel(c, w, text, tw, reserved, taken);
    if (placed) labelCells.set(w.id, placed);
    else {
      const idx = footnotes.length + 1;
      const mark = String(idx);
      const cand = straightCells(w);
      const mid = cand.length ? cand[Math.floor(cand.length / 2)] : null;
      if (mid) {
        const st = w.steps[mid.i];
        c.set(st.x, st.y, mark.length === 1 ? mark : mark[0], "elabel");
        labelCells.set(w.id, [[st.x, st.y]]);
        footnotes.push(`${idx} ${text}`);
      } else footnotes.push(`${w.desc ? w.desc + ": " : ""}${text}`);
    }
  }
  return { footnotes, labelCells };
}

function applyEnd(c: Canvas, st: Step, dirIn: number, mark: Mark, role: string, isStart: boolean): void {
  const outward = OPP[dirIn];
  if (mark === "none") {
    // attach: turn the box border cell into a junction
    const bx = st.x + DX[dirIn];
    const by = st.y + DY[dirIn];
    c.set(bx, by, JUNCTION[outward], c.getRole(bx, by) || role);
    return;
  }
  if (mark === "arrow") {
    c.set(st.x, st.y, ARROW[dirIn], "arrow");
    return;
  }
  if (mark === "dot") {
    c.set(st.x, st.y, "●", "mark");
    return;
  }
  // Relation ends, one cell each so they stay small and touch the box:
  //   one  = a bar across the wire, fused with it (├ ┤ ┬ ┴), which reads as the "||" next to the box border
  //   many = a crow's foot whose toes face the box (< > for a side entry, V and Λ for a top or bottom entry)
  c.set(st.x, st.y, mark === "one" ? ONE_END[dirIn] : MANY_END[dirIn], "mark");
  void isStart;
  void role;
}

function placeLabel(
  c: Canvas,
  w: WireSpec,
  text: string,
  tw: number,
  reserved: ((x: number, y: number) => boolean) | undefined,
  taken: Set<number>,
): Array<[number, number]> | null {
  const s = w.steps;
  // maximal runs of straight cells (same direction, consecutive steps)
  type Run = { cells: number[]; horizontal: boolean };
  const contiguous: Run[] = [];
  let cur: Run | null = null;
  for (let i = 1; i + 1 < s.length; i++) {
    if (s[i].dir !== s[i + 1].dir) {
      cur = null;
      continue;
    }
    if (cur && cur.cells[cur.cells.length - 1] === i - 1) cur.cells.push(i);
    else {
      cur = { cells: [i], horizontal: s[i].dir === 1 || s[i].dir === 3 };
      contiguous.push(cur);
    }
  }
  const free = (x: number, y: number) => c.isBlank(x, y) && !taken.has(y * c.w + x) && !(reserved && reserved(x, y));
  // 1. inline on the longest horizontal run
  const hor = contiguous.filter((r) => r.horizontal).sort((a, b) => b.cells.length - a.cells.length);
  for (const r of hor) {
    if (r.cells.length < tw + 2) continue;
    const xs = r.cells.map((i) => s[i].x).sort((p, q) => p - q);
    const x0 = xs[Math.floor((xs.length - tw) / 2)];
    const y = s[r.cells[0]].y;
    const cells: Array<[number, number]> = [];
    let ok = true;
    for (let k = 0; k < tw; k++) {
      const x = x0 + k;
      const cell = c.getCh(x, y);
      if (!"─┄".includes(cell)) {
        ok = false;
        break;
      }
      cells.push([x, y]);
    }
    if (!ok) continue;
    c.text(x0, y, text, "elabel");
    for (const [x, y2] of cells) taken.add(y2 * c.w + x);
    return cells;
  }
  // 2. beside a horizontal run (above, then below)
  for (const r of hor) {
    if (r.cells.length < Math.min(tw, 4)) continue;
    const mid = s[r.cells[Math.floor(r.cells.length / 2)]];
    const x0 = mid.x - Math.floor(tw / 2);
    for (const dy of [-1, 1]) {
      const y = mid.y + dy;
      let ok = true;
      for (let k = 0; k < tw; k++) if (!free(x0 + k, y)) ok = false;
      if (!ok) continue;
      c.text(x0, y, text, "label.edge");
      const cells: Array<[number, number]> = [];
      for (let k = 0; k < tw; k++) {
        taken.add(y * c.w + x0 + k);
        cells.push([x0 + k, y]);
      }
      return cells;
    }
  }
  // 3. beside a vertical run (right, then left)
  const ver = contiguous.filter((r) => !r.horizontal).sort((a, b) => b.cells.length - a.cells.length);
  for (const r of ver) {
    for (const ci of [Math.floor(r.cells.length / 2), 0, r.cells.length - 1]) {
      const st = s[r.cells[ci]];
      for (const side of [1, -1]) {
        const x0 = side === 1 ? st.x + 2 : st.x - 1 - tw;
        let ok = true;
        for (let k = 0; k < tw; k++) if (!free(x0 + k, st.y)) ok = false;
        if (!ok) continue;
        c.text(x0, st.y, text, "label.edge");
        const cells: Array<[number, number]> = [];
        for (let k = 0; k < tw; k++) {
          taken.add(st.y * c.w + x0 + k);
          cells.push([x0 + k, st.y]);
        }
        return cells;
      }
    }
  }
  // 4. beside any cell of the wire, ends included (very short wires)
  const pick = [Math.floor(s.length / 2), ...s.map((_, i) => i)];
  for (const i of pick) {
    const st = s[i];
    if (st === undefined) continue;
    const vertical = st.dir === 0 || st.dir === 2;
    const spots: Array<[number, number]> = vertical
      ? [
          [st.x + 2, st.y],
          [st.x - 1 - tw, st.y],
        ]
      : [
          [st.x - Math.floor(tw / 2), st.y - 1],
          [st.x - Math.floor(tw / 2), st.y + 1],
        ];
    for (const [x0, y] of spots) {
      let ok = true;
      for (let k = 0; k < tw; k++) if (!free(x0 + k, y)) ok = false;
      if (!ok) continue;
      c.text(x0, y, text, "label.edge");
      const cells: Array<[number, number]> = [];
      for (let k = 0; k < tw; k++) {
        taken.add(y * c.w + x0 + k);
        cells.push([x0 + k, y]);
      }
      return cells;
    }
  }
  return null;
}

// SequenceDiagram: columns left to right, steps top to bottom.
import { textWidth, truncateText, type Cell } from "../diagram/canvas.ts";
import type { El } from "../lang/library.ts";
import type { Anchor } from "./core.ts";
import type { Block, Target } from "./types.ts";
import { cell, fromRows, textCells, wrapText } from "./util.ts";

const ICON: Record<string, string> = { actor: "⚇", service: "◇", database: "◎", queue: "≡", external: "◌" };

type Ev =
  | { t: "msg"; from: number; to: number; label: string; style: string; id?: string; n: number; depth: number }
  | { t: "note"; over: number; to?: number; text: string; depth: number }
  | { t: "open"; kind: string; label: string; depth: number }
  | { t: "else"; label: string; depth: number }
  | { t: "close"; depth: number };

export function sequence(
  el: El,
  width: number,
  path: string,
): { block: Block; anchors: Map<string, Anchor>; overflow: boolean } {
  const parts: El[] = el.props.participants ?? [];
  const idx = new Map(parts.map((p, i) => [String(p.props.id), i]));
  const numbered = el.props.numbered !== false;
  const anchors = new Map<string, Anchor>();
  const evs: Ev[] = [];
  let n = 0;
  let maxDepth = 0;
  const walk = (steps: El[], depth: number) => {
    for (const s of steps ?? []) {
      if (s.type === "SeqMessage")
        evs.push({
          t: "msg",
          from: idx.get(s.props.from) ?? 0,
          to: idx.get(s.props.to) ?? 0,
          label: String(s.props.label ?? ""),
          style: s.props.style ?? "sync",
          id: s.props.id,
          n: ++n,
          depth,
        });
      else if (s.type === "SeqNote")
        evs.push({
          t: "note",
          over: idx.get(s.props.over) ?? 0,
          to: s.props.to !== undefined ? idx.get(s.props.to) : undefined,
          text: String(s.props.text ?? ""),
          depth,
        });
      else if (s.type === "SeqGroup") {
        if (depth >= 3) {
          walk(s.props.steps, depth);
          continue;
        }
        maxDepth = Math.max(maxDepth, depth + 1);
        evs.push({ t: "open", kind: s.props.kind, label: String(s.props.label ?? ""), depth });
        walk(s.props.steps, depth + 1);
        if (s.props.elseSteps && s.props.elseSteps.length) {
          evs.push({ t: "else", label: String(s.props.elseLabel ?? ""), depth });
          walk(s.props.elseSteps, depth + 1);
        }
        evs.push({ t: "close", depth });
      }
    }
  };
  walk(el.props.steps ?? [], 0);

  const layout = (cap: number, headCap: number) => {
    const heads = parts.map((p) => {
      const label = truncateText(String(p.props.label ?? ""), headCap);
      return { label, icon: ICON[p.props.kind ?? "service"] ?? "◇", w: textWidth(label) + 6 };
    });
    const gap = heads.slice(1).map((h, i) => Math.ceil((heads[i].w + h.w) / 2) + 2);
    const lw = (e: Extract<Ev, { t: "msg" }>) =>
      textWidth(truncateText(e.label, cap)) + (numbered ? String(e.n).length + 2 : 0);
    const spans = evs
      .filter((e): e is Extract<Ev, { t: "msg" }> => e.t === "msg" && e.from !== e.to)
      .sort((a, b) => Math.abs(a.from - a.to) - Math.abs(b.from - b.to));
    for (const m of spans) {
      const lo = Math.min(m.from, m.to);
      const hi = Math.max(m.from, m.to);
      const need = lw(m) + 6;
      let sum = 0;
      for (let j = lo; j < hi; j++) sum += gap[j];
      if (sum < need) gap[hi - 1] += need - sum;
    }
    let rightExtra = 0;
    let leftExtra = 0;
    for (const e of evs) {
      if (e.t === "msg" && e.from === e.to) {
        const need = lw(e) + 8;
        if (e.from < parts.length - 1) gap[e.from] = Math.max(gap[e.from], need);
        else rightExtra = Math.max(rightExtra, need);
      } else if (e.t === "note") {
        const nw = Math.min(30, textWidth(e.text) + 4);
        const half = Math.ceil(nw / 2) + 2;
        if (e.to === undefined || e.to === e.over) {
          if (e.over > 0) gap[e.over - 1] = Math.max(gap[e.over - 1], half + Math.ceil(heads[e.over - 1].w / 2));
          else leftExtra = Math.max(leftExtra, half);
          if (e.over < parts.length - 1) gap[e.over] = Math.max(gap[e.over], half + Math.ceil(heads[e.over + 1].w / 2));
          else rightExtra = Math.max(rightExtra, half);
        }
      }
    }
    const margin = 2 * maxDepth + 1;
    const x: number[] = [];
    x.push(margin + Math.max(Math.ceil(heads[0].w / 2), leftExtra));
    for (let i = 1; i < parts.length; i++) x.push(x[i - 1] + gap[i - 1]);
    const total = x[x.length - 1] + Math.max(Math.ceil(heads[heads.length - 1].w / 2), rightExtra) + margin + 1;
    return { heads, x, total, margin };
  };
  let lay = layout(48, 22);
  for (const [cap, hc] of [
    [36, 22],
    [28, 18],
    [20, 14],
    [14, 10],
    [10, 8],
  ] as const) {
    if (lay.total <= width) break;
    lay = layout(cap, hc);
  }
  const cap = lay.total <= width ? 48 : 10;
  void cap;
  const { heads, x, total } = lay;
  const msgCap = lay.total <= width ? 48 : 10;
  // rows
  const H0 = 3;
  let y = H0;
  const rowOf: number[] = [];
  const noteLines: string[][] = [];
  const noteBox: Array<{ x0: number; x1: number } | null> = [];
  evs.forEach((e, i) => {
    rowOf[i] = y;
    if (e.t === "msg") y += e.from === e.to ? 2 : 2;
    else if (e.t === "note") {
      const dx = e.to !== undefined ? Math.abs(x[e.to] - x[e.over]) : 0;
      const nw = e.to !== undefined && e.to !== e.over ? dx + 5 : Math.min(30, textWidth(e.text) + 4);
      const x0 =
        e.to !== undefined && e.to !== e.over ? Math.min(x[e.over], x[e.to]) - 2 : x[e.over] - Math.floor(nw / 2);
      noteBox[i] = { x0, x1: x0 + nw - 1 };
      const lines = wrapText(e.text, Math.max(3, nw - 4)).map((r) => r.map((c) => c.ch).join(""));
      noteLines[i] = lines;
      y += lines.length + 2;
    } else y += 1;
  });
  const lastRow = y;
  const height = lastRow + 1;
  const rows: Cell[][] = Array.from({ length: height }, () => Array.from({ length: total }, () => cell(" ")));
  const put = (xx: number, yy: number, ch: string, role: string) => {
    if (yy >= 0 && yy < height && xx >= 0 && xx < total) rows[yy][xx] = cell(ch, role);
  };
  const text = (xx: number, yy: number, s: string, role: string) =>
    textCells(s, role).forEach((c, k) => put(xx + k, yy, c.ch, c.role));
  // lifelines
  x.forEach((xx) => {
    for (let r = H0; r <= lastRow; r++) put(xx, r, "┆", "seq.life");
  });
  // activation bars
  const stacks: number[][] = parts.map(() => []);
  evs.forEach((e, i) => {
    if (e.t !== "msg" || e.from === e.to) return;
    const arrow = rowOf[i] + 1;
    if (e.style === "sync") stacks[e.to].push(arrow);
    else if (e.style === "return" && stacks[e.from].length) {
      const s = stacks[e.from].pop()!;
      for (let r = s; r <= arrow; r++) put(x[e.from], r, "┃", "seq.act");
    }
  });
  stacks.forEach((st, p) =>
    st.forEach((s) => {
      for (let r = s; r <= lastRow - 1; r++) put(x[p], r, "┃", "seq.act");
    }),
  );
  // headers
  const targets: Target[] = [];
  parts.forEach((p, i) => {
    const h = heads[i];
    const x0 = x[i] - Math.floor(h.w / 2);
    put(x0, 0, "╭", "seq.head");
    put(x0 + h.w - 1, 0, "╮", "seq.head");
    put(x0, 2, "╰", "seq.head");
    put(x0 + h.w - 1, 2, "╯", "seq.head");
    for (let k = 1; k < h.w - 1; k++) {
      put(x0 + k, 0, "─", "seq.head");
      put(x0 + k, 2, "─", "seq.head");
    }
    put(x0, 1, "│", "seq.head");
    put(x0 + h.w - 1, 1, "│", "seq.head");
    put(x[i], 2, "┬", "seq.head");
    put(x0 + 2, 1, h.icon, "seq.icon");
    text(x0 + 4, 1, h.label, "seq.title");
    const rect = { x: x0, y: 0, w: h.w, h: 3 };
    const id = `seq:${path}:${p.props.id}`;
    targets.push({ id, kind: "node", rect, label: `${p.props.label} (${p.props.kind ?? "service"})` });
    anchors.set(String(p.props.id), { rect, targetId: id });
  });
  // events
  const frames: Array<{ open: number; close: number; depth: number; elseRow?: number }> = [];
  const openStack: number[] = [];
  evs.forEach((e, i) => {
    const r = rowOf[i];
    if (e.t === "msg") {
      const label = (numbered ? `${e.n}. ` : "") + truncateText(e.label, msgCap);
      if (e.from === e.to) {
        const xx = x[e.from];
        put(xx, r, "├", "seq.arrow");
        put(xx + 1, r, "─", "seq.arrow");
        put(xx + 2, r, "─", "seq.arrow");
        put(xx + 3, r, "╮", "seq.arrow");
        put(xx + 3, r + 1, "╯", "seq.arrow");
        put(xx + 2, r + 1, "─", "seq.arrow");
        put(xx + 1, r + 1, e.style === "async" ? "◁" : "◀", "seq.arrow");
        text(xx + 5, r, label, "seq.label");
        if (e.id) anchors.set(e.id, { rect: { x: xx, y: r, w: Math.min(total - xx, textWidth(label) + 6), h: 2 } });
        return;
      }
      const a = x[e.from];
      const b = x[e.to];
      const lo = Math.min(a, b);
      const hi = Math.max(a, b);
      const dashed = e.style === "return";
      const ay = r + 1;
      for (let xx = lo + 1; xx < hi; xx++) put(xx, ay, dashed ? "┄" : "─", "seq.arrow");
      put(a, ay, a < b ? "├" : "┤", "seq.arrow");
      const open = e.style !== "sync";
      put(a < b ? hi - 1 : lo + 1, ay, a < b ? (open ? "▷" : "▶") : open ? "◁" : "◀", "seq.arrow");
      const span = hi - lo - 1;
      const lt = truncateText(label, Math.max(1, span - 2));
      const lx = lo + 1 + Math.max(0, Math.floor((span - textWidth(lt)) / 2));
      text(lx, r, lt, "seq.label");
      if (numbered) {
        const nlen = String(e.n).length + 1;
        for (let k = 0; k < nlen; k++) if (rows[r][lx + k]) rows[r][lx + k] = cell(rows[r][lx + k].ch, "seq.num");
      }
      if (e.id) anchors.set(e.id, { rect: { x: lo, y: r, w: hi - lo + 1, h: 2 } });
    } else if (e.t === "note") {
      const b = noteBox[i]!;
      const lines = noteLines[i];
      const nw = b.x1 - b.x0 + 1;
      for (let k = 0; k < lines.length + 2; k++) for (let xx = 0; xx < nw; xx++) put(b.x0 + xx, r + k, " ", "seq.note");
      put(b.x0, r, "┌", "seq.noteBorder");
      put(b.x1, r, "┐", "seq.noteBorder");
      put(b.x0, r + lines.length + 1, "└", "seq.noteBorder");
      put(b.x1, r + lines.length + 1, "┘", "seq.noteBorder");
      for (let xx = 1; xx < nw - 1; xx++) {
        put(b.x0 + xx, r, "─", "seq.noteBorder");
        put(b.x0 + xx, r + lines.length + 1, "─", "seq.noteBorder");
      }
      lines.forEach((ln, k) => {
        put(b.x0, r + 1 + k, "│", "seq.noteBorder");
        put(b.x1, r + 1 + k, "│", "seq.noteBorder");
        text(b.x0 + 2, r + 1 + k, ln, "seq.note");
      });
    } else if (e.t === "open") {
      openStack.push(frames.length);
      frames.push({ open: r, close: r, depth: e.depth });
      const f = frames[frames.length - 1];
      const left = 2 * e.depth;
      const right = total - 1 - 2 * e.depth;
      put(left, r, "┌", "seq.group");
      put(right, r, "┐", "seq.group");
      for (let xx = left + 1; xx < right; xx++) put(xx, r, "─", "seq.group");
      text(left + 1, r, ` ${e.kind} `, "seq.groupKind");
      text(
        left + 2 + textWidth(e.kind) + 1,
        r,
        truncateText(e.label, Math.max(1, right - left - textWidth(e.kind) - 6)),
        "seq.groupLabel",
      );
      void f;
    } else if (e.t === "else") {
      const f = frames[openStack[openStack.length - 1]];
      f.elseRow = r;
      const left = 2 * e.depth;
      const right = total - 1 - 2 * e.depth;
      put(left, r, "├", "seq.group");
      put(right, r, "┤", "seq.group");
      for (let xx = left + 1; xx < right; xx++) put(xx, r, "╌", "seq.group");
      if (e.label) text(left + 2, r, ` ${truncateText(e.label, right - left - 6)} `, "seq.groupLabel");
    } else if (e.t === "close") {
      const f = frames[openStack.pop()!];
      f.close = r;
      const left = 2 * e.depth;
      const right = total - 1 - 2 * e.depth;
      put(left, r, "└", "seq.group");
      put(right, r, "┘", "seq.group");
      for (let xx = left + 1; xx < right; xx++) put(xx, r, "─", "seq.group");
    }
  });
  for (const f of frames) {
    const left = 2 * f.depth;
    const right = total - 1 - 2 * f.depth;
    for (let r = f.open + 1; r < f.close; r++) {
      if (r === f.elseRow) continue;
      put(left, r, "│", "seq.group");
      put(right, r, "│", "seq.group");
    }
  }
  const title = el.props.title ? [textCells(truncateText(String(el.props.title), total), "chart.title")] : [];
  const all = [...title, ...rows];
  const off = title.length;
  for (const t of targets) t.rect.y += off;
  for (const [, a] of anchors) a.rect.y += off;
  return {
    block: {
      w: total,
      rows: all.map((r) =>
        r.length === total ? r : r.concat(Array.from({ length: total - r.length }, () => cell(" "))),
      ),
      targets,
    },
    anchors,
    overflow: total > width,
  };
}
void fromRows;

// Adapters: DiagramResult (Dag, Erd, InfraMap) and wide blocks -> Block with targets and tooltip anchors.
import { textWidth, type Cell } from "../diagram/canvas.ts";
import { dag } from "../diagram/dag.ts";
import { erd } from "../diagram/erd.ts";
import { infra } from "../diagram/infra.ts";
import type { DiagramResult } from "../diagram/layeredDiagram.ts";
import type { El } from "../lang/library.ts";
import type { Anchor } from "./core.ts";
import type { Block, Ctx, Target } from "./types.ts";
import { cell, fromRows, padRow, textCells, vstack } from "./util.ts";

/** Crop a block wider than `width` to a window starting at the saved pan offset. */
export function panBlock(inner: Block, width: number, key: string, ctx: Ctx): Block {
  if (inner.w <= width) return inner;
  const maxPan = inner.w - width;
  const pan = Math.min(maxPan, Math.max(0, ctx.state.pan[key] ?? 0));
  ctx.state.pan[key] = pan;
  const rows = inner.rows.map((r) => {
    const seg = padRow(r, inner.w).slice(pan, pan + width);
    if (seg.length && seg[0].ch === "") seg[0] = cell(" ");
    if (seg.length && pan + width < inner.w && seg[seg.length - 1].ch !== "" && r[pan + width]?.ch === "")
      seg[seg.length - 1] = cell(" ");
    return seg;
  });
  const targets: Target[] = inner.targets.map((t) => ({
    ...t,
    rect: { ...t.rect, x: t.rect.x - pan },
    pan: { key, x: t.rect.x, w: t.rect.w, view: width, total: inner.w },
  }));
  const hint = textCells(
    `${pan > 0 ? "‹" : " "} columns ${pan + 1}–${pan + width} of ${inner.w} ${pan < maxPan ? "›" : " "}  select a node to pan`,
    "text.muted",
  );
  return { w: width, rows: [...rows, padRow(hint, width)], targets };
}

/**
 * A diagram's layout depends only on the component and the width, but the widget lays out again for a note, a tab, a resize
 * or a late picture. Routing wires is the costly part, so each result is kept with its component (and dropped with it). Nothing
 * downstream changes a result: resultBlock copies the cells.
 */
const layouts = new WeakMap<El, Map<number, DiagramResult>>();
function layoutOnce(el: El, width: number, make: () => DiagramResult): DiagramResult {
  let byWidth = layouts.get(el);
  if (!byWidth) layouts.set(el, (byWidth = new Map()));
  let res = byWidth.get(width);
  if (!res) {
    res = make();
    byWidth.set(width, res);
    if (byWidth.size > 4) byWidth.delete(byWidth.keys().next().value as number); // a window being dragged tries many widths
  }
  return res;
}

function resultBlock(
  res: DiagramResult,
  title: string | undefined,
  path: string,
  extra?: (res: DiagramResult) => Cell[][],
): Block {
  const w = Math.max(res.width, 1);
  const rows: Cell[][] = res.rows.map((r) => r.slice()); // the cells are shared and frozen
  const targets: Target[] = res.nodes.map((n) => ({
    id: `node:${path}:${n.id}`,
    kind: "node",
    rect: { ...n.rect },
    label: n.label + (n.status ? `  ${n.status}` : ""),
  }));
  const body = fromRows(w, rows, targets);
  const top: Block[] = title ? [fromRows(w, [textCells(title, "chart.title")])] : [];
  const tail: Cell[][] = [];
  if (res.legend.length) {
    const parts: Cell[] = [];
    res.legend.forEach((e, i) => {
      if (i) parts.push(cell(" "), cell(" "));
      parts.push(...textCells(e.glyph, e.role), cell(" "), ...textCells(e.text, "legend"));
    });
    tail.push([], parts);
  }
  for (const f of res.footnotes) tail.push(textCells(f, "legend"));
  for (const wn of res.warnings) tail.push(textCells(`! ${wn}`, "text.warn"));
  if (extra) tail.push(...extra(res));
  const out = vstack([...top, body, ...(tail.length ? [fromRows(w, tail)] : [])], 0, w);
  const off = top.length;
  void off;
  return out;
}

function offsetAnchors(m: Map<string, Anchor>, dy: number) {
  for (const [, a] of m) a.rect = { ...a.rect, y: a.rect.y + dy };
}

export function dagBlock(
  el: El,
  width: number,
  path: string,
  ctx: Ctx,
): { block: Block; anchors: Map<string, Anchor> } {
  const p = el.props;
  const res = layoutOnce(el, width, () =>
    dag(
      (p.nodes ?? []).map((n: El) => ({ id: n.props.id, label: n.props.label, status: n.props.status })),
      p.edges ?? [],
      { direction: p.direction ?? null, maxWidth: width },
    ),
  );
  const inner = resultBlock(res, p.title, path);
  const anchors = new Map<string, Anchor>();
  const dy = p.title ? 1 : 0;
  for (const n of res.nodes)
    anchors.set(n.id, { rect: { ...n.rect, y: n.rect.y + dy }, targetId: `node:${path}:${n.id}` });
  const block = panBlock(inner, width, `pan:${path}`, ctx);
  remapAnchors(anchors, inner, block);
  return { block, anchors };
}

export function erdBlock(
  el: El,
  width: number,
  path: string,
  ctx: Ctx,
): { block: Block; anchors: Map<string, Anchor> } {
  const p = el.props;
  const res = layoutOnce(el, width, () =>
    erd(
      (p.entities ?? []).map((e: El) => ({ name: e.props.name, fields: e.props.fields })),
      (p.relations ?? []).map((r: El) => ({
        from: r.props.from,
        to: r.props.to,
        kind: r.props.kind,
        label: r.props.label,
      })),
      { maxWidth: width },
    ),
  );
  const inner = resultBlock(res, p.title, path);
  const anchors = new Map<string, Anchor>();
  const dy = p.title ? 1 : 0;
  for (const ent of (p.entities ?? []) as El[]) {
    const node = res.nodes.find((n) => n.id === ent.props.name);
    if (!node) continue;
    anchors.set(String(ent.props.name), {
      rect: { ...node.rect, y: node.rect.y + dy },
      targetId: `node:${path}:${node.id}`,
    });
    ((ent.props.fields ?? []) as string[]).forEach((f, i) => {
      const col = String(f).trim().split(/\s+/)[0];
      if (col)
        anchors.set(`${ent.props.name}.${col}`, {
          rect: { x: node.rect.x + 1, y: node.rect.y + dy + 3 + i, w: Math.max(1, node.rect.w - 2), h: 1 },
          badge: [node.rect.x + node.rect.w - 2, node.rect.y + dy + 3 + i],
        });
    });
  }
  const block = panBlock(inner, width, `pan:${path}`, ctx);
  remapAnchors(anchors, inner, block);
  return { block, anchors };
}

export function infraBlock(
  el: El,
  width: number,
  path: string,
  ctx: Ctx,
): { block: Block; anchors: Map<string, Anchor> } {
  const p = el.props;
  const zones = (p.zones ?? []).map((z: El) => ({
    name: z.props.name,
    tone: z.props.tone,
    nodes: (z.props.nodes ?? []).map((n: El) => ({
      id: n.props.id,
      label: n.props.label,
      kind: n.props.kind,
      status: n.props.status,
      detail: n.props.detail,
    })),
  }));
  const links = (p.links ?? []).map((l: El) => ({
    from: l.props.from,
    to: l.props.to,
    label: l.props.label,
    style: l.props.style,
  }));
  const res = layoutOnce(el, width, () => infra(zones, links, { direction: p.direction ?? null, maxWidth: width }));
  const inner = resultBlock(res, p.title, path);
  const anchors = new Map<string, Anchor>();
  const dy = p.title ? 1 : 0;
  for (const n of res.nodes)
    anchors.set(n.id, { rect: { ...n.rect, y: n.rect.y + dy }, targetId: `node:${path}:${n.id}` });
  for (const z of res.zones ?? [])
    anchors.set(`zone:${z.name}`, { rect: { x: z.rect.x, y: z.rect.y + dy, w: z.rect.w, h: 1 } });
  for (const e of res.edges) {
    const c = e.labelCells[0] ?? e.cells[Math.floor(e.cells.length / 2)];
    if (c)
      anchors.set(`${e.from}>${e.to}`, {
        rect: { x: Math.max(0, c[0] - 1), y: c[1] + dy, w: Math.max(3, e.labelCells.length + 2), h: 1 },
      });
  }
  const block = panBlock(inner, width, `pan:${path}`, ctx);
  remapAnchors(anchors, inner, block);
  return { block, anchors };
}

/** after panning, anchors must follow the targets they point at */
function remapAnchors(anchors: Map<string, Anchor>, inner: Block, panned: Block) {
  if (panned === inner) return;
  const pan = panned.targets[0]?.pan;
  const dx = pan ? pan.x - panned.targets[0].rect.x : 0;
  for (const [, a] of anchors) {
    a.rect = { ...a.rect, x: a.rect.x - dx };
    if (a.badge) a.badge = [a.badge[0] - dx, a.badge[1]];
  }
}

export { offsetAnchors, textWidth };

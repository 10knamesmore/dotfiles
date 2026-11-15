// Shared builder for layered diagrams (DAG, ERD): placement -> routing -> raster.

import { Canvas, textWidth, type Cell, type Rect } from "./canvas.ts";
import { assignCross, layerize } from "./layered.ts";
import { Router, type RouteReq, type Routed, type Waypoint } from "./router.ts";
import { drawWires, type Mark, type WireSpec } from "./wires.ts";

export interface PlacedNode {
  id: string;
  index: number;
  label: string;
  status: string | null;
  rect: Rect;
}

export interface PlacedEdge {
  /** index in the normalised input edge list */
  index: number;
  from: string;
  to: string;
  reversed: boolean;
  ok: boolean;
  forced: boolean;
  label?: string;
  cells: Array<[number, number]>;
  labelCells: Array<[number, number]>;
}

export interface DiagramResult {
  kind: string;
  direction: "LR" | "TB";
  width: number;
  height: number;
  rows: Cell[][];
  nodes: PlacedNode[];
  edges: PlacedEdge[];
  warnings: string[];
  footnotes: string[];
  degraded: boolean;
  overflow: boolean;
  /** statuses in use, for tests and the pi adapter */
  statuses: string[];
  /** legend entries drawn under the diagram */
  legend: LegendEntry[];
  /** edges that could not be routed cleanly. They are not drawn and are listed in the footnotes. */
  omitted?: Array<{ from: string; to: string; label?: string }>;
  /** zones (InfraMap), for hit-testing */
  zones?: Array<{ name: string; tone: string; rect: Rect }>;
}

export interface LegendEntry {
  glyph: string;
  role: string;
  text: string;
}

export interface LNodeSpec {
  id: string;
  label: string;
  status: string | null;
  baseW: number;
  baseH: number;
  paint: (c: Canvas, r: Rect) => void;
  /** rows (relative to the box top) where no wire may attach, e.g. a table separator */
  noPortRows?: number[];
}

export interface LEdgeSpec {
  a: number;
  b: number;
  label?: string;
  dashed?: boolean;
  fromMark: Mark;
  toMark: Mark;
}

export interface LayeredSpec {
  nodes: LNodeSpec[];
  edges: LEdgeSpec[];
  /** spacing between ports on one side, default 2 */
  portSpacing?: number;
  /** draw edges that were reversed to break a cycle dashed */
  dashReversed?: boolean;
}

interface Built {
  canvas: Canvas;
  nodes: PlacedNode[];
  edges: PlacedEdge[];
  failed: number;
  forced: number;
  footnotes: string[];
  omitted: Array<{ from: string; to: string; label?: string }>;
}

const MARGIN = 3;

function build(spec: LayeredSpec, dir: "LR" | "TB", spacing: number, relax: boolean): Built | { failed: number } {
  const n = spec.nodes.length;
  const TB = dir === "TB";
  const sp = spec.portSpacing ?? 2;
  const L = layerize(
    n,
    spec.edges.map((e) => ({ a: e.a, b: e.b })),
  );
  const N = L.itemLayer.length;
  const inDeg = new Array<number>(n).fill(0);
  const outDeg = new Array<number>(n).fill(0);
  for (const e of L.edges) {
    outDeg[e.a]++;
    inDeg[e.b]++;
  }
  const w = new Array<number>(N).fill(1);
  const h = new Array<number>(N).fill(1);
  for (let v = 0; v < n; v++) {
    const k = Math.max(inDeg[v], outDeg[v]);
    const need = k > 0 ? (k - 1) * sp + 3 : 1;
    if (TB) {
      w[v] = Math.max(spec.nodes[v].baseW, need);
      h[v] = spec.nodes[v].baseH;
    } else {
      w[v] = spec.nodes[v].baseW;
      h[v] = Math.max(spec.nodes[v].baseH, need);
    }
  }
  const cs = (i: number) => (TB ? w[i] : h[i]);
  const fs = (i: number) => (TB ? h[i] : w[i]);
  const csArr = Array.from({ length: N }, (_, i) => cs(i));
  const nodeGap = (TB ? 4 : 2) + spacing;
  const cross = assignCross(
    L,
    csArr,
    (a, b) => (a < n && b < n ? nodeGap : 2),
    (u, v) => (u >= n && v >= n ? 6 : u >= n || v >= n ? 3 : 1),
  );
  const center = (i: number) => cross[i] + (csArr[i] - 1) / 2;
  // band thickness and channel gaps along the flow axis
  const T = new Array<number>(L.nLayers).fill(1);
  for (let i = 0; i < N; i++) T[L.itemLayer[i]] = Math.max(T[L.itemLayer[i]], fs(i));
  // tracks a channel needs: the largest number of horizontal spans that overlap at one position
  const nonStraight = new Array<number>(Math.max(0, L.nLayers - 1)).fill(0);
  {
    const spans: Array<Array<[number, number]>> = Array.from({ length: Math.max(0, L.nLayers - 1) }, () => []);
    for (const s of L.segs) {
      const a = center(s.u);
      const b = center(s.v);
      if (Math.abs(a - b) > 1) spans[L.itemLayer[s.u]].push([Math.min(a, b), Math.max(a, b)]);
    }
    spans.forEach((sp, l) => {
      const ev: Array<[number, number]> = [];
      for (const [lo, hi] of sp) {
        ev.push([lo, 1]);
        ev.push([hi + 0.5, -1]);
      }
      ev.sort((p, q) => p[0] - q[0] || p[1] - q[1]);
      let cur = 0;
      let best = 0;
      for (const [, d] of ev) {
        cur += d;
        best = Math.max(best, cur);
      }
      nonStraight[l] = best;
    });
  }
  const labelNeed = new Array<number>(Math.max(0, L.nLayers - 1)).fill(0);
  for (const [ei, e] of L.edges.entries()) {
    const lab = spec.edges[e.src].label;
    if (lab)
      labelNeed[L.itemLayer[L.chains[ei][0]]] = Math.max(labelNeed[L.itemLayer[L.chains[ei][0]]], textWidth(lab));
  }
  const gapFlow = (l: number) => {
    const ns = nonStraight[l];
    let g = (TB ? 2 : 5) + ns + spacing * 2;
    if (labelNeed[l]) g = Math.max(g, TB ? 3 : labelNeed[l] + 4);
    return g;
  };
  const F = new Array<number>(L.nLayers).fill(0);
  F[0] = MARGIN;
  for (let l = 0; l + 1 < L.nLayers; l++) F[l + 1] = F[l] + T[l] + gapFlow(l);
  const rect: Rect[] = [];
  let W = 0;
  let H = 0;
  for (let i = 0; i < N; i++) {
    const l = L.itemLayer[i];
    const off = Math.floor((T[l] - fs(i)) / 2);
    const r = TB
      ? { x: MARGIN + cross[i], y: F[l] + off, w: w[i], h: h[i] }
      : { x: F[l] + off, y: MARGIN + cross[i], w: w[i], h: h[i] };
    rect.push(r);
    W = Math.max(W, r.x + r.w);
    H = Math.max(H, r.y + r.h);
  }
  W += MARGIN;
  H += MARGIN;
  const router = new Router(W, H);
  for (let v = 0; v < n; v++) {
    router.addSolid(rect[v], v);
    for (const row of spec.nodes[v].noPortRows ?? []) {
      router.addSolid({ x: rect[v].x - 1, y: rect[v].y + row, w: 1, h: 1 });
      router.addSolid({ x: rect[v].x + rect[v].w, y: rect[v].y + row, w: 1, h: 1 });
    }
  }

  // reserve the lanes of long edges so no other wire can turn or end on them
  for (const [ei, chain] of L.chains.entries()) {
    for (let k = 1; k + 1 < chain.length; k++) {
      const it = chain[k];
      const l = L.itemLayer[it];
      const r = rect[it];
      const cells = Array.from({ length: T[l] }, (_, d) => (TB ? { x: r.x, y: F[l] + d } : { x: F[l] + d, y: r.y }));
      router.reserve(ei, cells, TB);
    }
  }
  const fromPen = TB ? [300, 120, 0, 120] : [120, 0, 120, 300];
  const toPen = TB ? [0, 120, 300, 120] : [120, 300, 120, 0];
  const order = L.edges
    .map((_, i) => i)
    .sort((p, q) => {
      const sp1 = L.chains[p].length;
      const sp2 = L.chains[q].length;
      return sp2 - sp1 || p - q;
    });
  const routed = new Map<number, Routed>();
  let failed = 0;
  const forced = 0;
  const omitted: Built["omitted"] = [];
  for (const ei of order) {
    const e = L.edges[ei];
    const chain = L.chains[ei];
    const wps: Waypoint[] = [];
    for (let k = 1; k + 1 < chain.length; k++) {
      const it = chain[k];
      const l = L.itemLayer[it];
      const r = rect[it];
      const pts: Waypoint[] = TB
        ? [
            { x: r.x, y: F[l], dir: 2 },
            { x: r.x, y: F[l] + T[l] - 1, dir: 2 },
          ]
        : [
            { x: F[l], y: r.y, dir: 1 },
            { x: F[l] + T[l] - 1, y: r.y, dir: 1 },
          ];
      for (const p of pts)
        if (!wps.length || wps[wps.length - 1].x !== p.x || wps[wps.length - 1].y !== p.y) wps.push(p);
    }
    const req: RouteReq = {
      id: ei,
      from: rect[e.a],
      to: rect[e.b],
      fromId: e.a,
      toId: e.b,
      fromPen,
      toPen,
      waypoints: wps,
    };
    const r = router.route(req, false);
    if (!r.ok) {
      if (!relax) return { failed: failed + 1 };
      failed++;
      const o = spec.edges[e.src];
      omitted.push({ from: spec.nodes[o.a].id, to: spec.nodes[o.b].id, label: o.label });
      continue;
    }
    router.commit(r);
    routed.set(ei, r);
  }

  // raster
  const canvas = new Canvas(W, H);
  for (let v = 0; v < n; v++) spec.nodes[v].paint(canvas, rect[v]);
  const wires: WireSpec[] = [];
  for (const [ei, r] of routed) {
    const e = L.edges[ei];
    const o = spec.edges[e.src];
    wires.push({
      id: ei,
      steps: r.steps,
      dashed: !!o.dashed || (!!spec.dashReversed && e.rev),
      startMark: e.rev ? o.toMark : o.fromMark,
      endMark: e.rev ? o.fromMark : o.toMark,
      label: o.label,
      desc: `${spec.nodes[o.a].id} -> ${spec.nodes[o.b].id}`,
      role: "wire",
    });
  }
  const { footnotes, labelCells } = drawWires(canvas, wires, (x, y) => router.solid[y * W + x] === 1);

  // crop to content
  let minX = W;
  let minY = H;
  let maxX = -1;
  let maxY = -1;
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++)
      if (canvas.getCh(x, y) !== " " || canvas.getRole(x, y) !== "") {
        minX = Math.min(minX, x);
        maxX = Math.max(maxX, x);
        minY = Math.min(minY, y);
        maxY = Math.max(maxY, y);
      }
  if (maxX < 0) {
    minX = minY = 0;
    maxX = maxY = 0;
  }
  const out = new Canvas(maxX - minX + 1, maxY - minY + 1);
  for (let y = 0; y < out.h; y++)
    for (let x = 0; x < out.w; x++) out.set(x, y, canvas.getCh(x + minX, y + minY), canvas.getRole(x + minX, y + minY));
  const nodes: PlacedNode[] = spec.nodes.map((s, i) => ({
    id: s.id,
    index: i,
    label: s.label,
    status: s.status,
    rect: { x: rect[i].x - minX, y: rect[i].y - minY, w: rect[i].w, h: rect[i].h },
  }));
  const edges: PlacedEdge[] = [];
  for (const [ei, r] of [...routed.entries()].sort((a, b) => a[0] - b[0])) {
    const e = L.edges[ei];
    const o = spec.edges[e.src];
    edges.push({
      index: e.src,
      from: spec.nodes[o.a].id,
      to: spec.nodes[o.b].id,
      reversed: e.rev,
      ok: true,
      forced: r.forced,
      label: o.label,
      cells: r.steps.map((s) => [s.x - minX, s.y - minY] as [number, number]),
      labelCells: (labelCells.get(ei) ?? []).map(([x, y]) => [x - minX, y - minY] as [number, number]),
    });
  }
  for (const o of omitted) footnotes.push(`${o.from} -> ${o.to}${o.label ? ` (${o.label})` : ""} (not drawn)`);
  return { canvas: out, nodes, edges, failed, forced, footnotes, omitted };
}

export function layeredDiagram(kind: string, spec: LayeredSpec, dir: "LR" | "TB"): DiagramResult {
  let built: Built | null = null;
  for (const spacing of [0, 1, 2, 4, 7]) {
    const r = build(spec, dir, spacing, false);
    if ("canvas" in r) {
      built = r;
      break;
    }
  }
  if (!built) {
    const r = build(spec, dir, 4, true);
    if ("canvas" in r) built = r;
  }
  if (!built) throw new Error("layeredDiagram: could not place diagram");
  return {
    kind,
    direction: dir,
    width: built.canvas.w,
    height: built.canvas.h,
    rows: built.canvas.rows(),
    nodes: built.nodes,
    edges: built.edges,
    warnings: [],
    footnotes: built.footnotes,
    degraded: built.omitted.length > 0,
    overflow: false,
    statuses: [],
    legend: [],
    omitted: built.omitted,
  };
}

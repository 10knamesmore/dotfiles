// InfraMap(zones, links, direction?) -> DiagramResult.
// Zones are columns (LR) or rows (TB) that hold stacked nodes. Links are routed wires with a dot at
// the source, an arrowhead at the target and an optional label. Never throws on bad input.

import { Canvas, oneLine, textWidth, truncateText, type Rect } from "./canvas.ts";
import { paintBox } from "./dag.ts";
import type { DiagramResult, LegendEntry, PlacedEdge, PlacedNode } from "./layeredDiagram.ts";
import { budgeted, Router, type RouteReq, type Routed } from "./router.ts";
import { drawWires, type WireSpec } from "./wires.ts";

export const INFRA_KINDS = ["client", "gateway", "service", "gpu", "database", "storage", "queue", "cloud"] as const;
export const INFRA_STATUSES = ["up", "degraded", "down", "unknown"] as const;
export const INFRA_TONES = ["neutral", "info", "good", "warn", "bad"] as const;
// Every icon must be one cell wide in pi (test/width.test.ts): a wide one (the old storage icon, U+2630, was) slides the rest of its row.
export const KIND_ICON: Record<string, string> = {
  client: "□",
  gateway: "↔",
  service: "◇",
  gpu: "⌸",
  database: "◎",
  storage: "⊟",
  queue: "≡",
  cloud: "◌",
};
const STATUS_GLYPH: Record<string, string> = { up: "●", degraded: "●", down: "●", unknown: "○" };

export interface InfraNodeIn {
  id: string;
  label?: string;
  kind?: string | null;
  status?: string | null;
  detail?: string | null;
}
export interface InfraZoneIn {
  name: string;
  nodes?: unknown;
  tone?: string | null;
}
export interface InfraLinkIn {
  from: string;
  to: string;
  label?: string | null;
  style?: string | null;
}
export interface InfraOptions {
  direction?: "LR" | "TB" | null;
  maxWidth?: number;
}

interface INode {
  id: string;
  label: string;
  kind: string | null;
  status: string | null;
  detail: string;
  zone: number;
}
interface IZone {
  name: string;
  tone: string;
  nodes: number[];
}
interface NormInfra {
  zones: IZone[];
  nodes: INode[];
  links: Array<{ a: number; b: number; label?: string; dashed: boolean }>;
  warnings: string[];
}

export function normalizeInfra(zonesIn: unknown, linksIn: unknown): NormInfra {
  const warnings: string[] = [];
  const zones: IZone[] = [];
  const nodes: INode[] = [];
  const ids = new Map<string, number>();
  for (const rawZone of Array.isArray(zonesIn) ? zonesIn : []) {
    const z = (rawZone ?? {}) as InfraZoneIn;
    const name = oneLine(z.name ?? "");
    let tone = "neutral";
    if (z.tone != null && z.tone !== "") {
      if ((INFRA_TONES as readonly string[]).includes(String(z.tone))) tone = String(z.tone);
      else warnings.push(`unknown zone tone "${z.tone}" ignored`);
    }
    const zi = zones.length;
    const zone: IZone = { name: name || `zone ${zi + 1}`, tone, nodes: [] };
    for (const rawNode of Array.isArray(z.nodes) ? z.nodes : []) {
      const n = (rawNode ?? {}) as InfraNodeIn;
      const id = n.id == null ? "" : String(n.id);
      if (!id) {
        warnings.push("a node without an id was skipped");
        continue;
      }
      if (ids.has(id)) {
        warnings.push(`duplicate node id "${id}" ignored`);
        continue;
      }
      let kind: string | null = null;
      if (n.kind != null && n.kind !== "") {
        if ((INFRA_KINDS as readonly string[]).includes(String(n.kind))) kind = String(n.kind);
        else warnings.push(`unknown kind "${n.kind}" on "${id}" ignored`);
      }
      let status: string | null = null;
      if (n.status != null && n.status !== "") {
        if ((INFRA_STATUSES as readonly string[]).includes(String(n.status))) status = String(n.status);
        else warnings.push(`unknown status "${n.status}" on "${id}" ignored`);
      }
      ids.set(id, nodes.length);
      zone.nodes.push(nodes.length);
      nodes.push({
        id,
        label: oneLine(n.label ?? id) || id,
        kind,
        status,
        detail: n.detail ? oneLine(n.detail) : "",
        zone: zi,
      });
    }
    if (!zone.nodes.length) {
      warnings.push(`zone "${zone.name}" has no nodes and was skipped`);
      continue;
    }
    zones.push(zone);
  }
  // zone indexes may have shifted when empty zones were skipped
  zones.forEach((z, zi) => z.nodes.forEach((ni) => (nodes[ni].zone = zi)));
  const links: NormInfra["links"] = [];
  const seen = new Set<string>();
  for (const raw of Array.isArray(linksIn) ? linksIn : []) {
    const l = (raw ?? {}) as InfraLinkIn;
    const a = ids.get(String(l.from));
    const b = ids.get(String(l.to));
    if (a === undefined || b === undefined) {
      warnings.push(`link ${String(l.from)} -> ${String(l.to)} names an unknown node and was skipped`);
      continue;
    }
    if (a === b) {
      warnings.push(`link from "${nodes[a].id}" to itself was skipped`);
      continue;
    }
    const label = l.label ? oneLine(l.label) : undefined;
    const dashed = l.style === "dashed";
    const key = `${a}>${b}>${label ?? ""}>${dashed}`;
    if (seen.has(key)) continue;
    seen.add(key);
    links.push({ a, b, label, dashed });
  }
  return { zones, nodes, links, warnings };
}

interface Caps {
  label: number;
  detail: number;
  link: number;
}

const MARGIN = 3;

interface Laid {
  canvas: Canvas;
  nodes: PlacedNode[];
  zones: Array<{ name: string; tone: string; rect: Rect }>;
  edges: PlacedEdge[];
  failed: number;
  forced: number;
  footnotes: string[];
  omitted: Array<{ from: string; to: string; label?: string }>;
}

function layout(
  norm: NormInfra,
  dir: "LR" | "TB",
  caps: Caps,
  spacing: number,
  relax: boolean,
): Laid | { failed: number } {
  const TB = dir === "TB";
  const n = norm.nodes.length;
  // node content
  const shown = norm.nodes.map((nd) => {
    const label = truncateText(nd.label, caps.label);
    const detail = nd.detail ? truncateText(nd.detail, caps.detail) : "";
    const icon = nd.kind ? KIND_ICON[nd.kind] : "";
    const l1 = (icon ? 2 : 0) + textWidth(label) + (nd.status ? 2 : 0);
    const inner = Math.max(l1, textWidth(detail));
    return { label, detail, icon, w: inner + 4, h: detail ? 4 : 3 };
  });
  // uniform node size per zone along the flow axis
  const nodeW = new Array<number>(n).fill(0);
  const nodeH = new Array<number>(n).fill(0);
  for (const z of norm.zones) {
    const maxW = Math.max(...z.nodes.map((i) => shown[i].w));
    const maxH = Math.max(...z.nodes.map((i) => shown[i].h));
    for (const i of z.nodes) {
      nodeW[i] = TB ? shown[i].w : maxW;
      nodeH[i] = TB ? maxH : shown[i].h;
    }
  }
  const padF = TB ? 2 : 3;
  const padC = TB ? 3 : 2;
  const gapC = (TB ? 4 : 3) + spacing;
  // gap after the k-th node of a zone along the cross axis: wide enough for a label of a link between neighbours (TB)
  const gapAfter = (zi: number, k: number): number => {
    const z = norm.zones[zi];
    if (!TB || k + 1 >= z.nodes.length) return gapC;
    const a = z.nodes[k];
    const b = z.nodes[k + 1];
    let need = 0;
    for (const l of norm.links)
      if (l.label && ((l.a === a && l.b === b) || (l.a === b && l.b === a)))
        need = Math.max(need, Math.min(textWidth(l.label), caps.link));
    return Math.max(gapC, need ? need + 4 : 0);
  };
  // zone sizes
  const zoneF: number[] = [];
  const zoneC: number[] = [];
  norm.zones.forEach((z, zi) => {
    const fs = Math.max(...z.nodes.map((i) => (TB ? nodeH[i] : nodeW[i])));
    const cs = z.nodes.reduce((acc, i, k) => acc + (TB ? nodeW[i] : nodeH[i]) + (k ? gapAfter(zi, k - 1) : 0), 0);
    zoneF.push(fs + 2 * padF);
    zoneC.push(cs + 2 * padC);
  });
  // make sure a zone is wide enough for its caption
  norm.zones.forEach((z, zi) => {
    const need = textWidth(z.name) + 6;
    if (TB) zoneC[zi] = Math.max(zoneC[zi], need);
    else zoneF[zi] = Math.max(zoneF[zi], need);
  });
  // channels between zones
  const labelNeed = norm.zones.map(() => 0);
  for (const l of norm.links) {
    const za = norm.nodes[l.a].zone;
    const zb = norm.nodes[l.b].zone;
    if (l.label && Math.abs(za - zb) === 1) {
      const k = Math.min(za, zb);
      labelNeed[k] = Math.max(labelNeed[k], Math.min(textWidth(l.label), caps.link));
    }
  }
  const chan = (zi: number) => (TB ? 5 + spacing * 2 : 5 + Math.max(0, labelNeed[zi] - 4) + spacing * 2);
  const F: number[] = [];
  let pos = MARGIN;
  norm.zones.forEach((_, zi) => {
    F.push(pos);
    pos += zoneF[zi] + chan(zi);
  });
  const zoneRect: Rect[] = norm.zones.map((_, zi) =>
    TB ? { x: MARGIN, y: F[zi], w: zoneC[zi], h: zoneF[zi] } : { x: F[zi], y: MARGIN, w: zoneF[zi], h: zoneC[zi] },
  );
  const rect: Rect[] = new Array(n);
  norm.zones.forEach((z, zi) => {
    let c = padC;
    z.nodes.forEach((i, k) => {
      const zr = zoneRect[zi];
      rect[i] = TB
        ? { x: zr.x + c, y: zr.y + padF, w: nodeW[i], h: nodeH[i] }
        : { x: zr.x + padF, y: zr.y + c, w: nodeW[i], h: nodeH[i] };
      c += (TB ? nodeW[i] : nodeH[i]) + gapAfter(zi, k);
    });
  });
  let W = 0;
  let H = 0;
  for (const r of [...zoneRect, ...rect]) {
    W = Math.max(W, r.x + r.w);
    H = Math.max(H, r.y + r.h);
  }
  W += MARGIN;
  H += MARGIN;
  const router = new Router(W, H);
  for (const zr of zoneRect) router.addZoneBorder(zr);
  rect.forEach((r, i) => router.addSolid(r, i));

  // links
  const center = (r: Rect) => ({ x: r.x + (r.w - 1) / 2, y: r.y + (r.h - 1) / 2 });
  const order = norm.links
    .map((_, i) => i)
    .sort((p, q) => {
      const dp =
        Math.abs(center(rect[norm.links[p].a]).x - center(rect[norm.links[p].b]).x) +
        Math.abs(center(rect[norm.links[p].a]).y - center(rect[norm.links[p].b]).y);
      const dq =
        Math.abs(center(rect[norm.links[q].a]).x - center(rect[norm.links[q].b]).x) +
        Math.abs(center(rect[norm.links[q].a]).y - center(rect[norm.links[q].b]).y);
      return dp - dq || p - q;
    });
  const routed = new Map<number, Routed>();
  let failed = 0;
  const forced = 0;
  const omitted: Laid["omitted"] = [];
  for (const li of order) {
    const l = norm.links[li];
    const za = norm.nodes[l.a].zone;
    const zb = norm.nodes[l.b].zone;
    // preferred sides: 0 for the side facing the other node, 120 for the sides next to it, 300 for the far side
    let outSide: number;
    let inSide: number;
    if (za === zb) {
      const ca = center(rect[l.a]);
      const cb = center(rect[l.b]);
      if (TB) {
        outSide = cb.x >= ca.x ? 1 : 3;
        inSide = cb.x >= ca.x ? 3 : 1;
      } else {
        outSide = cb.y >= ca.y ? 2 : 0;
        inSide = cb.y >= ca.y ? 0 : 2;
      }
    } else {
      const forward = zb > za;
      if (TB) {
        outSide = forward ? 2 : 0;
        inSide = forward ? 0 : 2;
      } else {
        outSide = forward ? 1 : 3;
        inSide = forward ? 3 : 1;
      }
    }
    const pens = (side: number) => [0, 1, 2, 3].map((s) => (s === side ? 0 : s === (side + 2) % 4 ? 300 : 120));
    const req: RouteReq = {
      id: li,
      from: rect[l.a],
      to: rect[l.b],
      fromId: l.a,
      toId: l.b,
      fromPen: pens(outSide),
      toPen: pens(inSide),
      waypoints: [],
    };
    const r = router.route(req, false);
    if (!r.ok) {
      if (!relax) return { failed: failed + 1 };
      failed++;
      omitted.push({ from: norm.nodes[l.a].id, to: norm.nodes[l.b].id, label: l.label });
      continue;
    }
    router.commit(r);
    routed.set(li, r);
  }

  // raster
  const canvas = new Canvas(W, H);
  norm.zones.forEach((z, zi) => {
    const zr = zoneRect[zi];
    const role = `zone.${z.tone}`;
    canvas.set(zr.x, zr.y, "┌", role);
    canvas.set(zr.x + zr.w - 1, zr.y, "┐", role);
    canvas.set(zr.x, zr.y + zr.h - 1, "└", role);
    canvas.set(zr.x + zr.w - 1, zr.y + zr.h - 1, "┘", role);
    for (let x = zr.x + 1; x < zr.x + zr.w - 1; x++) {
      canvas.set(x, zr.y, "─", role);
      canvas.set(x, zr.y + zr.h - 1, "─", role);
    }
    for (let y = zr.y + 1; y < zr.y + zr.h - 1; y++) {
      canvas.set(zr.x, y, "│", role);
      canvas.set(zr.x + zr.w - 1, y, "│", role);
    }
    canvas.text(zr.x + 2, zr.y, ` ${truncateText(z.name, zr.w - 6)} `, `zone.title.${z.tone}`);
  });
  norm.nodes.forEach((nd, i) => {
    const r = rect[i];
    const sh = shown[i];
    const bad = nd.status === "degraded" || nd.status === "down" || nd.status === "unknown";
    paintBox(canvas, r, bad ? `box.${nd.status}` : "box", nd.status === "unknown");
    const y = r.y + 1;
    let x = r.x + 2;
    if (sh.icon) {
      canvas.set(x, y, sh.icon, `icon.${nd.kind}`);
      x += 2;
    }
    canvas.text(x, y, sh.label, "label");
    if (nd.status) canvas.set(r.x + r.w - 3, y, STATUS_GLYPH[nd.status], `status.${nd.status}`);
    if (sh.detail) canvas.text(r.x + 2, y + 1, sh.detail, "detail");
  });
  const wires: WireSpec[] = [];
  for (const [li, r] of routed) {
    const l = norm.links[li];
    wires.push({
      id: li,
      steps: r.steps,
      dashed: l.dashed,
      startMark: "dot",
      endMark: "arrow",
      label: l.label ? truncateText(l.label, caps.link) : undefined,
      desc: `${norm.nodes[l.a].id} -> ${norm.nodes[l.b].id}`,
      role: "wire",
    });
  }
  const { footnotes, labelCells } = drawWires(canvas, wires, (x, y) => router.solid[y * W + x] === 1);

  // crop
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
  const out = new Canvas(maxX - minX + 1, maxY - minY + 1);
  for (let y = 0; y < out.h; y++)
    for (let x = 0; x < out.w; x++) out.set(x, y, canvas.getCh(x + minX, y + minY), canvas.getRole(x + minX, y + minY));
  const sh = (r: Rect): Rect => ({ x: r.x - minX, y: r.y - minY, w: r.w, h: r.h });
  const nodes: PlacedNode[] = norm.nodes.map((nd, i) => ({
    id: nd.id,
    index: i,
    label: nd.label,
    status: nd.status,
    rect: sh(rect[i]),
  }));
  const edges: PlacedEdge[] = [...routed.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([li, r]) => ({
      index: li,
      from: norm.nodes[norm.links[li].a].id,
      to: norm.nodes[norm.links[li].b].id,
      reversed: false,
      ok: true,
      forced: r.forced,
      label: norm.links[li].label,
      cells: r.steps.map((s) => [s.x - minX, s.y - minY] as [number, number]),
      labelCells: (labelCells.get(li) ?? []).map(([x, y]) => [x - minX, y - minY] as [number, number]),
    }));
  for (const o of omitted) footnotes.push(`${o.from} -> ${o.to}${o.label ? ` (${o.label})` : ""} (not drawn)`);
  return {
    canvas: out,
    nodes,
    zones: norm.zones.map((z, zi) => ({ name: z.name, tone: z.tone, rect: sh(zoneRect[zi]) })),
    edges,
    failed,
    forced,
    footnotes,
    omitted,
  };
}

function build(norm: NormInfra, dir: "LR" | "TB", caps: Caps): DiagramResult {
  let laid: Laid | null = null;
  for (const spacing of [0, 1, 2, 4, 7]) {
    const r = layout(norm, dir, caps, spacing, false);
    if ("canvas" in r) {
      laid = r;
      break;
    }
  }
  if (!laid) {
    const r = layout(norm, dir, caps, 4, true);
    if ("canvas" in r) laid = r;
  }
  if (!laid) throw new Error("infra: could not place diagram");
  const used = INFRA_STATUSES.filter((s) => norm.nodes.some((nd) => nd.status === s));
  const legend: LegendEntry[] = used.map((s) => ({ glyph: STATUS_GLYPH[s], role: `status.${s}`, text: s }));
  return {
    kind: "infra",
    direction: dir,
    width: laid.canvas.w,
    height: laid.canvas.h,
    rows: laid.canvas.rows(),
    nodes: laid.nodes,
    edges: laid.edges,
    warnings: [],
    footnotes: laid.footnotes,
    degraded: laid.omitted.length > 0,
    overflow: false,
    statuses: used,
    legend,
    zones: laid.zones,
    omitted: laid.omitted,
  };
}

const CAPS: Caps[] = [
  { label: 22, detail: 26, link: 18 },
  { label: 16, detail: 18, link: 12 },
  { label: 12, detail: 12, link: 9 },
  { label: 8, detail: 8, link: 6 },
];

export function infra(zonesIn: unknown, linksIn: unknown, opts: InfraOptions = {}): DiagramResult {
  return budgeted(() => infraLayout(zonesIn, linksIn, opts));
}

function infraLayout(zonesIn: unknown, linksIn: unknown, opts: InfraOptions): DiagramResult {
  const norm = normalizeInfra(zonesIn, linksIn);
  const maxWidth = Math.max(20, opts.maxWidth ?? 100);
  if (!norm.nodes.length) {
    return {
      kind: "infra",
      direction: "LR",
      width: 0,
      height: 0,
      rows: [],
      nodes: [],
      edges: [],
      warnings: [...norm.warnings, "the map has no nodes"],
      footnotes: [],
      degraded: false,
      overflow: false,
      statuses: [],
      legend: [],
      zones: [],
    };
  }
  const dirs: Array<"LR" | "TB"> = opts.direction === "LR" || opts.direction === "TB" ? [opts.direction] : ["LR", "TB"];
  let best: DiagramResult | null = null;
  let chosen: DiagramResult | null = null;
  for (const caps of CAPS) {
    for (const d of dirs) {
      const r = build(norm, d, caps);
      if (!best || r.width < best.width) best = r;
      if (r.width <= maxWidth) {
        chosen = r;
        break;
      }
    }
    if (chosen) break;
  }
  if (!chosen) {
    const tries = dirs.map((d) => build(norm, d, CAPS[1]));
    best = tries.reduce((a, b) => (b.width < a.width ? b : a));
  }
  const res = chosen ?? best!;
  res.overflow = !chosen;
  res.warnings = [...norm.warnings];
  if (res.degraded) res.warnings.push("some links could not be routed cleanly");
  return res;
}

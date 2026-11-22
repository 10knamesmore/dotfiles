// Dag(nodes, edges, direction?) -> DiagramResult. Never throws on bad input.

import { Canvas, oneLine, textWidth, truncateText, type Rect } from "./canvas.ts";
import { layeredDiagram, type DiagramResult, type LEdgeSpec, type LNodeSpec } from "./layeredDiagram.ts";
import { budgeted } from "./router.ts";

export const DAG_STATUSES = ["done", "running", "failed", "pending", "skipped"] as const;
export type DagStatus = (typeof DAG_STATUSES)[number];

export const STATUS_ICON: Record<string, string> = { done: "✓", running: "◔", failed: "✗", pending: "○", skipped: "⊘" };

export interface DagNodeIn {
  id: string;
  label?: string;
  status?: string | null;
}

export interface DagOptions {
  direction?: "LR" | "TB" | null;
  /** columns available, default 100 */
  maxWidth?: number;
  /** longest label before truncation, default 28 */
  maxLabel?: number;
}

interface Norm {
  nodes: Array<{ id: string; label: string; status: DagStatus | null }>;
  edges: Array<{ a: number; b: number }>;
  warnings: string[];
  selfLoops: Set<number>;
}

export function normalizeDag(nodesIn: unknown, edgesIn: unknown): Norm {
  const warnings: string[] = [];
  const nodes: Norm["nodes"] = [];
  const ids = new Map<string, number>();
  for (const raw of Array.isArray(nodesIn) ? nodesIn : []) {
    const r = (raw ?? {}) as DagNodeIn;
    const id = typeof r.id === "string" ? r.id : r.id == null ? "" : String(r.id);
    if (!id) {
      warnings.push("a node without an id was skipped");
      continue;
    }
    if (ids.has(id)) {
      warnings.push(`duplicate node id "${id}" ignored`);
      continue;
    }
    let status: DagStatus | null = null;
    if (r.status != null && r.status !== "") {
      if ((DAG_STATUSES as readonly string[]).includes(String(r.status))) status = r.status as DagStatus;
      else warnings.push(`unknown status "${r.status}" on "${id}" ignored`);
    }
    ids.set(id, nodes.length);
    nodes.push({ id, label: oneLine(r.label ?? id) || id, status });
  }
  const edges: Norm["edges"] = [];
  const seen = new Set<string>();
  const selfLoops = new Set<number>();
  for (const raw of Array.isArray(edgesIn) ? edgesIn : []) {
    if (!Array.isArray(raw) || raw.length < 2) {
      warnings.push("an edge that is not a [from, to] pair was skipped");
      continue;
    }
    const a = ids.get(String(raw[0]));
    const b = ids.get(String(raw[1]));
    if (a === undefined || b === undefined) {
      warnings.push(`edge ${String(raw[0])} -> ${String(raw[1])} names an unknown node and was skipped`);
      continue;
    }
    if (a === b) {
      selfLoops.add(a);
      warnings.push(`self loop on "${nodes[a].id}" shown as a marker`);
      continue;
    }
    const key = `${a}>${b}`;
    if (seen.has(key)) continue;
    seen.add(key);
    edges.push({ a, b });
  }
  return { nodes, edges, warnings, selfLoops };
}

export function paintBox(c: Canvas, r: Rect, role: string, dashed: boolean): void {
  const hz = dashed ? "╌" : "─";
  const vt = dashed ? "╎" : "│";
  c.set(r.x, r.y, "╭", role);
  c.set(r.x + r.w - 1, r.y, "╮", role);
  c.set(r.x, r.y + r.h - 1, "╰", role);
  c.set(r.x + r.w - 1, r.y + r.h - 1, "╯", role);
  for (let x = r.x + 1; x < r.x + r.w - 1; x++) {
    c.set(x, r.y, hz, role);
    c.set(x, r.y + r.h - 1, hz, role);
  }
  for (let y = r.y + 1; y < r.y + r.h - 1; y++) {
    c.set(r.x, y, vt, role);
    c.set(r.x + r.w - 1, y, vt, role);
  }
}

function nodeSpec(n: Norm["nodes"][number], cap: number, loop: boolean): LNodeSpec {
  const label = truncateText(n.label, cap);
  const icon = n.status ? STATUS_ICON[n.status] : "";
  const shown = (loop ? "∞ " : "") + label;
  const cw = (icon ? 2 : 0) + textWidth(shown);
  return {
    id: n.id,
    label: n.label,
    status: n.status,
    baseW: cw + 4,
    baseH: 3,
    paint: (c, r) => {
      const role = n.status ? `box.${n.status}` : "box";
      paintBox(c, r, role, n.status === "skipped");
      const y = r.y + Math.floor(r.h / 2);
      const x = r.x + 1 + Math.floor((r.w - 2 - cw) / 2);
      let cx = x;
      if (icon) {
        c.set(cx, y, icon, `icon.${n.status}`);
        cx += 2;
      }
      c.text(cx, y, shown, "label");
    },
  };
}

function attempt(norm: Norm, dir: "LR" | "TB", cap: number): DiagramResult {
  const nodes = norm.nodes.map((n, i) => nodeSpec(n, cap, norm.selfLoops.has(i)));
  const edges: LEdgeSpec[] = norm.edges.map((e) => ({ a: e.a, b: e.b, fromMark: "none", toMark: "arrow" }));
  return layeredDiagram("dag", { nodes, edges, dashReversed: true }, dir);
}

const CAPS = [28, 20, 14, 10, 8, 6];

export function dag(nodesIn: unknown, edgesIn: unknown, opts: DagOptions = {}): DiagramResult {
  return budgeted(() => dagLayout(nodesIn, edgesIn, opts));
}

function dagLayout(nodesIn: unknown, edgesIn: unknown, opts: DagOptions): DiagramResult {
  const norm = normalizeDag(nodesIn, edgesIn);
  const maxWidth = Math.max(20, opts.maxWidth ?? 100);
  const maxLabel = Math.max(6, opts.maxLabel ?? 28);
  if (!norm.nodes.length) {
    return {
      kind: "dag",
      direction: "LR",
      width: 0,
      height: 0,
      rows: [],
      nodes: [],
      edges: [],
      warnings: [...norm.warnings, "the graph has no nodes"],
      footnotes: [],
      degraded: false,
      overflow: false,
      statuses: [],
      legend: [],
    };
  }
  const dirs: Array<"LR" | "TB"> = opts.direction === "LR" || opts.direction === "TB" ? [opts.direction] : ["LR", "TB"];
  const longest = Math.max(...norm.nodes.map((n) => textWidth(n.label)));
  let best: DiagramResult | null = null;
  let chosen: DiagramResult | null = null;
  for (const cap of [maxLabel, ...CAPS.filter((c) => c < maxLabel)]) {
    for (const d of dirs) {
      const r = attempt(norm, d, cap);
      if (!best || r.width < best.width) best = r;
      if (r.width <= maxWidth) {
        chosen = r;
        break;
      }
    }
    if (chosen) break;
    if (longest <= cap) break; // shrinking more changes nothing
  }
  if (!chosen) {
    // nothing fits even with short labels: the view will pan anyway, so keep the labels readable
    const cap = Math.min(maxLabel, 14);
    const tries = dirs.map((d) => attempt(norm, d, cap));
    best = tries.reduce((a, b) => (b.width < a.width ? b : a));
  }
  const res = chosen ?? best!;
  res.overflow = !chosen;
  res.warnings = [...norm.warnings];
  if (res.degraded) res.warnings.push("some edges could not be routed cleanly");
  res.statuses = DAG_STATUSES.filter((s) => norm.nodes.some((n) => n.status === s));
  res.legend = res.statuses.map((s) => ({
    glyph: STATUS_ICON[s],
    role: `icon.${s}`,
    text: s + (s === "skipped" ? " (dashed)" : ""),
  }));
  return res;
}

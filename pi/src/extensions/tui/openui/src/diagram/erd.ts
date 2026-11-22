// Erd(entities, relations) -> DiagramResult. Tables are boxes, relations are wires with
// bar (one) and crow's-foot (many) ends. Never throws on bad input.

import { Canvas, oneLine, textWidth, truncateText, type Rect } from "./canvas.ts";
import { paintBox } from "./dag.ts";
import { layeredDiagram, type DiagramResult, type LEdgeSpec, type LNodeSpec } from "./layeredDiagram.ts";
import { budgeted } from "./router.ts";
import type { Mark } from "./wires.ts";

export interface EntityIn {
  name: string;
  fields?: unknown;
}
export interface RelationIn {
  from: string;
  to: string;
  kind?: string | null;
  label?: string | null;
}
export interface ErdOptions {
  direction?: "LR" | "TB" | null;
  maxWidth?: number;
}

export type RelKind = "1:1" | "1:N" | "N:1" | "N:M";
const KINDS: RelKind[] = ["1:1", "1:N", "N:1", "N:M"];

interface Field {
  name: string;
  type: string;
  flags: string;
}
interface NormErd {
  entities: Array<{ name: string; fields: Field[] }>;
  rels: Array<{ a: number; b: number; kind: RelKind; label?: string }>;
  selfRefs: Set<number>;
  warnings: string[];
}

export function endMarks(kind: RelKind): { from: Mark; to: Mark } {
  switch (kind) {
    case "1:1":
      return { from: "one", to: "one" };
    case "N:1":
      return { from: "many", to: "one" };
    case "N:M":
      return { from: "many", to: "many" };
    default:
      return { from: "one", to: "many" };
  }
}

export function normalizeErd(entitiesIn: unknown, relationsIn: unknown): NormErd {
  const warnings: string[] = [];
  const entities: NormErd["entities"] = [];
  const ids = new Map<string, number>();
  for (const raw of Array.isArray(entitiesIn) ? entitiesIn : []) {
    const r = (raw ?? {}) as EntityIn;
    const name = oneLine(r.name ?? "");
    if (!name) {
      warnings.push("an entity without a name was skipped");
      continue;
    }
    if (ids.has(name)) {
      warnings.push(`duplicate entity "${name}" ignored`);
      continue;
    }
    const fields: Field[] = [];
    for (const f of Array.isArray(r.fields) ? r.fields : []) {
      const words = oneLine(String(f ?? ""))
        .split(" ")
        .filter(Boolean);
      if (!words.length) continue;
      fields.push({ name: words[0], type: words[1] ?? "", flags: words.slice(2).join(" ") });
    }
    ids.set(name, entities.length);
    entities.push({ name, fields });
  }
  const rels: NormErd["rels"] = [];
  const seen = new Set<string>();
  const selfRefs = new Set<number>();
  for (const raw of Array.isArray(relationsIn) ? relationsIn : []) {
    const r = (raw ?? {}) as RelationIn;
    const a = ids.get(String(r.from));
    const b = ids.get(String(r.to));
    if (a === undefined || b === undefined) {
      warnings.push(`relation ${String(r.from)} -> ${String(r.to)} names an unknown entity and was skipped`);
      continue;
    }
    let kind: RelKind = "1:N";
    if (r.kind != null && r.kind !== "") {
      if ((KINDS as string[]).includes(String(r.kind))) kind = r.kind as RelKind;
      else warnings.push(`unknown relation kind "${r.kind}" treated as 1:N`);
    }
    const label = r.label ? oneLine(r.label) : undefined;
    if (a === b) {
      selfRefs.add(a);
      warnings.push(`self relation on "${entities[a].name}" shown as a marker`);
      continue;
    }
    const key = `${a}>${b}>${kind}>${label ?? ""}`;
    if (seen.has(key)) continue;
    seen.add(key);
    rels.push({ a, b, kind, label });
  }
  return { entities, rels, selfRefs, warnings };
}

function tableSpec(e: NormErd["entities"][number], fieldCap: number, selfRef: boolean): LNodeSpec {
  const title = (selfRef ? "∞ " : "") + e.name;
  const rows = e.fields.map((f) => ({ name: f.name, type: f.type, flags: f.flags }));
  const nw = Math.max(0, ...rows.map((r) => textWidth(r.name)));
  const tw = Math.max(0, ...rows.map((r) => textWidth(r.type)));
  const fw = Math.max(0, ...rows.map((r) => textWidth(r.flags)));
  const rowW = (r: { name: string; type: string; flags: string }) =>
    nw + (tw ? 2 + tw : 0) + (fw ? 2 + fw : 0) + 0 * textWidth(r.name);
  const bodyW = rows.length ? rowW(rows[0]) : 0;
  const inner = Math.min(Math.max(textWidth(title), bodyW), Math.max(fieldCap, textWidth(title)));
  const w = inner + 4;
  const h = rows.length ? 4 + rows.length : 3;
  return {
    id: e.name,
    label: e.name,
    status: null,
    baseW: w,
    baseH: h,
    noPortRows: rows.length ? [2] : [],
    paint: (c: Canvas, r: Rect) => {
      paintBox(c, r, "box.erd", false);
      c.text(r.x + 2, r.y + 1, truncateText(title, r.w - 4), "erd.title");
      if (!rows.length) return;
      c.set(r.x, r.y + 2, "├", "box.erd");
      c.set(r.x + r.w - 1, r.y + 2, "┤", "box.erd");
      for (let x = r.x + 1; x < r.x + r.w - 1; x++) c.set(x, r.y + 2, "─", "box.erd");
      rows.forEach((row, i) => {
        const y = r.y + 3 + i;
        const avail = r.w - 4;
        let x = r.x + 2;
        const put = (s: string, role: string, width: number) => {
          const t = truncateText(s, Math.max(0, avail - (x - (r.x + 2))));
          c.text(x, y, t, role);
          x += width + 2;
        };
        put(row.name, /\bPK\b/.test(row.flags) ? "erd.pk" : "erd.field", nw);
        if (tw) put(row.type, "erd.type", tw);
        if (fw) put(row.flags, /\bPK\b|\bFK\b/.test(row.flags) ? "erd.key" : "erd.flag", fw);
      });
    },
  };
}

const CAPS = [60, 40, 30, 22, 16];

export function erd(entitiesIn: unknown, relationsIn: unknown, opts: ErdOptions = {}): DiagramResult {
  return budgeted(() => erdLayout(entitiesIn, relationsIn, opts));
}

function erdLayout(entitiesIn: unknown, relationsIn: unknown, opts: ErdOptions): DiagramResult {
  const norm = normalizeErd(entitiesIn, relationsIn);
  const maxWidth = Math.max(20, opts.maxWidth ?? 100);
  if (!norm.entities.length) {
    return {
      kind: "erd",
      direction: "LR",
      width: 0,
      height: 0,
      rows: [],
      nodes: [],
      edges: [],
      warnings: [...norm.warnings, "the diagram has no entities"],
      footnotes: [],
      degraded: false,
      overflow: false,
      statuses: [],
      legend: [],
    };
  }
  const edges: LEdgeSpec[] = norm.rels.map((r) => {
    const m = endMarks(r.kind);
    return { a: r.a, b: r.b, label: r.label, fromMark: m.from, toMark: m.to };
  });
  const dirs: Array<"LR" | "TB"> = opts.direction === "LR" || opts.direction === "TB" ? [opts.direction] : ["LR", "TB"];
  let best: DiagramResult | null = null;
  let chosen: DiagramResult | null = null;
  for (const cap of CAPS) {
    const nodes = norm.entities.map((e, i) => tableSpec(e, cap, norm.selfRefs.has(i)));
    for (const d of dirs) {
      const r = layeredDiagram("erd", { nodes, edges, portSpacing: 4 }, d);
      if (!best || r.width < best.width) best = r;
      if (r.width <= maxWidth) {
        chosen = r;
        break;
      }
    }
    if (chosen) break;
  }
  if (!chosen) {
    const nodes = norm.entities.map((e, i) => tableSpec(e, 30, norm.selfRefs.has(i)));
    const tries = dirs.map((d) => layeredDiagram("erd", { nodes, edges, portSpacing: 4 }, d));
    best = tries.reduce((a, b) => (b.width < a.width ? b : a));
  }
  const res = chosen ?? best!;
  res.overflow = !chosen;
  res.warnings = [...norm.warnings];
  if (res.degraded) res.warnings.push("some relations could not be routed cleanly");
  return res;
}

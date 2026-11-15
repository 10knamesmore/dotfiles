// Component signatures, validation and normalisation of OpenUI Lang programs.
// One table drives: argument checking, the typed element tree the renderers read, and the reference text for the model.

import { markdownImages } from "./md.ts";
import { parseProgram, type Expr } from "./parse.ts";

export type T =
  | "string"
  | "number"
  | "boolean"
  | "any"
  | "action"
  | "cell"
  | "child"
  | { enum: readonly string[] }
  | { arr: T; min?: number; max?: number }
  | { comp: readonly string[] };

export interface Param {
  name: string;
  t: T;
  opt?: boolean;
  /** most characters in this string argument (the default is LIMITS.string) */
  max?: number;
}
export interface CompSpec {
  name: string;
  params: Param[];
  desc: string;
  /** only valid inside a parent (its own argument slot) */
  helper?: boolean;
}

/**
 * Size limits. A widget is drawn again whenever pi redraws, and a model can write far more than a terminal can show, so each
 * limit is a validation error with a line number: the model fixes it and nothing oversized is ever laid out. Sized for a
 * terminal of about 100 columns; the stress tests (test/limits.test.ts) draw every component at its limit.
 */
export const LIMITS = {
  /** characters of OpenUI Lang in one widget */
  source: 100_000,
  /** errors reported at once */
  errors: 25,
  /** components nested inside each other */
  depth: 16,
  /** components in one widget */
  elements: 1_500,
  /** characters in one string, unless the argument says otherwise */
  string: 2_000,
  /** characters in one table cell */
  cell: 300,
  text: 8_000,
  markdown: 30_000,
  code: 30_000,
  codeLines: 400,
  answer: 4_000,
  /** an Image src: a path, or a small data: URI. Larger pictures belong in a file */
  src: 100_000,
  children: 40,
  tabs: 12,
  tableRows: 100,
  tableCols: 16,
  bars: 40,
  points: 200,
  series: 8,
  axes: 16,
  dagNodes: 60,
  dagEdges: 150,
  entities: 20,
  fields: 30,
  relations: 40,
  zones: 8,
  zoneNodes: 12,
  /** nodes and links in one InfraMap: every link is a route search, and the cost climbs steeply past about 30 nodes */
  infraNodes: 30,
  infraLinks: 45,
  /** links on a GeoMap */
  links: 60,
  geoPoints: 100,
  faq: 30,
  steps: 100,
  /** pictures in one widget: Image components and `![alt](path)` lines together. Each can be megabytes */
  images: 12,
  /** Choice questions in one widget: they share one Send answers bar and one message */
  choices: 12,
  /** options of one Choice (the lower bound is 2) */
  choiceOptions: 6,
  /** characters in a Choice question and in one of its options (the message carries them as written) */
  question: 200,
  option: 120,
  /** characters of the user's own Other answer */
  other: 500,
} as const;

const S: T = "string";
const strs: T = { arr: S };
const kids: T = { arr: "child", max: LIMITS.children };
const tips: T = { arr: { comp: ["Tooltip"] } };
const p = (name: string, t: T, opt = false, max?: number): Param => ({ name, t, opt, ...(max ? { max } : {}) });
const e = (...v: string[]): T => ({ enum: v });

export const SPECS: CompSpec[] = [
  {
    name: "Stack",
    params: [
      p("children", kids),
      p("direction", e("row", "column"), true),
      p("gap", e("none", "xs", "s", "m", "l", "xl"), true),
      p("wrap", "boolean", true),
    ],
    desc: 'Layout container. direction "column" (default) or "row". gap none|xs|s|m|l|xl (default m). wrap lets a row wrap.',
  },
  {
    name: "Text",
    params: [p("text", S, false, LIMITS.text), p("variant", e("title", "heading", "body", "muted", "code"), true)],
    desc: "A line or paragraph of text (default body).",
  },
  { name: "Card", params: [p("children", kids), p("title", S, true)], desc: "Bordered panel with an optional title." },
  {
    name: "Metric",
    params: [
      p("label", S),
      p("value", S),
      p("delta", S, true),
      p("tone", e("good", "warn", "bad", "neutral"), true),
      p("tooltips", tips, true),
    ],
    desc: "KPI tile. value and delta are strings. tone colours the delta.",
  },
  {
    name: "Table",
    params: [
      p("columns", { arr: S, max: LIMITS.tableCols }),
      p("rows", { arr: { arr: "cell", max: LIMITS.tableCols }, max: LIMITS.tableRows }),
      p("tooltips", tips, true),
    ],
    desc: "Data table. rows is an array of rows, each the same length as columns (strings or numbers). Click headers to sort, cells to view/copy their full value, or copy CSV to export the displayed row order. Tooltip row numbers identify the original data rows even after sorting.",
  },
  {
    name: "BarChart",
    params: [
      p("labels", { arr: S, max: LIMITS.bars }),
      p("values", { arr: "number", max: LIMITS.bars }),
      p("title", S, true),
      p("unit", S, true),
      p("tooltips", tips, true),
    ],
    desc: "Bar chart of one series. labels and values are parallel arrays. unit is a suffix such as %.",
  },
  {
    name: "Callout",
    params: [p("text", S), p("tone", e("info", "success", "warn", "error"), true)],
    desc: "Highlighted note.",
  },
  {
    name: "Button",
    params: [p("label", S), p("action", "action", true)],
    desc: "Clickable button. Without an action it sends its label to the assistant.",
  },
  {
    name: "TabItem",
    helper: true,
    params: [p("label", S), p("content", kids)],
    desc: "One tab. Only valid inside Tabs.",
  },
  {
    name: "Tabs",
    params: [p("items", { arr: { comp: ["TabItem"] }, min: 1, max: LIMITS.tabs })],
    desc: "Tabbed container.",
  },
  {
    name: "Markdown",
    params: [p("text", S, false, LIMITS.markdown)],
    desc: 'Rich text from markdown: headings, **bold**, *italic*, `code`, lists, > quotes, tables, fenced code, ---. A line that is only ![alt](path "caption") draws a picture (a local file or data: URI, as for Image).',
  },
  {
    name: "CodeBlock",
    params: [p("code", S, false, LIMITS.code), p("language", S, true), p("title", S, true)],
    desc: "Syntax highlighted code. language: python|js|ts|bash|sql|yaml|json|go|rust (others unhighlighted). Click copies it.",
  },
  {
    name: "Image",
    params: [p("src", S, false, LIMITS.src), p("alt", S, true), p("caption", S, true), p("tooltips", tips, true)],
    desc: "An image rendered through the Kitty graphics protocol. Other terminals show a labelled frame with the file name and size.",
  },
  {
    name: "DagNode",
    helper: true,
    params: [p("id", S), p("label", S), p("status", e("done", "running", "failed", "pending", "skipped"), true)],
    desc: "One DAG node.",
  },
  {
    name: "Dag",
    params: [
      p("nodes", { arr: { comp: ["DagNode"] }, min: 1, max: LIMITS.dagNodes }),
      p("edges", { arr: { arr: S }, max: LIMITS.dagEdges }),
      p("direction", e("LR", "TB"), true),
      p("title", S, true),
      p("tooltips", tips, true),
    ],
    desc: "Directed graph with automatic layered layout. edges are [fromId, toId] pairs. direction omitted = automatic.",
  },
  {
    name: "Entity",
    helper: true,
    params: [p("name", S), p("fields", { arr: S, max: LIMITS.fields })],
    desc: 'ERD table. fields are strings like "id uuid PK", "user_id uuid FK", "email text UNIQUE".',
  },
  {
    name: "Relation",
    helper: true,
    params: [p("from", S), p("to", S), p("kind", e("1:1", "1:N", "N:1", "N:M"), true), p("label", S, true)],
    desc: "ERD relation between two entity names. 1:N (default): the first entity is the one side.",
  },
  {
    name: "Erd",
    params: [
      p("entities", { arr: { comp: ["Entity"] }, min: 1, max: LIMITS.entities }),
      p("relations", { arr: { comp: ["Relation"] }, max: LIMITS.relations }),
      p("title", S, true),
      p("tooltips", tips, true),
    ],
    desc: "Entity-relationship diagram with crow's-foot relations.",
  },
  {
    name: "GeoPoint",
    helper: true,
    params: [
      p("label", S),
      p("lat", "number"),
      p("lon", "number"),
      p("tone", e("good", "warn", "bad", "info", "neutral"), true),
    ],
    desc: "Map pin in decimal degrees.",
  },
  {
    name: "GeoMap",
    params: [
      p("points", { arr: { comp: ["GeoPoint"] }, min: 1, max: LIMITS.geoPoints }),
      p("links", { arr: { arr: S }, max: LIMITS.links }, true),
      p("title", S, true),
      p("tooltips", tips, true),
    ],
    desc: "Offline world map (coarse coastlines) zoomed to fit the pins. links are [labelA, labelB] pairs drawn dashed.",
  },
  {
    name: "FaqItem",
    helper: true,
    params: [p("question", S), p("answer", S, false, LIMITS.answer)],
    desc: "One Q&A entry (answer is markdown). Only valid inside Faq.",
  },
  {
    name: "Faq",
    params: [p("items", { arr: { comp: ["FaqItem"] }, min: 1, max: LIMITS.faq }), p("firstOpen", "boolean", true)],
    desc: "Collapsible Q&A list.",
  },
  {
    name: "Choice",
    params: [
      p("question", S, false, LIMITS.question),
      p("options", { arr: S, min: 2, max: LIMITS.choiceOptions }),
      p("hint", S, true),
      p("multiple", "boolean", true),
      p("other", "boolean", true),
    ],
    desc: "A question for the USER. Answers are staged and sent together as ONE message when the user presses Send answers. multiple (4th argument) true allows several options. other (5th argument, default true) adds an Other... row where the user types their own answer; false hides it.",
  },
  {
    name: "Series",
    helper: true,
    params: [p("category", S), p("values", { arr: "number", max: LIMITS.points })],
    desc: "One data series: name and numbers parallel to the chart labels.",
  },
  {
    name: "LineChart",
    params: [
      p("labels", { arr: S, max: LIMITS.points }),
      p("series", { arr: { comp: ["Series"] }, min: 1, max: LIMITS.series }),
      p("variant", e("linear", "natural", "step"), true),
      p("xLabel", S, true),
      p("yLabel", S, true),
      p("height", "number", true),
      p("tooltips", tips, true),
    ],
    desc: "Line chart over categories, several series. variant natural (smooth, default), linear or step.",
  },
  {
    name: "AreaChart",
    params: [
      p("labels", { arr: S, max: LIMITS.points }),
      p("series", { arr: { comp: ["Series"] }, min: 1, max: LIMITS.series }),
      p("variant", e("linear", "natural", "step"), true),
      p("xLabel", S, true),
      p("yLabel", S, true),
      p("height", "number", true),
      p("tooltips", tips, true),
    ],
    desc: "Filled STACKED area chart: series stack into a growing total.",
  },
  {
    name: "PieChart",
    params: [
      p("labels", strs),
      p("values", { arr: "number" }),
      p("variant", e("pie", "donut"), true),
      p("appearance", e("circular", "semiCircular"), true),
      p("tooltips", tips, true),
    ],
    desc: "Parts of a whole, at most 8 slices. Values are raw numbers, not percentages.",
  },
  {
    name: "RadarChart",
    params: [
      p("labels", { arr: S, min: 3, max: LIMITS.axes }),
      p("series", { arr: { comp: ["Series"] }, min: 1, max: LIMITS.series }),
      p("tooltips", tips, true),
    ],
    desc: "Radar chart: one axis per label, each series parallel to labels.",
  },
  {
    name: "InfraNode",
    helper: true,
    params: [
      p("id", S),
      p("label", S),
      p("kind", e("client", "gateway", "service", "gpu", "database", "storage", "queue", "cloud"), true),
      p("status", e("up", "degraded", "down", "unknown"), true),
      p("detail", S, true),
    ],
    desc: "One box inside an InfraZone. id is unique across the map.",
  },
  {
    name: "InfraZone",
    helper: true,
    params: [
      p("name", S),
      p("nodes", { arr: { comp: ["InfraNode"] }, min: 1, max: LIMITS.zoneNodes }),
      p("tone", e("neutral", "info", "good", "warn", "bad"), true),
    ],
    desc: "A labelled box that groups nodes.",
  },
  {
    name: "InfraLink",
    helper: true,
    params: [p("from", S), p("to", S), p("label", S, true), p("style", e("solid", "dashed"), true)],
    desc: "An arrow between two InfraNode ids.",
  },
  {
    name: "InfraMap",
    params: [
      p("zones", { arr: { comp: ["InfraZone"] }, min: 1, max: LIMITS.zones }),
      p("links", { arr: { comp: ["InfraLink"] }, max: LIMITS.infraLinks }),
      p("title", S, true),
      p("direction", e("LR", "TB"), true),
      p("tooltips", tips, true),
    ],
    desc: "Infrastructure map: zones are columns (LR, default) or rows (TB).",
  },
  {
    name: "SeqParticipant",
    helper: true,
    params: [p("id", S), p("label", S), p("kind", e("actor", "service", "database", "queue", "external"), true)],
    desc: "One column of a SequenceDiagram.",
  },
  {
    name: "SeqMessage",
    helper: true,
    params: [p("from", S), p("to", S), p("label", S), p("style", e("sync", "async", "return"), true), p("id", S, true)],
    desc: "An arrow between participant ids. from == to is a self-call.",
  },
  {
    name: "SeqNote",
    helper: true,
    params: [p("over", S), p("text", S), p("to", S, true)],
    desc: "A note over a participant, or spanning to another.",
  },
  {
    name: "SeqGroup",
    helper: true,
    params: [
      p("kind", e("alt", "opt", "loop", "par")),
      p("label", S),
      p("steps", { arr: { comp: ["SeqMessage", "SeqNote", "SeqGroup"] }, max: LIMITS.steps }),
      p("elseSteps", { arr: { comp: ["SeqMessage", "SeqNote", "SeqGroup"] }, max: LIMITS.steps }, true),
      p("elseLabel", S, true),
    ],
    desc: "A frame around steps. elseSteps adds a dashed divider and a second branch.",
  },
  {
    name: "SequenceDiagram",
    params: [
      p("participants", { arr: { comp: ["SeqParticipant"] }, min: 1, max: 10 }),
      p("steps", { arr: { comp: ["SeqMessage", "SeqNote", "SeqGroup"] }, max: LIMITS.steps }),
      p("title", S, true),
      p("numbered", "boolean", true),
      p("tooltips", tips, true),
    ],
    desc: "Calls between participants: columns left to right, steps top to bottom.",
  },
  {
    name: "Tooltip",
    helper: true,
    params: [
      p("target", S),
      p("hint", S, true),
      p("detail", S, true),
      p("code", S, true),
      p("language", S, true),
      p("tone", e("info", "good", "warn", "bad"), true),
      p("link", S, true),
      p("linkLabel", S, true),
    ],
    desc: "Hover hint with an optional click-to-open panel for one element of the host component.",
  },
];

export const LIB = new Map(SPECS.map((s) => [s.name, s]));
export const TOP_LEVEL = SPECS.filter((s) => !s.helper).map((s) => s.name);

/** What each Tooltip host accepts as `target`. Kept in one place so validation, rendering and the reference agree. */
export const TARGET_HELP: Record<string, string> = {
  Dag: "Use a DagNode id.",
  Erd: "Use an Entity name or entity.column.",
  InfraMap: "Use an InfraNode id, zone:<zone name>, or <fromId>><toId> for a link.",
  SequenceDiagram: "Use a SeqParticipant id or a SeqMessage id (a message needs an id to carry a tooltip).",
  Table: "Use row:N (1-based), col:<header> or cell:N.<header>.",
  Metric: "Use the metric's own label.",
  BarChart: "Use a bar label.",
  GeoMap: "Use a GeoPoint label, or A>B for a link.",
  Image: "Use the word image.",
  LineChart: "Use an x label (a category) or series:<name>.",
  AreaChart: "Use an x label (a category) or series:<name>.",
  PieChart: "Use a slice label.",
  RadarChart: "Use an axis label or series:<name>.",
};

export function tipTargets(el: El): string[] {
  const pr = el.props;
  const names = (xs: El[] | undefined, key: string): string[] =>
    (xs ?? []).map((x) => String(x.props?.[key] ?? "")).filter(Boolean);
  switch (el.type) {
    case "Dag":
      return names(pr.nodes, "id");
    case "Erd":
      return (pr.entities ?? []).flatMap((e: El) => [
        String(e.props.name),
        ...((e.props.fields ?? []) as string[]).map((f) => `${e.props.name}.${String(f).trim().split(/\s+/)[0]}`),
      ]);
    case "InfraMap":
      return [
        ...(pr.zones ?? []).flatMap((z: El) => [`zone:${z.props.name}`, ...names(z.props.nodes, "id")]),
        ...(pr.links ?? []).map((l: El) => `${l.props.from}>${l.props.to}`),
      ];
    case "SequenceDiagram": {
      const ids = names(pr.participants, "id");
      const walk = (steps: El[] | undefined) => {
        for (const s of steps ?? []) {
          if (s.type === "SeqMessage" && s.props.id) ids.push(String(s.props.id));
          else if (s.type === "SeqGroup") {
            walk(s.props.steps);
            walk(s.props.elseSteps);
          }
        }
      };
      walk(pr.steps);
      return ids;
    }
    case "Table": {
      const cols = ((pr.columns ?? []) as string[]).map(String);
      const rows = (pr.rows ?? []) as unknown[][];
      return [
        ...rows.map((_, i) => `row:${i + 1}`),
        ...cols.map((c) => `col:${c}`),
        ...rows.flatMap((_, i) => cols.map((c) => `cell:${i + 1}.${c}`)),
      ];
    }
    case "Metric":
      return [String(pr.label ?? "")];
    case "BarChart":
    case "PieChart":
      return ((pr.labels ?? []) as string[]).map(String);
    case "GeoMap":
      return [...names(pr.points, "label"), ...(pr.links ?? []).map((l: string[]) => `${l[0]}>${l[1]}`)];
    case "Image":
      return ["image"];
    case "LineChart":
    case "AreaChart":
    case "RadarChart":
      return [
        ...((pr.labels ?? []) as string[]).map(String),
        ...names(pr.series, "category").map((c) => `series:${c}`),
      ];
  }
  return [];
}

export interface El {
  type: string;
  props: Record<string, any>;
  line: number;
  /** The variable this element was defined under (`tbl` in `tbl = Table(...)`). Absent for an element written inline. */
  name?: string;
}
export interface Action {
  steps: Array<{ name: string; args: any[] }>;
}

export interface Compiled {
  ok: boolean;
  errors: string[];
  warnings: string[];
  root?: El;
}

export function showType(t: T): string {
  if (typeof t === "string") return t === "child" ? "component" : t === "cell" ? "string | number" : t;
  if ("enum" in t) return t.enum.map((x) => JSON.stringify(x)).join(" | ");
  if ("arr" in t) {
    const inner = showType(t.arr);
    return (inner.includes(" ") && !inner.startsWith("Tooltip") ? `(${inner})` : inner) + "[]";
  }
  return t.comp.join(" | ");
}

/** Reads and checks a program. Never throws: whatever goes wrong comes back as an error the model can read. */
export function compile(source: string): Compiled {
  try {
    const text = String(source ?? "");
    if (text.length > LIMITS.source)
      return {
        ok: false,
        errors: [
          `the source is ${Math.round(text.length / 1000)} KB; the limit is ${LIMITS.source / 1000} KB. Show less data, or split it into several widgets`,
        ],
        warnings: [],
      };
    const c = compileProgram(text);
    if (c.errors.length > LIMITS.errors) {
      const more = c.errors.length - LIMITS.errors;
      return {
        ...c,
        errors: [
          ...c.errors.slice(0, LIMITS.errors),
          `... and ${more} more error${more === 1 ? "" : "s"}; fix these first`,
        ],
        root: undefined,
        ok: false,
      };
    }
    return c;
  } catch (e) {
    return {
      ok: false,
      errors: [`the widget could not be read: ${(e as Error).message}. Make it smaller and less nested`],
      warnings: [],
    };
  }
}

function compileProgram(source: string): Compiled {
  const prog = parseProgram(source);
  const errors = [...prog.errors];
  const warnings: string[] = [];
  if (!prog.statements.has("root")) {
    errors.push("define the entry point: root = Stack([...]) (put it on the first line)");
    return { ok: false, errors, warnings };
  }
  if (prog.order[0] !== "root")
    warnings.push("root is not the first statement; put it first so the shell appears immediately");

  // the variable each call was defined under, so a note can say `tbl` instead of "a table"
  const nameOf = new Map<Expr, string>();
  for (const [stmtName, st] of prog.statements) if (st.expr.k === "call") nameOf.set(st.expr, stmtName);
  const used = new Set<string>(["root"]);
  const visiting: string[] = [];
  const where = (line: number) => `line ${line}: `;

  const deref = (x: Expr): Expr | null => {
    let cur = x;
    const chain: string[] = [];
    while (cur.k === "ref") {
      const st = prog.statements.get(cur.name);
      if (!st) {
        errors.push(`${where(cur.line)}"${cur.name}" is used but never defined`);
        return null;
      }
      if (chain.includes(cur.name) || visiting.includes(cur.name)) {
        errors.push(`${where(cur.line)}"${cur.name}" refers to itself (a cycle)`);
        return null;
      }
      chain.push(cur.name);
      used.add(cur.name);
      cur = st.expr;
    }
    return cur;
  };
  // a reference to a component: track it while building so a cycle is detected
  const withVisit = <R>(x: Expr, f: () => R): R => {
    if (x.k === "ref") visiting.push(x.name);
    try {
      return f();
    } finally {
      if (x.k === "ref") visiting.pop();
    }
  };

  const label = (x: Expr) =>
    x.k === "call"
      ? x.name
      : x.k === "str"
        ? "a string"
        : x.k === "num"
          ? "a number"
          : x.k === "bool"
            ? "a boolean"
            : x.k === "arr"
              ? "an array"
              : x.k === "obj"
                ? "an object"
                : x.k === "null"
                  ? "null"
                  : x.k === "action"
                    ? `@${x.name}`
                    : "a reference";

  const plain = (x: Expr): any => {
    const d = deref(x);
    if (!d) return null;
    switch (d.k) {
      case "str":
      case "num":
      case "bool":
        return d.v;
      case "null":
        return null;
      case "arr":
        return d.items.map(plain);
      case "obj":
        return Object.fromEntries(d.entries.map(([k, v]) => [k, plain(v)]));
      default:
        return null;
    }
  };

  const tooLong = (v: string, max: number, ctx: string, line: number) => {
    if (v.length > max) errors.push(`${where(line)}${ctx} is ${v.length} characters; the limit is ${max}. Shorten it`);
  };
  const resolve = (x: Expr, t: T, ctx: string, maxChars: number = LIMITS.string): any => {
    const d0 = x;
    const d = deref(x);
    if (!d) return undefined;
    return withVisit(d0, () => {
      const bad = (want: string) => {
        errors.push(`${where(d.line)}${ctx} expects ${want} but got ${label(d)}`);
        return undefined;
      };
      if (typeof t === "string") {
        switch (t) {
          case "string":
            if (d.k !== "str") return bad("a string");
            tooLong(d.v, maxChars, ctx, d.line);
            return d.v;
          case "number":
            return d.k === "num" ? d.v : bad("a number");
          case "boolean":
            return d.k === "bool" ? d.v : bad("true or false");
          case "cell":
            if (d.k !== "str" && d.k !== "num") return bad("a string or a number");
            if (d.k === "str") tooLong(d.v, LIMITS.cell, ctx, d.line);
            return d.v;
          case "any":
            return plain(d);
          case "action": {
            const steps: Action["steps"] = [];
            const add = (a: Expr) => {
              const da = deref(a);
              if (!da) return;
              if (da.k === "action") {
                if (da.name === "ToAssistant") {
                  const msg = da.args[0] ? plain(da.args[0]) : undefined;
                  if (typeof msg !== "string" || !msg)
                    errors.push(`${where(da.line)}@ToAssistant needs a message string`);
                  else {
                    if (msg.length > 500)
                      errors.push(
                        `${where(da.line)}@ToAssistant messages are limited to 500 characters (this one has ${msg.length})`,
                      );
                    steps.push({ name: "ToAssistant", args: [msg, da.args[1] ? plain(da.args[1]) : undefined] });
                  }
                } else if (da.name === "OpenUrl") {
                  const url = da.args[0] ? plain(da.args[0]) : undefined;
                  if (typeof url !== "string") errors.push(`${where(da.line)}@OpenUrl needs a url string`);
                  else steps.push({ name: "OpenUrl", args: [url] });
                } else errors.push(`${where(da.line)}unknown action step @${da.name} (use @ToAssistant or @OpenUrl)`);
              } else if (da.k === "call" && da.name === "Action") {
                const inner = da.args[0] ? deref(da.args[0]) : null;
                if (!inner || inner.k !== "arr")
                  errors.push(`${where(da.line)}Action needs an array of steps: Action([@ToAssistant("...")])`);
                else inner.items.forEach(add);
              } else
                errors.push(`${where(da.line)}an action must be @ToAssistant("..."), @OpenUrl("...") or Action([...])`);
            };
            add(d);
            return { steps } satisfies Action;
          }
          case "child": {
            if (d.k !== "call") return bad("a component, such as Text(...) or Card(...)");
            const spec = LIB.get(d.name);
            if (spec?.helper) {
              errors.push(
                `${where(d.line)}${d.name} is not a standalone component: it only belongs inside its parent (${helperParent(d.name)}). Put the layout component there instead`,
              );
              return undefined;
            }
            return build(d);
          }
        }
      }
      if ("enum" in t) {
        if (d.k !== "str") return bad(`one of ${t.enum.map((v) => JSON.stringify(v)).join(", ")}`);
        if (!t.enum.includes(d.v)) {
          errors.push(
            `${where(d.line)}${ctx} must be one of ${t.enum.map((v) => JSON.stringify(v)).join(", ")}, not ${JSON.stringify(d.v)}`,
          );
          return undefined;
        }
        return d.v;
      }
      if ("arr" in t) {
        if (d.k !== "arr") return bad("an array [...]");
        const out = d.items.map((it, i) => resolve(it, t.arr, `${ctx}[${i + 1}]`)).filter((v) => v !== undefined);
        if (t.min !== undefined && d.items.length < t.min)
          errors.push(`${where(d.line)}${ctx} needs at least ${t.min} item(s)`);
        if (t.max !== undefined && d.items.length > t.max)
          errors.push(`${where(d.line)}${ctx} allows at most ${t.max} item(s), got ${d.items.length}`);
        return out;
      }
      // { comp: [...] }
      if (d.k !== "call") return bad(t.comp.join(" or "));
      if (!t.comp.includes(d.name)) {
        errors.push(`${where(d.line)}${ctx} expects ${t.comp.join(" or ")} but got ${d.name}`);
        return undefined;
      }
      return build(d);
    });
  };

  const helperParent = (name: string): string => {
    const parents = SPECS.filter((s) => s.params.some((q) => JSON.stringify(q.t).includes(`"${name}"`))).map(
      (s) => s.name,
    );
    return parents.join(" or ") || "its parent";
  };

  let depth = 0;
  let built = 0;
  let pictures = 0;
  let choices = 0;
  const build = (call: Extract<Expr, { k: "call" }>): El | undefined => {
    const spec = LIB.get(call.name);
    if (!spec) {
      errors.push(`${where(call.line)}unknown component "${call.name}". Components: ${TOP_LEVEL.join(", ")}`);
      return undefined;
    }
    if (depth >= LIMITS.depth) {
      errors.push(
        `${where(call.line)}${spec.name} is nested more than ${LIMITS.depth} components deep. Flatten the layout`,
      );
      return undefined;
    }
    if (++built === LIMITS.elements + 1)
      errors.push(
        `${where(call.line)}the widget has more than ${LIMITS.elements} components. Show less, or split it into several widgets`,
      );
    if (built > LIMITS.elements) return undefined;
    depth++;
    try {
      return buildInner(call, spec);
    } finally {
      depth--;
    }
  };
  const buildInner = (call: Extract<Expr, { k: "call" }>, spec: CompSpec): El | undefined => {
    if (call.args.length > spec.params.length)
      errors.push(
        `${where(call.line)}${spec.name} takes at most ${spec.params.length} argument(s), got ${call.args.length}`,
      );
    const props: Record<string, any> = {};
    spec.params.forEach((param, i) => {
      const arg = call.args[i];
      const d = arg ? deref(arg) : null;
      if (!arg || (d && d.k === "null")) {
        if (!param.opt)
          errors.push(
            `${where(call.line)}${spec.name} is missing the required argument "${param.name}" (position ${i + 1})`,
          );
        return;
      }
      if (spec.name === "Choice" && param.name === "hint" && d && d.k === "bool") {
        errors.push(
          `${where(d.line)}Choice argument "hint" expects a string but got ${d.v}. hint is the third argument: write null for a skipped optional argument, e.g. Choice("Which checks?", ["Disk", "Memory"], null, true)`,
        );
        return;
      }
      if (spec.name === "Choice" && (param.name === "multiple" || param.name === "other") && d && d.k !== "bool") {
        errors.push(
          `${where(d.line)}Choice argument "${param.name}" expects true or false but got ${label(d)}. It is the ${i + 1}th argument: write null for each skipped optional argument, e.g. Choice("Which checks?", ["Disk", "Memory"], null, true)`
            .replace("1th", "1st")
            .replace("2th", "2nd")
            .replace("3th", "3rd"),
        );
        return;
      }
      props[param.name] = resolve(arg, param.t, `${spec.name} argument "${param.name}"`, param.max);
    });
    const el: El = { type: spec.name, props, line: call.line, ...(nameOf.has(call) ? { name: nameOf.get(call) } : {}) };
    crossCheck(el);
    return el;
  };

  const checkTips = (el: El) => {
    const tipsList = (el.props.tooltips ?? []) as El[];
    if (!tipsList.length) return;
    const valid = tipTargets(el);
    const err = (line: number, m: string) => errors.push(`${where(line)}${el.type}: ${m}`);
    const seenTargets = new Set<string>();
    for (const t of tipsList) {
      const pr = t.props;
      const target = String(pr.target ?? "");
      if (!valid.includes(target)) {
        const shown = valid.slice(0, 14).join(", ") + (valid.length > 14 ? ", ..." : "");
        err(
          t.line,
          `Tooltip target "${target}" matches nothing here. ${TARGET_HELP[el.type] ?? ""} Valid targets: ${shown || "(none)"}`.replace(
            "  ",
            " ",
          ),
        );
      }
      if (seenTargets.has(target)) err(t.line, `two Tooltips aim at "${target}"; use one`);
      seenTargets.add(target);
      if (typeof pr.hint === "string" && pr.hint.length > 140)
        err(t.line, `Tooltip hint is ${pr.hint.length} characters; keep it to 140 or move the rest into detail`);
      if (typeof pr.detail === "string" && pr.detail.length > 800)
        err(t.line, `Tooltip detail is ${pr.detail.length} characters; keep it to 800`);
      if (typeof pr.code === "string" && pr.code.split("\n").length > 40)
        err(t.line, "Tooltip code has more than 40 lines");
      if (typeof pr.link === "string" && !/^https?:\/\//i.test(pr.link))
        err(t.line, "Tooltip link must start with http:// or https://");
    }
    if (tipsList.length > 12)
      errors.push(`${where(el.line)}${el.type}: ${tipsList.length} Tooltips is a lot; use about 8 at most`);
  };

  const crossCheck = (el: El) => {
    const pr = el.props;
    const err = (m: string) => errors.push(`${where(el.line)}${el.type}: ${m}`);
    checkTips(el);
    // pictures: an Image, or lines of `![alt](path)` in a Markdown or an FAQ answer
    const mdText = el.type === "Markdown" ? pr.text : el.type === "FaqItem" ? pr.answer : undefined;
    const here = el.type === "Image" ? 1 : mdText !== undefined ? markdownImages(mdText).length : 0;
    if (here) {
      const before = pictures;
      pictures += here;
      if (before <= LIMITS.images && pictures > LIMITS.images)
        err(
          `this is picture ${LIMITS.images + 1}: a widget has at most ${LIMITS.images} (Image components and ![alt](path) lines together). Show fewer`,
        );
    }
    const dupes = (xs: string[]) => xs.filter((x, i) => xs.indexOf(x) !== i);
    switch (el.type) {
      case "Table":
        if (Array.isArray(pr.columns) && Array.isArray(pr.rows))
          pr.rows.forEach((r: any[], i: number) => {
            if (r.length !== pr.columns.length)
              err(`row ${i + 1} has ${r.length} cells but there are ${pr.columns.length} columns`);
          });
        break;
      case "BarChart":
      case "PieChart":
        if (Array.isArray(pr.labels) && Array.isArray(pr.values) && pr.labels.length !== pr.values.length)
          err(`labels (${pr.labels.length}) and values (${pr.values.length}) must be the same length`);
        if (el.type === "PieChart" && Array.isArray(pr.values)) {
          if (pr.values.length > 8) err("use at most 8 slices");
          if (pr.values.some((v: number) => v < 0)) err("values must not be negative");
        }
        break;
      case "LineChart":
      case "AreaChart":
      case "RadarChart":
        if (Array.isArray(pr.labels) && Array.isArray(pr.series))
          for (const s of pr.series as El[])
            if (s?.props?.values && s.props.values.length !== pr.labels.length)
              err(
                `series "${s.props.category}" has ${s.props.values.length} values but there are ${pr.labels.length} labels`,
              );
        break;
      case "Dag": {
        const ids: string[] = (pr.nodes ?? []).map((n: El) => n.props.id);
        for (const d of dupes(ids)) err(`duplicate node id "${d}"`);
        for (const ed of pr.edges ?? []) {
          if (ed.length !== 2) err(`an edge must be [fromId, toId], got ${ed.length} item(s)`);
          else
            for (const id of ed)
              if (!ids.includes(id))
                err(
                  `edge refers to "${id}", which is not a DagNode id. Use the id strings, not variable names. Ids: ${ids.join(", ")}`,
                );
        }
        break;
      }
      case "Erd": {
        const names: string[] = (pr.entities ?? []).map((n: El) => n.props.name);
        for (const d of dupes(names)) err(`duplicate entity "${d}"`);
        for (const r of (pr.relations ?? []) as El[])
          for (const k of ["from", "to"])
            if (r.props[k] && !names.includes(r.props[k]))
              err(`relation ${k} "${r.props[k]}" is not an Entity name. Names: ${names.join(", ")}`);
        break;
      }
      case "Choice": {
        if (++choices === LIMITS.choices + 1)
          err(
            `this is Choice ${LIMITS.choices + 1}: a widget has at most ${LIMITS.choices} Choices. Ask fewer questions, or split them into several widgets`,
          );
        // empty once control, invisible and blank characters are gone: it would draw as nothing and be sent as nothing
        const blank = (v: unknown) => String(v ?? "").replace(/[\p{Cc}\p{Cf}\s]/gu, "") === "";
        if (blank(pr.question)) err("the question is empty. Write the question the user answers");
        ((pr.options ?? []) as string[]).forEach((o, i) => {
          if (blank(o)) err(`option ${i + 1} is empty. Give every option some text`);
          else if (o.length > LIMITS.option)
            err(`option ${i + 1} is ${o.length} characters; the limit is ${LIMITS.option}. Shorten it`);
        });
        break;
      }
      case "CodeBlock":
        if (typeof pr.code === "string" && pr.code.split("\n").length > LIMITS.codeLines)
          err(`the code has more than ${LIMITS.codeLines} lines; show the part that matters`);
        break;
      case "InfraMap": {
        const ids: string[] = (pr.zones ?? []).flatMap((z: El) => (z.props.nodes ?? []).map((n: El) => n.props.id));
        if (ids.length > LIMITS.infraNodes)
          err(
            `${ids.length} nodes in total; the limit is ${LIMITS.infraNodes}. Draw the parts that matter, or split it into several maps`,
          );
        for (const d of dupes(ids)) err(`node id "${d}" is used twice (ids are unique across the whole map)`);
        for (const l of (pr.links ?? []) as El[])
          for (const k of ["from", "to"])
            if (l.props[k] && !ids.includes(l.props[k]))
              err(`link ${k} "${l.props[k]}" is not an InfraNode id. Ids: ${ids.join(", ")}`);
        break;
      }
      case "GeoMap": {
        const labels: string[] = (pr.points ?? []).map((n: El) => n.props.label);
        for (const d of dupes(labels)) err(`duplicate point label "${d}"`);
        for (const l of pr.links ?? [])
          for (const x of l)
            if (!labels.includes(x))
              err(`link refers to "${x}", which is not a GeoPoint label. Labels: ${labels.join(", ")}`);
        for (const pt of (pr.points ?? []) as El[]) {
          if (pt.props.lat < -90 || pt.props.lat > 90)
            err(`latitude of "${pt.props.label}" must be between -90 and 90`);
          if (pt.props.lon < -180 || pt.props.lon > 180)
            err(`longitude of "${pt.props.label}" must be between -180 and 180`);
        }
        break;
      }
      case "SequenceDiagram": {
        const ids: string[] = (pr.participants ?? []).map((n: El) => n.props.id);
        for (const d of dupes(ids)) err(`duplicate participant id "${d}"`);
        const walk = (steps: El[]) => {
          for (const s of steps ?? []) {
            if (s.type === "SeqMessage") {
              for (const k of ["from", "to"])
                if (!ids.includes(s.props[k]))
                  err(`message ${k} "${s.props[k]}" is not a SeqParticipant id. Ids: ${ids.join(", ")}`);
            } else if (s.type === "SeqNote") {
              for (const k of ["over", "to"])
                if (s.props[k] && !ids.includes(s.props[k]))
                  err(`note ${k} "${s.props[k]}" is not a SeqParticipant id. Ids: ${ids.join(", ")}`);
            } else if (s.type === "SeqGroup") {
              walk(s.props.steps);
              walk(s.props.elseSteps);
            }
          }
        };
        walk(pr.steps);
        break;
      }
    }
  };

  const rootStmt = prog.statements.get("root")!;
  visiting.push("root");
  const rootEl = resolve(rootStmt.expr, "child", "root");
  visiting.pop();
  if (rootEl && rootEl.type !== "Stack")
    errors.push(`${where(rootStmt.line)}root must be a Stack(...), not ${rootEl.type}`);
  for (const name of prog.order)
    if (!used.has(name))
      errors.push(
        `${where(prog.statements.get(name)!.line)}"${name}" is defined but not reachable from root, so it would not render. Add it to its parent's children or items array`,
      );
  const seen = new Set<string>();
  const uniq = errors.filter((m) => (seen.has(m) ? false : (seen.add(m), true)));
  return { ok: uniq.length === 0 && !!rootEl, errors: uniq, warnings, root: uniq.length === 0 ? rootEl : undefined };
}

/** Text for the model: syntax rules, signatures, actions, tooltips, examples. */
export function reference(): string {
  const sig = (s: CompSpec) =>
    `${s.name}(${s.params.map((q) => `${q.name}${q.opt ? "?" : ""}: ${showType(q.t)}`).join(", ")}) — ${s.desc}`;
  return `You write OpenUI Lang to draw an interactive widget in the terminal (unicode, drawn inline in the pi transcript). Pass it as the \`source\` of render_widget. Output only OpenUI Lang statements.

## Syntax
1. One statement per line: \`name = Expression\`. \`root = Stack([...])\` comes first.
2. Expressions: "strings" (backslash escapes, use \\n for newlines), numbers, true/false, null, [arrays], {objects}, Component(args), @Step(args), references to other names.
3. Arguments are POSITIONAL. Write Metric("CPU", "38%", "+2%", "good"), never delta: "+2%". Skip an optional argument in the middle with null.
4. Every name except root must be referenced by another statement, or it is an error (it would not render). References may be used before they are defined.
5. Helper components (TabItem, DagNode, Entity, Relation, Series, GeoPoint, FaqItem, Infra*, Seq*, Tooltip) belong only in their parent's argument.
6. The widget is drawn in a terminal of about 100 columns: keep labels short, tables under about 12 rows.

## Components
${SPECS.map(sig).join("\n")}

## Tooltips
\`tooltips: Tooltip[]\` is the LAST argument of Dag, Erd, InfraMap, SequenceDiagram, Table, Metric, BarChart, LineChart, AreaChart, PieChart, RadarChart, GeoMap and Image, so write null for each skipped optional argument before it, e.g. Dag(nodes, edges, null, null, [t1]).
Targets: Dag node id; Erd entity name or entity.column; InfraMap node id, zone:<zone name> or <fromId>><toId>; SequenceDiagram participant id or SeqMessage id; Table row:3, col:status or cell:3.status; Metric its own label; BarChart a bar label; GeoMap a point label or A>B; Image the word image; LineChart and AreaChart an x label (that point) or series:<name>; PieChart a slice label; RadarChart an axis label or series:<name>.
An element that has a tooltip shows a small ◆ on it. Hovering shows the hint; clicking (or Enter) opens the panel with detail, code and link. Use a tooltip for context the reader may want but should not have to read: why a point spiked, what a slice contains, what a node does.
hint is the short text shown on hover (140 characters). detail (800), code (40 lines) and link (http/https) open in a panel when the element is clicked.

## Limits
A widget over a limit is not drawn, and the error says which. Source ${LIMITS.source / 1000} KB, ${LIMITS.elements} components, ${LIMITS.depth} levels of nesting, ${LIMITS.children} children per Stack or Card, ${LIMITS.tabs} tabs. Text ${LIMITS.text} characters, Markdown ${LIMITS.markdown}, any other string ${LIMITS.string}, a table cell ${LIMITS.cell}. Table ${LIMITS.tableRows} rows by ${LIMITS.tableCols} columns. BarChart ${LIMITS.bars} bars. LineChart and AreaChart ${LIMITS.points} points and ${LIMITS.series} series. RadarChart ${LIMITS.axes} axes. Dag ${LIMITS.dagNodes} nodes and ${LIMITS.dagEdges} edges. Erd ${LIMITS.entities} entities, ${LIMITS.fields} fields each, ${LIMITS.relations} relations. InfraMap ${LIMITS.infraNodes} nodes and ${LIMITS.infraLinks} links. GeoMap ${LIMITS.geoPoints} points and ${LIMITS.links} links. Faq ${LIMITS.faq} items. Pictures ${LIMITS.images} (Image components and markdown picture lines together). CodeBlock ${LIMITS.codeLines} lines. Choices ${LIMITS.choices} per widget, a question ${LIMITS.question} characters, an option ${LIMITS.option}, ${LIMITS.choiceOptions} options, the user's Other answer ${LIMITS.other}. A SequenceDiagram ${LIMITS.steps} steps. Prefer far less: the reader sees one terminal screen.

## Actions
Button("Restart", @ToAssistant("Restart the vllm service")) sends that text to the assistant as the user's next turn (500 characters at most). A Button without an action sends its label. Several steps: Action([@ToAssistant("...")]). Do not use @OpenUrl: put links in a Text.
Choice(question, options, hint, multiple, other) asks the user. A click only stages an answer; the user's Send answers button sends every Choice of the widget together as ONE message (one single question gives "<question> — <option>"; otherwise a numbered list, "(no answer)" for a skipped question, "other: ..." for typed text). multiple: true allows several options; other (default true) adds an Other... row for the user's own answer. Write null for a skipped optional argument: Choice("Which checks?", ["Disk", "Memory"], null, true).

## Choosing components
Metric for a number, Table for comparisons, BarChart for one series of categories, LineChart for trends with several series, AreaChart for stacked volume, PieChart for parts of a whole (8 slices at most), RadarChart for comparing entities across metrics, Dag for task graphs and pipelines, Erd for database schemas, InfraMap for how systems connect, SequenceDiagram for call flows, GeoMap for places, Tabs/Card to organise. Series values must have the same length as labels. All data is literal: no queries, no variables. Image shows a real picture in terminals that support it (Kitty, Ghostty, WezTerm, Warp, herdr): src is an absolute file path (png, jpg, gif, webp) or a data: URI.

## Example
root = Stack([title, kpis, load, go])
title = Text("Cluster status", "title")
kpis = Stack([m1, m2, m3], "row")
m1 = Metric("GPU util", "87%", "+4% vs 1h ago", "warn")
m2 = Metric("Nodes up", "4 / 4", "all healthy", "good")
m3 = Metric("Queue", "12", "-3", "good")
load = Card([chart], "Load by node")
chart = BarChart(["spark-1", "spark-2", "spark-3", "spark-4"], [91, 84, 77, 62], null, "%")
go = Button("Show slowest requests", @ToAssistant("Show the 10 slowest vLLM requests from the last hour"))
`;
}

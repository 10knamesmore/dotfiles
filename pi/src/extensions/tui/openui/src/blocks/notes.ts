// Notes: the user annotates an element of a widget and sends the notes back to the agent as one message.
// Pure logic, no pi imports. The words a note carries about an element come from the widget (an agent wrote them) and end up
// in a message the user sends, so they are cleaned first: control and invisible characters go, backticks and quotes are made
// harmless, and `@file:`-style references are defused.
import { textWidth, truncateText, type Cell } from "../diagram/canvas.ts";
import type { El } from "../lang/library.ts";
import type { Block, ItemInfo, Note, Target, WState } from "./types.ts";
import { boxed, cell, fromRows, textCells, vstack, wrapText } from "./util.ts";

export const NOTE_LIMITS = { note: 1000, notes: 20, label: 40, cell: 60, excerpt: 60 };

/** Components a note can be attached to. Buttons and choices act, and layout components only hold others. */
export const NOTEABLE = new Set([
  "Text",
  "Markdown",
  "Callout",
  "CodeBlock",
  "Faq",
  "Metric",
  "Image",
  "Table",
  "BarChart",
  "LineChart",
  "AreaChart",
  "PieChart",
  "RadarChart",
  "GeoMap",
  "Dag",
  "Erd",
  "InfraMap",
  "SequenceDiagram",
]);
/** Components whose whole body is one item: there is nothing finer to point at. */
export const WHOLE_ONLY = new Set(["Text", "Markdown", "Callout", "CodeBlock", "Faq", "Metric", "Image"]);

// ---------- cleaning ----------

/** Control characters, line and paragraph separators and Unicode format characters (zero width, bidi marks): none may reach a message. */
export function strip(value: unknown, keepNewline = false): string {
  const s = value == null ? "" : String(value);
  return s
    .replace(/\r\n?/g, "\n")
    .replace(/[\p{Cc}\p{Cf}\u2028\u2029]/gu, (ch) => (keepNewline && ch === "\n" ? ch : ""));
}

/** Any run of whitespace becomes one space; the ends are trimmed. */
export function collapse(value: unknown): string {
  const s = value == null ? "" : String(value);
  return s
    .replace(/\p{Cf}/gu, "")
    .replace(/[^\P{Cc}\t\n\r]/gu, "")
    .replace(/\s+/g, " ")
    .trim();
}

export function clip(s: string, max: number): string {
  const chars = Array.from(s);
  return chars.length <= max
    ? s
    : chars
        .slice(0, Math.max(0, max - 3))
        .join("")
        .trimEnd() + "...";
}

/** A word that starts with @ could become a file reference in a message: write it as (at). */
export const neutralize = (s: string): string => s.replace(/(^|\s)@(?=\S)/g, "$1(at)");

/** A word of the widget that goes inside backticks or double quotes of the message: one line, no backtick or quote of its own. */
export function quiet(raw: unknown, max: number): string {
  return clip(neutralize(collapse(raw).replace(/`/g, "'").replace(/"/g, "'")), max);
}

/** The user's own text: newlines stay, trailing blanks go. Not neutralized: it is theirs. */
export function cleanNote(raw: unknown): string {
  return strip(raw, true)
    .split("\n")
    .map((l) => l.replace(/[ \t]+$/g, ""))
    .join("\n")
    .trim();
}

// ---------- words for an item ----------

const q = (s: unknown, max = NOTE_LIMITS.label) => `"${quiet(s, max)}"`;
const b = (s: unknown, max = NOTE_LIMITS.label) => `\`${quiet(s, max)}\``;
const firstLine = (s: unknown) =>
  collapse(
    String(s ?? "")
      .split("\n")
      .find((l) => l.trim()) ?? "",
  );

/** What identifies a whole component when it has no finer key: a short quote of its own words. */
function excerpt(el: El): string {
  const p = el.props;
  switch (el.type) {
    case "Text":
    case "Callout":
      return quiet(p.text, NOTE_LIMITS.excerpt);
    case "Markdown":
      return quiet(
        String(p.text ?? "")
          .replace(/^#+\s*/gm, "")
          .split("\n")
          .find((l) => l.trim()) ?? "",
        NOTE_LIMITS.excerpt,
      );
    case "CodeBlock":
      return quiet(p.title ?? firstLine(p.code), NOTE_LIMITS.excerpt);
    case "Faq":
      return quiet(p.items?.[0]?.props?.question ?? "", NOTE_LIMITS.excerpt);
    case "Metric":
      return quiet(p.label, NOTE_LIMITS.label);
    case "Image":
      return quiet(p.alt ?? p.src, NOTE_LIMITS.excerpt);
    default:
      return p.title ? quiet(p.title, NOTE_LIMITS.excerpt) : "";
  }
}

function tableWords(el: El, key: string): { where: string; short: string } {
  const rows = (el.props.rows ?? []) as unknown[][];
  const cols = ((el.props.columns ?? []) as string[]).map(String);
  let m: RegExpExecArray | null;
  if ((m = /^row:(\d+)$/.exec(key))) {
    const first = rows[Number(m[1]) - 1]?.[0];
    return {
      where: `row ${m[1]}${first !== undefined ? ` (${q(first, NOTE_LIMITS.cell)})` : ""}`,
      short: `row ${m[1]}`,
    };
  }
  if ((m = /^col:(.*)$/.exec(key))) return { where: `column ${b(m[1])}`, short: `column ${quiet(m[1], 20)}` };
  if ((m = /^cell:(\d+)\.(.*)$/.exec(key))) {
    const v = rows[Number(m[1]) - 1]?.[cols.indexOf(m[2])];
    return {
      where: `row ${m[1]}, column ${b(m[2])}${v !== undefined ? `, cell ${q(v, NOTE_LIMITS.cell)}` : ""}`,
      short: `row ${m[1]}, ${quiet(m[2], 20)}`,
    };
  }
  return { where: key, short: key };
}

/** The words that say which element of a component a note is about. `key` is the tooltip vocabulary; "" is the whole component. */
export function itemWords(el: El, key: string): { where: string; short: string } {
  if (!key) {
    const x = excerpt(el);
    return { where: x ? `"${x}"` : "", short: x ? `"${clip(x, 24)}"` : "" };
  }
  let m: RegExpExecArray | null;
  switch (el.type) {
    case "Table":
      return tableWords(el, key);
    case "Dag": {
      const n = (el.props.nodes as El[] | undefined)?.find((x) => x.props.id === key);
      const label = n && n.props.label !== key ? ` (${q(n.props.label)})` : "";
      return { where: `node ${b(key)}${label}`, short: `node ${quiet(n?.props.label ?? key, 24)}` };
    }
    case "Erd":
      return key.includes(".")
        ? { where: `column ${b(key)}`, short: `column ${quiet(key, 30)}` }
        : { where: `table ${b(key)}`, short: `table ${quiet(key, 24)}` };
    case "InfraMap":
      if ((m = /^zone:(.*)$/.exec(key))) return { where: `zone ${q(m[1])}`, short: `zone ${quiet(m[1], 24)}` };
      if (key.includes(">")) {
        const [from, to] = key.split(">");
        return { where: `link ${b(from)} -> ${b(to)}`, short: `link ${quiet(key, 30)}` };
      }
      return { where: `node ${b(key)}`, short: `node ${quiet(key, 24)}` };
    case "SequenceDiagram": {
      const isPart = (el.props.participants as El[] | undefined)?.some((x) => x.props.id === key);
      return {
        where: `${isPart ? "participant" : "message"} ${b(key)}`,
        short: `${isPart ? "participant" : "message"} ${quiet(key, 24)}`,
      };
    }
    case "GeoMap":
      if (key.includes(">")) {
        const [from, to] = key.split(">");
        return { where: `link ${q(from)} -> ${q(to)}`, short: `link ${quiet(key, 30)}` };
      }
      return { where: `pin ${q(key)}`, short: `pin ${quiet(key, 24)}` };
    case "BarChart":
      return { where: `bar ${q(key)}`, short: `bar ${quiet(key, 24)}` };
    case "PieChart": {
      const labels = (el.props.labels as string[]) ?? [];
      const v = (el.props.values as number[])?.[labels.indexOf(key)];
      return { where: `slice ${q(key)}${v !== undefined ? ` (value ${v})` : ""}`, short: `slice ${quiet(key, 24)}` };
    }
    case "LineChart":
    case "AreaChart":
    case "RadarChart":
      if ((m = /^series:(.*)$/.exec(key))) return { where: `series ${q(m[1])}`, short: `series ${quiet(m[1], 24)}` };
      return el.type === "RadarChart"
        ? { where: `axis ${q(key)}`, short: `axis ${quiet(key, 24)}` }
        : { where: `point ${q(key)}`, short: `point ${quiet(key, 24)}` };
  }
  return { where: key, short: key };
}

/** Words and identity for one item. `path` is the component's place in the tree; `tab` the tabs it sits under. */
export function itemInfo(el: El, key: string, path: string, tabs: string[], parentKey?: string): ItemInfo {
  const w = itemWords(el, key);
  const tab = tabs.length ? tabs.map((t) => quiet(t, 30)).join(" > ") : undefined;
  return {
    host: path,
    type: el.type,
    name: el.name,
    key,
    where: w.where,
    short: [el.type, w.short].filter(Boolean).join(" > "),
    line: el.line,
    ...(tab ? { tab } : {}),
    ...(parentKey !== undefined ? { parent: `item:${path}:${parentKey}` } : {}),
  };
}

export const itemId = (path: string, key: string) => `item:${path}:${key}`;

/** The item that contains this one: a table cell's row, an ERD column's table, otherwise the whole component. */
export function parentKeyOf(type: string, key: string): string {
  if (type === "Table") {
    const m = /^cell:(\d+)\./.exec(key);
    if (m) return `row:${m[1]}`;
  }
  if (type === "Erd" && key.includes(".")) return key.split(".")[0];
  return "";
}

// ---------- the message ----------

/** A title to name the widget in the message: the first heading, or the first component title. */
export function widgetTitle(root: El | undefined): string {
  let found = "";
  const walk = (el: El | undefined) => {
    if (!el || found) return;
    if (el.type === "Text" && (el.props.variant === "title" || el.props.variant === "heading"))
      found = String(el.props.text ?? "");
    else if (typeof el.props.title === "string" && el.props.title) found = el.props.title;
    if (found) return;
    for (const v of Object.values(el.props ?? {}))
      for (const x of Array.isArray(v) ? v : [v])
        if (x && typeof x === "object" && "type" in (x as object) && "props" in (x as object)) walk(x as El);
  };
  walk(root);
  return quiet(found, 60);
}

export function describeNote(n: Note): string {
  const who = n.info.name ? b(n.info.name) : `the ${n.info.type}${n.info.line ? ` at line ${n.info.line}` : ""}`;
  const where = n.info.where ? ` > ${n.info.where}` : "";
  const tab = n.info.tab ? ` [tab "${n.info.tab}"]` : "";
  return `${who} (${n.info.type})${tab}${where}`;
}

/** The text that lands in the message box. Numbered, so the agent can answer each note by its number. */
export function buildMessage(notes: Note[], title: string): string {
  const lines = [
    `Notes on the widget${title ? ` "${title}"` : " you drew"} (${notes.length}). Please answer each note by its number. Names in backticks are the variable names in the widget source; table rows count from 1, not counting the header row.`,
    "",
  ];
  notes.forEach((n, i) => {
    lines.push(`${i + 1}. ${describeNote(n)}`);
    for (const l of n.text.split("\n")) lines.push(`   ${l}`);
  });
  return lines.join("\n");
}

// ---------- the tray and the pins ----------

const ROLE_PIN = "note.pin";

function isBlank(c: Cell | undefined) {
  return !!c && c.ch === " " && c.role === "";
}

/** Draw the number of a note on its element where it does not hide anything: the left end of a box's top border, the cell before a one-line element, or just after it. */
export function placePin(block: Block, t: Target, n: number): boolean {
  const digits = String(n);
  const w = digits.length;
  const r = t.rect;
  const put = (x: number, y: number): boolean => {
    const row = block.rows[y];
    if (!row || x < 0 || x + w > row.length) return false;
    for (let i = 0; i < w; i++) row[x + i] = cell(digits[i], ROLE_PIN);
    return true;
  };
  const row = block.rows[r.y];
  if (!row) return false;
  // the top border of a box
  if (r.h >= 2) {
    for (let x = r.x + 2; x + w < r.x + r.w - 1; x++) {
      let ok = true;
      for (let i = 0; i < w + 1; i++) {
        const c = row[x + i];
        if (!c || !(c.ch === "─" || c.ch === "╌")) ok = false;
      }
      if (ok) return put(x, r.y);
    }
  }
  // before the element, then after the last character of its first row
  if (r.x - w >= 0 && Array.from({ length: w }, (_, i) => row[r.x - w + i]).every(isBlank)) return put(r.x - w, r.y);
  let xe = Math.min(block.w, r.x + r.w) - 1;
  while (xe >= r.x && isBlank(row[xe])) xe--;
  const after = xe + 1 + (row[xe + 1]?.ch === "◆" ? 2 : 0);
  if (after + w <= block.w && Array.from({ length: w }, (_, i) => row[after + i]).every(isBlank))
    return put(after, r.y);
  return false;
}

/** Pins on the elements that have notes, then the tray under the widget: one entry per note and the two buttons. */
export function decorateNotes(block: Block, state: WState, width: number): Block {
  if (!state.notes.length) return block;
  state.notes.forEach((n, i) => {
    const t = block.targets.find((x) => x.id === n.itemId);
    if (t) placePin(block, t, i + 1);
  });
  const iw = Math.max(20, width - 4);
  const rows: Cell[][] = [];
  const targets: Target[] = [];
  state.notes.forEach((n, i) => {
    const head: Cell[] = [
      ...textCells(`${i + 1}`, ROLE_PIN),
      cell(" "),
      ...textCells(truncateText(describeNote(n).replace(/`/g, ""), iw - 4), "notes.where"),
    ];
    const body = wrapText(n.text.replace(/\n/g, " ¶ "), iw - 3, "notes.text").slice(0, 2);
    const y0 = rows.length;
    rows.push(head, ...body.map((l) => [cell(" "), cell(" "), cell(" "), ...l]));
    targets.push({
      id: `note:${n.id}`,
      kind: "note",
      rect: { x: 0, y: y0, w: iw, h: 1 + body.length },
      label: `note ${i + 1}`,
      noteId: n.id,
    });
  });
  const addLabel = `[ Add ${state.notes.length} note${state.notes.length === 1 ? "" : "s"} to your message ]`;
  const clearLabel = "[ Clear ]";
  const btn: Cell[] = [...textCells(addLabel, "btn"), cell(" "), cell(" "), ...textCells(clearLabel, "btn")];
  targets.push({
    id: "noteadd",
    kind: "noteadd",
    rect: { x: 0, y: rows.length, w: textWidth(addLabel), h: 1 },
    label: addLabel,
  });
  targets.push({
    id: "noteclear",
    kind: "noteclear",
    rect: { x: textWidth(addLabel) + 2, y: rows.length, w: textWidth(clearLabel), h: 1 },
    label: clearLabel,
  });
  rows.push(btn);
  const tray = boxed(fromRows(iw, rows, targets), {
    role: "notes.border",
    title: `Notes (${state.notes.length})`,
    titleRole: "notes.title",
  });
  const out = vstack([block, tray], 0, Math.max(block.w, tray.w));
  return out;
}

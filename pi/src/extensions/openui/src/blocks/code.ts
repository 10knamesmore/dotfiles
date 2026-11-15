import { COPIED_LABEL, COPY_LABEL } from "@dotfiles/pi-ui/action-feedback";
import { textWidth, type Cell } from "../diagram/canvas.ts";
import type { Block, Target } from "./types.ts";
import { boxed, cell, fromRows, textCells, vstack } from "./util.ts";

const KW: Record<string, string[]> = {
  python:
    "def class return if elif else for while in not and or import from as with try except finally raise lambda None True False pass break continue yield async await global nonlocal is del assert".split(
      " ",
    ),
  js: "const let var function return if else for while of in new class extends import export from default async await try catch finally throw switch case break continue typeof instanceof null undefined true false this interface type enum implements public private readonly".split(
    " ",
  ),
  bash: "if then else elif fi for do done while until case esac function in echo export cd sudo set local return exit source".split(
    " ",
  ),
  sql: "select from where join left right inner outer on group by order limit offset insert into values update set delete create table alter drop index and or not null as distinct having union all primary key foreign references default count sum avg min max like in is between exists case when then else end with".split(
    " ",
  ),
  yaml: ["true", "false", "null", "yes", "no"],
  json: ["true", "false", "null"],
  go: "func package import var const type struct interface return if else for range switch case default go defer chan map select break continue nil true false".split(
    " ",
  ),
  rust: "fn let mut pub struct enum impl trait use mod return if else match for while loop in as ref self Self true false const static async await move where type unsafe".split(
    " ",
  ),
};
const ALIAS: Record<string, string> = {
  py: "python",
  python3: "python",
  javascript: "js",
  jsx: "js",
  typescript: "js",
  ts: "js",
  tsx: "js",
  node: "js",
  sh: "bash",
  shell: "bash",
  zsh: "bash",
  yml: "yaml",
  golang: "go",
  rs: "rust",
  postgresql: "sql",
  postgres: "sql",
  mysql: "sql",
  jsonc: "json",
};
const COMMENT: Record<string, string> = {
  python: "#",
  bash: "#",
  yaml: "#",
  js: "//",
  go: "//",
  rust: "//",
  sql: "--",
  json: "",
};

export function normLang(lang?: string): string | null {
  const l = (lang ?? "").toLowerCase().trim();
  const k = ALIAS[l] ?? l;
  return KW[k] ? k : null;
}

function highlightLine(line: string, lang: string | null): Cell[] {
  if (!lang) return textCells(line, "code");
  const kw = new Set(KW[lang]);
  const ci = lang === "sql";
  const cm = COMMENT[lang];
  const out: Cell[] = [];
  let i = 0;
  const push = (s: string, role: string) => out.push(...textCells(s, role));
  while (i < line.length) {
    const c = line[i];
    if (cm && line.startsWith(cm, i)) {
      push(line.slice(i), "code.com");
      break;
    }
    if (c === '"' || c === "'" || c === "`") {
      let j = i + 1;
      while (j < line.length && line[j] !== c) j += line[j] === "\\" ? 2 : 1;
      const s = line.slice(i, Math.min(j + 1, line.length));
      const next = line.slice(j + 1).trimStart()[0];
      push(s, (lang === "json" || lang === "yaml") && next === ":" ? "code.key" : "code.str");
      i = j + 1;
      continue;
    }
    if (/[0-9]/.test(c) && !/[A-Za-z_]/.test(line[i - 1] ?? "")) {
      const m = /^\d[\d_.]*(e[+-]?\d+)?[a-z]*/i.exec(line.slice(i))!;
      push(m[0], "code.num");
      i += m[0].length;
      continue;
    }
    if (/[A-Za-z_$]/.test(c)) {
      const m = /^[A-Za-z_$][\w$-]*/.exec(line.slice(i))!;
      const w = m[0];
      let role = "code";
      if (kw.has(ci ? w.toLowerCase() : w)) role = "code.kw";
      else if (lang === "yaml" && line.slice(i + w.length).startsWith(":")) role = "code.key";
      else if (line[i + w.length] === "(") role = "code.fn";
      else if (/^[A-Z][a-z]/.test(w) && lang !== "bash") role = "code.type";
      else if (lang === "bash" && line[i - 1] === "$") role = "code.fn";
      push(w, role);
      i += w.length;
      continue;
    }
    if (lang === "bash" && c === "$") {
      push("$", "code.fn");
      i++;
      continue;
    }
    push(c, "code");
    i++;
  }
  return out;
}

export function codeInner(code: string, lang: string | undefined, width: number): Block {
  const l = normLang(lang);
  const lines = String(code ?? "")
    .replace(/\t/g, "  ")
    .replace(/\n$/, "")
    .split("\n");
  const rows = lines.flatMap((ln) => {
    const cells = highlightLine(ln, l);
    const wrapped: Cell[][] = [];
    for (let start = 0; start < cells.length;) {
      let end = Math.min(start + width, cells.length);
      // Keep a wide character and its continuation cell on the same row.
      if (cells[end]?.ch === "") end--;
      wrapped.push(cells.slice(start, end));
      start = end;
    }
    return wrapped.length ? wrapped : [[]];
  });
  return fromRows(width, rows);
}

export function codeBlock(
  code: string,
  lang: string | undefined,
  title: string | undefined,
  width: number,
  copyId: string | null,
  copied: boolean,
): Block {
  const inner = codeInner(code, lang, Math.max(8, width - 4));
  const label = [title, lang && !title ? lang : title && lang ? lang : ""].filter(Boolean).join(" · ");
  const tagText = copyId ? (copied ? COPIED_LABEL : COPY_LABEL) : undefined;
  const b = boxed(inner, {
    role: "code.border",
    title: label || undefined,
    titleRole: "code.title",
    tag: tagText,
    tagRole: copied ? "action.success" : "action",
  });
  if (copyId && tagText) {
    const len = textWidth(tagText);
    const t: Target = {
      id: copyId,
      kind: "copy",
      rect: { x: b.w - 1 - len, y: 0, w: len, h: 1 },
      label: "copy code",
      copy: code,
    };
    b.targets.push(t);
  }
  return b;
}

void cell;
void vstack;

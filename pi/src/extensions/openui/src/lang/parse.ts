// OpenUI Lang parser (the positional, literal-data subset): `name = Expr` statements, hoisted references.
// Expr: "string", number, true/false/null, [array], {object}, Component(args...), @Step(args...), reference.
// Positional arguments only. Collects every error with a line number instead of stopping at the first.

export type Expr =
  | { k: "str"; v: string; line: number }
  | { k: "num"; v: number; line: number }
  | { k: "bool"; v: boolean; line: number }
  | { k: "null"; line: number }
  | { k: "arr"; items: Expr[]; line: number }
  | { k: "obj"; entries: Array<[string, Expr]>; line: number }
  | { k: "call"; name: string; args: Expr[]; line: number }
  | { k: "action"; name: string; args: Expr[]; line: number }
  | { k: "ref"; name: string; line: number };

export interface Program {
  statements: Map<string, { expr: Expr; line: number }>;
  order: string[];
  errors: string[];
}

type Tok = { t: "id" | "str" | "num" | "at" | "p"; v: string; line: number };

function tokenize(src: string, errors: string[]): Tok[] {
  const out: Tok[] = [];
  let i = 0;
  let line = 1;
  while (i < src.length) {
    const c = src[i];
    if (c === "\n") {
      line++;
      i++;
      continue;
    }
    if (c === " " || c === "\t" || c === "\r") {
      i++;
      continue;
    }
    if (c === "/" && src[i + 1] === "/") {
      while (i < src.length && src[i] !== "\n") i++;
      continue;
    }
    if (c === '"') {
      let j = i + 1;
      let s = "";
      let closed = false;
      while (j < src.length) {
        const d = src[j];
        if (d === "\\") {
          const e = src[j + 1];
          if (e === "n") s += "\n";
          else if (e === "t") s += "\t";
          else if (e === "r") s += "\r";
          else if (e === "u" && /^[0-9a-fA-F]{4}$/.test(src.slice(j + 2, j + 6))) {
            s += String.fromCharCode(parseInt(src.slice(j + 2, j + 6), 16));
            j += 4;
          } else if (e !== undefined) s += e;
          j += 2;
          continue;
        }
        if (d === '"') {
          closed = true;
          break;
        }
        if (d === "\n") line++;
        s += d;
        j++;
      }
      if (!closed) errors.push(`line ${line}: a string is not closed (missing a double quote)`);
      out.push({ t: "str", v: s, line });
      i = j + 1;
      continue;
    }
    if (/[0-9]/.test(c) || (c === "-" && /[0-9.]/.test(src[i + 1] ?? ""))) {
      const m = /^-?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?/.exec(src.slice(i));
      if (m) {
        out.push({ t: "num", v: m[0], line });
        i += m[0].length;
        continue;
      }
    }
    if (/[A-Za-z_$]/.test(c)) {
      let j = i + 1;
      while (j < src.length && /[A-Za-z0-9_$]/.test(src[j])) j++;
      out.push({ t: "id", v: src.slice(i, j), line });
      i = j;
      continue;
    }
    if (c === "@") {
      let j = i + 1;
      while (j < src.length && /[A-Za-z0-9_]/.test(src[j])) j++;
      out.push({ t: "at", v: src.slice(i + 1, j), line });
      i = j;
      continue;
    }
    if ("=()[]{},:".includes(c)) {
      out.push({ t: "p", v: c, line });
      i++;
      continue;
    }
    errors.push(`line ${line}: unexpected character "${c}"`);
    i++;
  }
  return out;
}

/** Deepest nesting of arrays, objects and calls inside one statement. 16 nested components need about 32. */
export const MAX_NESTING = 48;

export function parseProgram(src: string): Program {
  const errors: string[] = [];
  const toks = tokenize(String(src ?? ""), errors);
  const statements: Program["statements"] = new Map();
  const order: string[] = [];
  let p = 0;
  const peek = () => toks[p];
  const isP = (v: string) => toks[p]?.t === "p" && toks[p].v === v;

  class ParseError extends Error {}
  const fail = (line: number, msg: string): never => {
    throw new ParseError(`line ${line}: ${msg}`);
  };

  // each nested [ { or ( adds a level of recursion: bound it so a hostile program cannot overflow the stack
  let level = 0;
  const expr = (): Expr => {
    if (++level > MAX_NESTING) {
      level = 0;
      return fail(peek()?.line ?? 1, `brackets are nested more than ${MAX_NESTING} deep`);
    }
    try {
      return exprInner();
    } finally {
      if (level > 0) level--;
    }
  };
  const exprInner = (): Expr => {
    const t = peek();
    if (!t) return fail(toks[toks.length - 1]?.line ?? 1, "the statement ends too early");
    if (t.t === "str") {
      p++;
      return { k: "str", v: t.v, line: t.line };
    }
    if (t.t === "num") {
      p++;
      return { k: "num", v: Number(t.v), line: t.line };
    }
    if (t.t === "at") {
      p++;
      if (!isP("(")) return fail(t.line, `@${t.v} needs arguments: @${t.v}(...)`);
      const args = argList(t.line);
      return { k: "action", name: t.v, args, line: t.line };
    }
    if (t.t === "p" && t.v === "[") {
      p++;
      const items: Expr[] = [];
      while (!isP("]")) {
        if (!peek()) return fail(t.line, "an array is not closed (missing ])");
        items.push(expr());
        if (isP(",")) p++;
        else if (!isP("]")) return fail(peek()?.line ?? t.line, `expected "," or "]" in an array`);
      }
      p++;
      return { k: "arr", items, line: t.line };
    }
    if (t.t === "p" && t.v === "{") {
      p++;
      const entries: Array<[string, Expr]> = [];
      while (!isP("}")) {
        const key = peek();
        if (!key) return fail(t.line, "an object is not closed (missing })");
        if (key.t !== "id" && key.t !== "str") return fail(key.line, "an object key must be a name or a string");
        p++;
        if (!isP(":")) return fail(key.line, `an object key needs ":" after it`);
        p++;
        entries.push([key.v, expr()]);
        if (isP(",")) p++;
        else if (!isP("}")) return fail(peek()?.line ?? t.line, `expected "," or "}" in an object`);
      }
      p++;
      return { k: "obj", entries, line: t.line };
    }
    if (t.t === "id") {
      p++;
      if (t.v === "true" || t.v === "false") return { k: "bool", v: t.v === "true", line: t.line };
      if (t.v === "null") return { k: "null", line: t.line };
      if (isP("(")) {
        const args = argList(t.line);
        return { k: "call", name: t.v, args, line: t.line };
      }
      return { k: "ref", name: t.v, line: t.line };
    }
    return fail(t.line, `unexpected "${t.v}"`);
  };

  const argList = (line: number): Expr[] => {
    p++; // (
    const args: Expr[] = [];
    while (!isP(")")) {
      const t = peek();
      if (!t) return fail(line, "a call is not closed (missing a closing parenthesis)");
      if (t.t === "id" && toks[p + 1]?.t === "p" && toks[p + 1].v === ":")
        return fail(
          t.line,
          `named argument "${t.v}:" is not supported. Arguments are positional: write the value in order, use null to skip an optional one`,
        );
      args.push(expr());
      if (isP(",")) p++;
      else if (!isP(")")) return fail(peek()?.line ?? line, `expected "," or ")" in a call`);
    }
    p++;
    return args;
  };

  while (p < toks.length) {
    const start = toks[p];
    try {
      if (start.t !== "id" || !(toks[p + 1]?.t === "p" && toks[p + 1].v === "="))
        return fail(start.line, `expected a statement like  name = Component(...)  but found "${start.v}"`) as never;
      const name = start.v;
      p += 2;
      const e = expr();
      if (statements.has(name))
        errors.push(`line ${start.line}: "${name}" is defined twice (the later definition wins)`);
      else order.push(name);
      statements.set(name, { expr: e, line: start.line });
    } catch (e) {
      if (!(e instanceof ParseError)) throw e;
      errors.push(e.message);
      // resynchronise at the next `identifier =`
      p++;
      while (p < toks.length && !(toks[p].t === "id" && toks[p + 1]?.t === "p" && toks[p + 1].v === "=")) p++;
    }
  }
  return { statements, order, errors };
}

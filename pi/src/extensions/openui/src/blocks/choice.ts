// Choice: what an answer is and the one message the user sends. Pure logic, no pi imports.
// A click stages an answer; nothing is sent until the user presses Send answers, and then every Choice of the widget goes in ONE
// message. Port of hermes-openui's choice-core.js, on pi-openui's own cleaning rules (notes.ts).
//
// The message is the contract with the agent. One single-choice question answered with an option keeps the line agents already
// know, `<question> — <option>`. Anything else is a numbered list. The agent wrote the question and the options and the message
// is sent without a review step, so those words are cleaned like a Notes quote: control and invisible characters out, whitespace
// collapsed, `@file:`-style references defused. The user's own Other text is theirs: only control characters go.
import type { El } from "../lang/library.ts";
import { LIMITS } from "../lang/library.ts";
import { clip, collapse, neutralize, quiet, strip } from "./notes.ts";

export const NO_ANSWER = "(no answer)";

/** A question as the board knows it. `question` and `options` are the cleaned words the message uses. */
export interface Choice {
  question: string;
  options: string[];
  multiple: boolean;
  other: boolean;
  /** a fingerprint of the words and the two flags: Choices with one key are ONE question, they share their answer and count once */
  key: string;
}

/** What the user has staged for one question. `picks` are option indexes, ascending. `other` is the Other row being selected. */
export interface Answer {
  picks: number[];
  other: boolean;
  text: string;
}
export type Answers = Record<string, Answer>;

const BLANK: Answer = { picks: [], other: false, text: "" };
const cur = (a: Answers, c: Choice): Answer => a[c.key] ?? BLANK;
const word = (raw: unknown, max: number): string => clip(neutralize(collapse(raw)), max);

export function makeChoice(props: Record<string, any>): Choice {
  const options = (Array.isArray(props?.options) ? props.options : []).map((o: unknown) => word(o, LIMITS.option));
  const question = word(props?.question, LIMITS.question);
  const multiple = props?.multiple === true;
  const other = props?.other !== false;
  return { question, options, multiple, other, key: JSON.stringify([question, options, multiple, other]) };
}

/** Every Choice of a compiled tree in document order, whether or not its tab was ever shown. Identical questions are merged. */
export function collectChoices(root: El | undefined): Choice[] {
  const out: Choice[] = [];
  const keys = new Set<string>();
  // the compiler bounds the depth (LIMITS.depth) and an element reached twice is read once, so the walk needs no bound of its own:
  // one would silently drop a Choice that is drawn and clickable
  const seen = new Set<object>();
  const walk = (x: unknown) => {
    if (!x || typeof x !== "object" || seen.has(x)) return;
    seen.add(x);
    if (Array.isArray(x)) return x.forEach(walk);
    const el = x as El;
    if (el.type === "Choice") {
      const c = makeChoice(el.props);
      if (!keys.has(c.key)) {
        keys.add(c.key);
        out.push(c);
      }
      return;
    }
    if (el.props && typeof el.props === "object") for (const v of Object.values(el.props)) walk(v);
  };
  walk(root);
  return out;
}

// ---------- the answers (every function returns new objects) ----------

/** The user's own text, tidied: a tab is a space, trailing blanks and control characters go, a run of blank lines is one blank line. */
function tidyOther(raw: unknown): string {
  return strip(raw == null ? "" : String(raw).replace(/\t/g, " "), true)
    .split("\n")
    .map((l) => l.replace(/[ \t]+$/g, ""))
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

/**
 * The user's own text as it is kept and sent: newlines stay (a run of blank lines is one, so a message cannot be padded), a tab is
 * a space, trailing blanks and control characters go, at most LIMITS.other characters. Not defused: it is theirs.
 */
export function cleanOther(raw: unknown): string {
  return Array.from(tidyOther(raw)).slice(0, LIMITS.other).join("");
}

/** How long the text is once tidied and before the cut, so "shortened" is said only when something is cut. */
export const otherLength = (raw: unknown): number => Array.from(tidyOther(raw)).length;

export function toggleOption(a: Answers, c: Choice, i: number): Answers {
  if (!Number.isInteger(i) || i < 0 || i >= c.options.length) return a;
  const now = cur(a, c);
  let next: Answer;
  if (c.multiple)
    next = {
      ...now,
      picks: now.picks.includes(i) ? now.picks.filter((x) => x !== i) : [...now.picks, i].sort((x, y) => x - y),
    };
  else
    next = now.picks.length === 1 && now.picks[0] === i ? { ...now, picks: [] } : { ...now, picks: [i], other: false }; // another option moves the pick and replaces Other
  return { ...a, [c.key]: next };
}

/** The user saved their own answer. Text selects the Other row (single mode: it replaces the option); empty text unselects it. */
export function setOther(a: Answers, c: Choice, text: string): Answers {
  if (!c.other) return a;
  const clean = cleanOther(text);
  const now = cur(a, c);
  if (!clean) return { ...a, [c.key]: { ...now, other: false, text: "" } };
  return { ...a, [c.key]: { picks: c.multiple ? now.picks : [], other: true, text: clean } };
}

const otherOf = (c: Choice, a: Answer): string => (c.other && a.other ? cleanOther(a.text) : "");

/** An Other row that is selected but empty is not an answer. */
export function isAnswered(c: Choice, a: Answers): boolean {
  const now = cur(a, c);
  return now.picks.some((i) => i >= 0 && i < c.options.length) || otherOf(c, now) !== "";
}

export function progress(cs: Choice[], a: Answers): { answered: number; total: number } {
  return { answered: cs.filter((c) => isAnswered(c, a)).length, total: cs.length };
}

/** `disk; memory; other: also look at the logs`, or "" when not answered. A second line of Other text is indented under the question. */
export function answerWords(c: Choice, a: Answers): string {
  const now = cur(a, c);
  const parts = now.picks.filter((i) => i >= 0 && i < c.options.length).map((i) => c.options[i]);
  const text = otherOf(c, now);
  if (text) parts.push("other: " + text.split("\n").join("\n   "));
  return parts.join("; ");
}

// ---------- the message ----------

/** pi, like most hosts, may run a message that starts with `/name` as a command; a question must never be the first words of one. */
const SLASH_COMMAND = /^\/[^\s/]*(?:\s|$)/;
export const runsAsCommand = (text: string): boolean => SLASH_COMMAND.test(String(text).trim());

export function buildMessage(cs: Choice[], a: Answers, title: string): string {
  if (cs.length === 1 && !cs[0].multiple) {
    const c = cs[0];
    const now = cur(a, c);
    if (now.picks.length === 1 && c.options[now.picks[0]] !== undefined && !otherOf(c, now)) {
      const line = `${c.question} \u2014 ${c.options[now.picks[0]]}`;
      if (!runsAsCommand(line)) return line;
    }
  }
  const name = title ? quiet(title, 60) : "";
  const head = name
    ? `Answers to the questions in widget \`${name}\` (${cs.length}):`
    : `Answers to the questions in this widget (${cs.length}):`;
  const lines = cs.map((c, i) => `${i + 1}. ${c.question}\n   ${answerWords(c, a) || NO_ANSWER}`);
  return `${head}\n\n${lines.join("\n")}`;
}

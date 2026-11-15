// Role -> style mapping. Actions use the shared UI feedback styles; other elements retain
// their diagram-specific focus and hover treatments.

import type { Palette } from "./canvas.ts";
import type { ActionAppearance } from "@dotfiles/pi-ui/action-feedback";

export type Style = (s: string) => string;

export interface PaletteTokens {
  action: (appearance: ActionAppearance) => Style;
  good: Style;
  bad: Style;
  warn: Style;
  accent: Style;
  info: Style;
  muted: Style;
  dim: Style;
  bold: Style;
  italic: Style;
  underline: Style;
  strike: Style;
  code: Style;
  codeBorder: Style;
  keyword: Style;
  string: Style;
  number: Style;
  comment: Style;
  fn: Style;
  type: Style;
  link: Style;
  /** categorical series colors 0..7 */
  series: Style[];
  /** the hover bubble background (and its text) */
  bubble: Style;
  /** two-colour pixel for the pie: top and bottom slice (null = empty) */
  pie: (top: number | null, bottom: number | null) => Style;
}

const reverse: Style = (s) => `\x1b[7m${s}\x1b[27m`;
const underline: Style = (s) => `\x1b[4m${s}\x1b[24m`;
const compose = (...fs: Array<Style | undefined>): Style | undefined => {
  const list = fs.filter(Boolean) as Style[];
  return list.length ? (s) => list.reduce((acc, f) => f(acc), s) : undefined;
};

export function createPalette(t: PaletteTokens): Palette {
  const status: Record<string, Style> = {
    done: t.good,
    up: t.good,
    running: t.accent,
    failed: t.bad,
    down: t.bad,
    degraded: t.warn,
    pending: t.muted,
    skipped: t.muted,
    unknown: t.muted,
  };
  const tone: Record<string, Style> = {
    neutral: t.muted,
    info: t.info,
    good: t.good,
    warn: t.warn,
    bad: t.bad,
    success: t.good,
    error: t.bad,
  };
  const ser = (i: number) => t.series[((i % 8) + 8) % 8];
  return (role) => {
    if (!role) return undefined;
    const parts = role.split(".");
    let focus = false;
    let hover = false;
    while (parts.length > 1 && (parts[parts.length - 1] === "focus" || parts[parts.length - 1] === "hover")) {
      if (parts.pop() === "focus") focus = true;
      else hover = true;
    }
    if (parts.length === 1 && (parts[0] === "focus" || parts[0] === "hover")) {
      if (parts[0] === "focus") focus = true;
      else hover = true;
      parts.length = 0;
    }
    const [base, a, b] = parts;
    let f: Style | undefined;
    switch (base) {
      // diagrams (existing)
      case "box":
      case "icon":
      case "status":
        f = a ? status[a] : undefined;
        break;
      case "zone":
        f = a === "title" ? compose(b ? tone[b] : undefined, t.bold) : a ? tone[a] : t.muted;
        break;
      case "wire":
        f = t.muted;
        break;
      case "arrow":
      case "mark":
        f = t.accent;
        break;
      case "label":
        f = a === "edge" ? t.dim : undefined;
        break;
      case "elabel":
      case "detail":
      case "legend":
        f = t.dim;
        break;
      case "erd":
        f =
          a === "title"
            ? t.bold
            : a === "pk" || a === "key"
              ? t.warn
              : a === "type" || a === "flag"
                ? t.dim
                : undefined;
        break;
      // text and layout
      case "text":
        f =
          a === "title"
            ? compose(t.accent, t.bold)
            : a === "heading"
              ? t.bold
              : a === "muted"
                ? t.dim
                : a === "code"
                  ? t.code
                  : a === "warn"
                    ? t.warn
                    : undefined;
        break;
      case "title":
        f = t.bold;
        break;
      case "card":
        f = a === "title" ? t.bold : t.muted;
        break;
      case "tile":
        f = t.muted;
        break;
      case "metric":
        f = a === "label" ? t.dim : a === "value" ? t.bold : a ? tone[a] : undefined;
        break;
      case "callout":
        f = a ? tone[a] : undefined;
        break;
      case "btn":
      case "action":
        return t.action(
          a === "success"
            ? "success"
            : a === "pending"
              ? "pending"
              : a === "error"
                ? "error"
                : focus || hover
                  ? "active"
                  : "idle",
        );
      case "choice":
        f =
          a === "q"
            ? t.bold
            : a === "picked"
              ? compose(t.accent, t.bold)
              : a === "tag" || a === "other"
                ? t.dim
                : a === "locked"
                  ? t.dim
                  : a === "sent"
                    ? t.good
                    : undefined;
        break;
      case "tab":
        f =
          a === "on"
            ? t.accent
            : a === "label"
              ? compose(t.accent, t.bold)
              : a === "off"
                ? t.dim
                : a === "rule"
                  ? t.muted
                  : undefined;
        break;
      case "faq":
        f = a === "q" ? t.bold : t.accent;
        break;
      case "image":
        f = a === "alt" ? t.bold : a === "border" ? t.muted : t.dim;
        break;
      case "tip":
        f =
          a === "bubble" ? compose(b ? tone[b] : undefined, t.bubble) : a === "hint" ? undefined : a ? tone[a] : t.info;
        break;
      case "note":
        f = a === "pin" ? compose(t.accent, t.bold, reverse) : compose(a ? tone[a] : t.info, t.bold);
        break;
      case "notes":
        f =
          a === "on"
            ? compose(t.accent, t.bold, reverse)
            : a === "title"
              ? compose(t.accent, t.bold)
              : a === "border"
                ? t.muted
                : a === "where"
                  ? t.dim
                  : undefined;
        break;
      case "foot":
        f = a === "mark" ? t.accent : t.dim;
        break;
      // markdown
      case "md":
        f =
          a === "h1"
            ? compose(t.accent, t.bold)
            : a === "h2"
              ? t.bold
              : a === "h3"
                ? compose(t.dim, t.bold)
                : a === "bold"
                  ? t.bold
                  : a === "italic"
                    ? t.italic
                    : a === "code"
                      ? t.code
                      : a === "quote"
                        ? compose(t.dim, t.italic)
                        : a === "quoteBar" || a === "hr"
                          ? t.muted
                          : a === "link"
                            ? compose(t.link, t.underline)
                            : a === "url"
                              ? t.dim
                              : a === "strike"
                                ? t.strike
                                : a === "bullet"
                                  ? t.accent
                                  : undefined;
        break;
      case "code":
        f =
          a === "kw"
            ? t.keyword
            : a === "str"
              ? t.string
              : a === "num"
                ? t.number
                : a === "com"
                  ? t.comment
                  : a === "fn"
                    ? t.fn
                    : a === "type"
                      ? t.type
                      : a === "key"
                        ? t.accent
                        : a === "border"
                          ? t.codeBorder
                          : a === "title"
                            ? t.codeBorder
                            : a === "copied"
                              ? t.good
                              : undefined;
        break;
      case "table":
        f = a === "head" ? t.bold : t.muted;
        break;
      // charts
      case "chart":
        f =
          a === "title"
            ? t.bold
            : a === "axis" || a === "grid"
              ? t.muted
              : a === "axisLabel" || a === "label"
                ? t.dim
                : undefined;
        break;
      case "series":
        f = ser(Number(a ?? 0));
        break;
      case "pie": {
        const top = a === "x" || a === undefined ? null : Number(a);
        const bot = b === "x" || b === undefined ? null : Number(b);
        f = top === null && bot === null ? undefined : t.pie(top, bot);
        break;
      }
      // map
      case "map":
        f =
          a === "land"
            ? t.dim
            : a === "coast"
              ? t.muted
              : a === "link"
                ? t.warn
                : a === "label"
                  ? t.bold
                  : a === "pin"
                    ? tone[b ?? "info"]
                    : undefined;
        break;
      // sequence
      case "seq":
        f =
          a === "life"
            ? t.dim
            : a === "act"
              ? t.accent
              : a === "head" || a === "group"
                ? t.muted
                : a === "icon"
                  ? t.accent
                  : a === "title"
                    ? t.bold
                    : a === "num"
                      ? t.dim
                      : a === "noteBorder"
                        ? t.warn
                        : a === "groupKind"
                          ? compose(t.accent, t.bold)
                          : a === "groupLabel"
                            ? t.dim
                            : undefined;
        break;
    }
    // Border characters are line art: underlining or reversing them draws a tick or a block on every row of a vertical
    // edge. Those cells light up in the accent colour instead; text cells keep underline (hover) and reverse (focus).
    const lineArt = base === "box" || base === "tile" || base === "card" || base === "zone" || a === "border";
    if (lineArt && (focus || hover)) return (s) => t.bold(t.accent(s));
    if (focus) return (s) => reverse(f ? f(s) : s);
    if (hover) return (s) => underline(f ? f(s) : s);
    return f;
  };
}

const sgr =
  (open: string, close: string): Style =>
  (s) =>
    `\x1b[${open}m${s}\x1b[${close}m`;
const C256 = [36, 32, 33, 31, 35, 34, 96, 93];
export const ansiPalette: Palette = createPalette({
  action: (appearance) =>
    sgr(
      appearance === "success"
        ? "32;1"
        : appearance === "error"
          ? "31;1"
          : appearance === "pending"
            ? "90"
            : appearance === "active"
              ? "36;1;4"
              : "34",
      "0",
    ),
  good: sgr("32", "39"),
  bad: sgr("31", "39"),
  warn: sgr("33", "39"),
  accent: sgr("36", "39"),
  info: sgr("34", "39"),
  muted: sgr("90", "39"),
  dim: sgr("2", "22"),
  bold: sgr("1", "22"),
  italic: sgr("3", "23"),
  underline: sgr("4", "24"),
  strike: sgr("9", "29"),
  code: sgr("33", "39"),
  codeBorder: sgr("90", "39"),
  keyword: sgr("35", "39"),
  string: sgr("32", "39"),
  number: sgr("33", "39"),
  comment: sgr("90", "39"),
  fn: sgr("34", "39"),
  type: sgr("36", "39"),
  link: sgr("34", "39"),
  series: C256.map((c) => sgr(String(c), "39")),
  bubble: (s) => `\x1b[30;47m${s}\x1b[0m`,
  pie: (a, b) => (s) => {
    const fg = a !== null ? C256[a % 8] : C256[(b ?? 0) % 8];
    const bg = a !== null && b !== null ? C256[b % 8] + 10 : null;
    return `\x1b[${fg}${bg ? `;${bg}` : ""}m${s}\x1b[0m`;
  },
});

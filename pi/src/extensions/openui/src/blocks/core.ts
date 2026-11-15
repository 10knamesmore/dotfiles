// Layout and simple components: Stack, Text, Card, Callout, Metric, Button, Choice, Tabs, Faq, Image + tooltips.
import { textWidth, truncateText, type Cell, type Rect } from "../diagram/canvas.ts";
import type { El } from "../lang/library.ts";
import { buildMessage, cleanOther, makeChoice, progress, type Choice } from "./choice.ts";
import { codeBlock, codeInner } from "./code.ts";
import { markdownBlock } from "./markdown.ts";
import { itemId, itemInfo, NOTEABLE, parentKeyOf, WHOLE_ONLY } from "./notes.ts";
import type { Block, Ctx, Target, Tip, WState } from "./types.ts";
import {
  blank,
  boxed,
  cell,
  fromRows,
  hstack,
  padRow,
  shiftTargets,
  textCells,
  vstack,
  wrapRuns,
  wrapText,
} from "./util.ts";

const ROW_GAP: Record<string, number> = { none: 0, xs: 0, s: 0, m: 1, l: 1, xl: 2 };
const COL_GAP: Record<string, number> = { none: 0, xs: 1, s: 1, m: 2, l: 3, xl: 4 };

/** width an element wants when it sits in a row, or null when it should share the space */
export function natural(el: El): number | null {
  const p = el.props;
  switch (el.type) {
    case "Button":
      return textWidth(String(p.label ?? "")) + 4;
    case "Metric":
      return Math.max(
        16,
        textWidth(String(p.label ?? "")) + 4,
        textWidth(String(p.value ?? "")) + 4,
        textWidth(String(p.delta ?? "")) + 4,
      );
    case "Text":
      return Math.min(60, textWidth(String(p.text ?? "")));
    default:
      return null;
  }
}

export function stack(el: El, width: number, ctx: Ctx, path: string): Block {
  const kids: El[] = el.props.children ?? [];
  const dir = el.props.direction ?? "column";
  const gapTok = el.props.gap ?? "m";
  if (dir === "column") {
    const blocks = kids.map((k, i) => ctx.render(k, width, `${path}.${i}`));
    return vstack(blocks, ROW_GAP[gapTok] ?? 1, width);
  }
  const gap = COL_GAP[gapTok] ?? 2;
  const nat = kids.map(natural);
  const lines: number[][] = [];
  if (el.props.wrap) {
    let cur: number[] = [];
    let used = 0;
    const share = Math.max(14, Math.floor((width - 2 * gap) / 3));
    kids.forEach((_, i) => {
      const w = Math.min(width, nat[i] ?? share);
      if (cur.length && used + gap + w > width) {
        lines.push(cur);
        cur = [];
        used = 0;
      }
      used += (cur.length ? gap : 0) + w;
      cur.push(i);
    });
    if (cur.length) lines.push(cur);
  } else lines.push(kids.map((_, i) => i));
  const rendered = lines.map((idxs) => {
    const n = idxs.length;
    const fixed = idxs.reduce((a, i) => a + (nat[i] ?? 0), 0);
    const flex = idxs.filter((i) => nat[i] === null);
    const room = width - gap * (n - 1) - fixed;
    const flexW = flex.length ? Math.max(8, Math.floor(room / flex.length)) : 0;
    let widths = idxs.map((i) => nat[i] ?? flexW);
    let total = widths.reduce((a, b) => a + b, 0) + gap * (n - 1);
    while (total > width && Math.max(...widths) > 6) {
      const k = widths.indexOf(Math.max(...widths));
      widths[k]--;
      total--;
    }
    widths = widths.map((w) => Math.max(1, w));
    const blocks = idxs.map((i, k) => ctx.render(kids[i], widths[k], `${path}.${i}`));
    return hstack(blocks, gap);
  });
  const out = vstack(rendered, 0, width);
  return { ...out, rows: out.rows.map((r) => padRow(r, width)) };
}

export function text(el: El, width: number): Block {
  const v = String(el.props.text ?? "");
  const role =
    { title: "text.title", heading: "text.heading", muted: "text.muted", code: "text.code" }[
      el.props.variant as string
    ] ?? "";
  return fromRows(width, wrapText(v, width, role));
}

export function card(el: El, width: number, ctx: Ctx, path: string): Block {
  const inner = stack(
    { type: "Stack", props: { children: el.props.children ?? [], gap: "m" }, line: el.line },
    Math.max(4, width - 4),
    ctx,
    path,
  );
  return boxed(inner, { role: "card", title: el.props.title, titleRole: "card.title" });
}

const CALLOUT_ICON: Record<string, string> = { info: "◉", success: "✓", warn: "⚠", error: "✗" };
export function callout(el: El, width: number): Block {
  const tone = el.props.tone ?? "info";
  const inner = fromRows(Math.max(4, width - 6), wrapText(String(el.props.text ?? ""), Math.max(4, width - 6), ""));
  const body = fromRows(
    inner.w + 2,
    inner.rows.map((r, i) => [i === 0 ? cell(CALLOUT_ICON[tone], `callout.${tone}`) : cell(" "), cell(" "), ...r]),
  );
  return boxed(body, { role: `callout.${tone}` });
}

export function metric(
  el: El,
  width: number,
  path: string,
): { block: Block; anchors: Map<string, { rect: Rect; targetId?: string }> } {
  const p = el.props;
  const w = Math.max(10, width);
  const rows = [
    textCells(truncateText(String(p.label), w - 4), "metric.label"),
    textCells(truncateText(String(p.value), w - 4), "metric.value"),
  ];
  if (p.delta) rows.push(textCells(truncateText(String(p.delta), w - 4), `metric.${p.tone ?? "neutral"}`));
  const block = boxed(fromRows(w - 4, rows), { role: "tile" });
  void path;
  return { block, anchors: new Map([[String(p.label), { rect: { x: 0, y: 0, w: block.w, h: block.rows.length } }]]) };
}

export function button(el: El, width: number, path: string, ctx: Ctx): Block {
  const p = el.props;
  const label = String(p.label ?? "");
  const steps: Array<{ name: string; args: any[] }> = p.action?.steps ?? [];
  const say = steps.find((s) => s.name === "ToAssistant");
  const url = steps.find((s) => s.name === "OpenUrl");
  const send = say ? String(say.args[0]) : url ? undefined : label;
  const id = `btn:${path}`;
  const delivery = ctx.state.delivery[id];
  const copied = ctx.state.copied === id;
  const feedback = copied ? "✓ ok" : delivery === "sent" ? "✓ sent" : delivery === "failed" ? "failed" : delivery;
  const room = Math.min(Math.max(7, textWidth(label)), Math.max(1, width - 4));
  const shown = truncateText(feedback ?? label, room);
  const role =
    copied || delivery === "sent"
      ? "action.success"
      : delivery === "failed"
        ? "action.error"
        : delivery
          ? "action.pending"
          : "action";
  const cells = textCells(`-[${shown}${" ".repeat(Math.max(0, room - textWidth(shown)))}]-`, role);
  const t: Target = {
    id,
    kind: "button",
    rect: { x: 0, y: 0, w: cells.length, h: 1 },
    label,
    send,
    openUrl: url ? String(url.args[0]) : undefined,
  };
  return fromRows(Math.max(cells.length, 1), [cells], [t]);
}

const SEND = "-[Send answers]-";
const CLEAR = "-[Clear]-";

/** One question: its tag, hint, one row per option and (unless `other` is false) an Other row. A click only stages the answer. */
export function choice(el: El, width: number, ctx: Ctx, path: string): Block {
  const c = makeChoice(el.props);
  const ans = ctx.state.answers[c.key];
  const locked = ctx.state.sent;
  const head = wrapRuns([{ t: c.question, role: "choice.q" }], width);
  const tag = [textCells(c.multiple ? "Choose all that apply" : "Choose one", "choice.tag")];
  const hint = el.props.hint ? wrapText(String(el.props.hint), width, "text.muted") : [];
  const rows: Cell[][] = [...head, ...tag, ...hint];
  const targets: Target[] = [];
  const row = (on: boolean, label: string, index: number | "other") => {
    const role = on ? "choice.picked" : locked ? "choice.locked" : "choice";
    const mark = c.multiple ? (on ? "[✓]" : "[ ]") : on ? "●" : "○";
    const indent = textWidth(mark) + 1;
    // the whole option is drawn: what is sent is what the user could read. A long one wraps under its first word.
    const lines = wrapText(label, Math.max(4, width - indent), role);
    const y = rows.length;
    lines.forEach((l, k) =>
      rows.push(
        k === 0
          ? [...textCells(mark, role), cell(" ", role), ...l]
          : [...Array.from({ length: indent }, () => cell(" ")), ...l],
      ),
    );
    if (!locked)
      targets.push({
        id: `choice:${path}:${index}`,
        kind: "choice",
        rect: { x: 0, y, w: width, h: lines.length },
        label,
        choice: { of: c, index },
      });
  };
  c.options.forEach((o, i) => row(!!ans?.picks.includes(i), o, i));
  if (c.other) {
    const typed = ans?.other ? cleanOther(ans.text) : "";
    row(typed !== "", "Other...", "other");
    if (typed) {
      const room = Math.max(4, width - 4);
      const lines = wrapText(typed.replace(/\n/g, " ¶ "), room, "choice.other");
      const shown = lines.slice(0, 2);
      if (lines.length > 2)
        shown[1] = textCells(truncateText(shown[1].map((x) => x.ch).join("") + "…", room), "choice.other");
      for (const l of shown) rows.push([cell(" "), cell(" "), cell(" "), cell(" "), ...l]);
    }
  }
  return fromRows(width, rows, targets);
}

/**
 * The bar under a widget that has Choices: how many are answered, and the buttons. Send carries the finished message, and has a
 * target only when something is answered; Clear only when something is staged. After Send the bar only says so.
 */
export function answerBar(choices: Choice[], state: WState, title: string, width: number): Block {
  if (state.sent)
    return fromRows(width, [
      state.sending
        ? textCells(
            state.delivery.send === "queued" ? "Queued for the assistant." : "Sending to the assistant…",
            "action.pending",
          )
        : textCells("✓ Sent to the assistant.", "action.success"),
    ]);
  const { answered, total } = progress(choices, state.answers);
  const staged = Object.values(state.answers).some((a) => a.picks.length > 0 || a.other);
  const count = textCells(`${answered} of ${total} answered`, "text.muted");
  const sendW = textWidth(SEND);
  const clearW = textWidth(CLEAR);
  const send = textCells(SEND, answered > 0 ? "btn" : "choice.locked");
  const clear = textCells(CLEAR, staged ? "btn" : "choice.locked");
  const gap = [cell(" "), cell(" ")];
  // as much as fits on one row: the count and both buttons, else the count above the buttons, else one button per row
  const rows: Cell[][] = [];
  let sendAt: { x: number; y: number };
  let clearAt: { x: number; y: number };
  if (count.length + 2 + sendW + 2 + clearW <= width) {
    rows.push([...count, ...gap, ...send, ...gap, ...clear]);
    sendAt = { x: count.length + 2, y: 0 };
    clearAt = { x: count.length + 2 + sendW + 2, y: 0 };
  } else if (sendW + 2 + clearW <= width) {
    rows.push(count, [...send, ...gap, ...clear]);
    sendAt = { x: 0, y: 1 };
    clearAt = { x: sendW + 2, y: 1 };
  } else {
    rows.push(count, send, clear);
    sendAt = { x: 0, y: 1 };
    clearAt = { x: 0, y: 2 };
  }
  const targets: Target[] = [];
  if (answered > 0)
    targets.push({
      id: "send",
      kind: "send",
      rect: { ...sendAt, w: sendW, h: 1 },
      label: "Send answers",
      bar: { choices, title },
    });
  if (staged) targets.push({ id: "clear", kind: "clear", rect: { ...clearAt, w: clearW, h: 1 }, label: "Clear" });
  return fromRows(width, rows, targets);
}

export function tabs(el: El, width: number, ctx: Ctx, path: string): Block {
  const items: El[] = el.props.items ?? [];
  if (!items.length) return blank(width, 0);
  const sel = Math.min(Math.max(0, ctx.state.tab[path] ?? 0), items.length - 1);
  let labels = items.map((it) => truncateText(String(it.props.label ?? ""), 22));
  const need = () => labels.reduce((a, l) => a + textWidth(l) + 4, 0);
  let cap = 22;
  while (need() > width && cap > 4) labels = items.map((it) => truncateText(String(it.props.label ?? ""), --cap));
  const l0: ReturnType<typeof textCells> = [];
  const l1: typeof l0 = [];
  const l2: typeof l0 = [];
  const targets: Target[] = [];
  labels.forEach((lab, i) => {
    const iw = textWidth(lab) + 2;
    const on = i === sel;
    const role = on ? "tab.on" : "tab.off";
    const x0 = l1.length;
    if (on) {
      l0.push(cell("╭", role), ...Array.from({ length: iw }, () => cell("─", role)), cell("╮", role));
      l1.push(cell("│", role), cell(" ", role), ...textCells(lab, "tab.label"), cell(" ", role), cell("│", role));
      l2.push(cell("╯", role), ...Array.from({ length: iw }, () => cell(" ")), cell("╰", role));
    } else {
      l0.push(...Array.from({ length: iw + 2 }, () => cell(" ")));
      l1.push(cell(" "), cell(" "), ...textCells(lab, role), cell(" "), cell(" "));
      l2.push(...Array.from({ length: iw + 2 }, () => cell("─", "tab.rule")));
    }
    targets.push({
      id: `tab:${path}:${i}`,
      kind: "tab",
      rect: { x: x0, y: 1, w: iw + 2, h: 1 },
      label: String(items[i].props.label ?? ""),
    });
  });
  while (l2.length < width) l2.push(cell("─", "tab.rule"));
  const strip = fromRows(width, [l0, l1, l2], targets);
  ctx.tabs.push(String(items[sel].props.label ?? ""));
  let content: Block;
  try {
    content = stack(
      { type: "Stack", props: { children: items[sel].props.content ?? [], gap: "m" }, line: el.line },
      width,
      ctx,
      `${path}.t${sel}`,
    );
  } finally {
    ctx.tabs.pop();
  }
  return vstack([strip, vstack([blank(width, 0), content], 0, width)], 0, width);
}

export function faq(el: El, width: number, ctx: Ctx, path: string): Block {
  const items: El[] = el.props.items ?? [];
  const out: Block[] = [];
  items.forEach((it, i) => {
    const id = `faq:${path}:${i}`;
    if (ctx.state.faq[id] === undefined) ctx.state.faq[id] = !!el.props.firstOpen && i === 0;
    const open = ctx.state.faq[id];
    const qRows = wrapRuns([{ t: String(it.props.question ?? ""), role: "faq.q" }], width - 2);
    const head = fromRows(
      width,
      qRows.map((r, k) => [cell(k === 0 ? (open ? "▾" : "▸") : " ", "faq.mark"), cell(" "), ...r]),
      [{ id, kind: "faq", rect: { x: 0, y: 0, w: width, h: qRows.length }, label: String(it.props.question ?? "") }],
    );
    if (!open) {
      out.push(head);
      return;
    }
    const ans = markdownBlock(String(it.props.answer ?? ""), Math.max(4, width - 2), ctx, `${path}.a${i}`);
    // the answer sits two columns in; its targets (a picture) move with it
    const body = fromRows(
      width,
      ans.rows.map((r) => [cell(" "), cell(" "), ...r]),
      shiftTargets(ans.targets, 2, 0),
    );
    out.push(vstack([head, body], 0, width));
  });
  return vstack(out, 0, width);
}

export function image(el: El, width: number, path: string, ctx: Ctx): { block: Block; anchors: Map<string, Anchor> } {
  const p = el.props;
  const src = String(p.src ?? "");
  const svc = ctx.services.images;
  const name = src.startsWith("data:") ? "inline image" : src.split(/[\\/]/).pop() || src;
  const alt = String(p.alt ?? name);
  const placed = svc?.place(src, Math.min(Math.max(8, width - 2), 64), 22) ?? null;
  let block: Block;
  if (placed) {
    // the picture is drawn over these cells by the host; here they are reserved, framed and clickable
    const area = blank(placed.cols, placed.rows);
    area.targets.push({
      id: `image:${path}`,
      kind: "image",
      rect: { x: 0, y: 0, w: placed.cols, h: placed.rows },
      label: `${alt}`,
      image: { key: placed.key, cols: placed.cols, rows: placed.rows },
    });
    const cap = p.caption ? fromRows(placed.cols, wrapText(String(p.caption), placed.cols, "text.muted")) : null;
    block = boxed(cap ? vstack([area, cap], 0, placed.cols) : area, {
      role: "image.border",
      title: alt,
      titleRole: "image.alt",
      padX: 0,
    });
  } else {
    const facts = svc ? svc.describe(src) : name;
    const lines = [
      textCells(`▒ ${alt}`, "image.alt"),
      textCells(truncateText(facts, Math.max(4, width - 6)), "text.muted"),
    ];
    if (p.caption) lines.push(textCells(truncateText(String(p.caption), Math.max(4, width - 6)), "text.muted"));
    block = boxed(fromRows(Math.max(8, Math.min(width - 4, Math.max(...lines.map((l) => l.length)) + 1)), lines), {
      role: "image.border",
      dashed: true,
    });
    block.targets.push({
      id: `image:${path}`,
      kind: "image",
      rect: { x: 0, y: 0, w: block.w, h: block.rows.length },
      label: alt,
    });
  }
  const frame = { x: 0, y: 0, w: block.w, h: block.rows.length };
  const self = block.targets.find((t) => t.kind === "image");
  return { block, anchors: new Map([["image", { rect: frame, targetId: self?.id }]]) };
}

// ---------- tooltips ----------

export interface Anchor {
  rect: Rect;
  targetId?: string;
  /** where the tooltip badge goes, when the default (end of the first row of `rect`) would be wrong */
  badge?: [number, number];
  /** draw the badge even over a cell that is not empty (a point on a braille curve) */
  badgeForce?: boolean;
  /** cells besides `rect` that belong to the element (a pie wedge) */
  cells?: Array<[number, number]>;
}

/** The mark that says "this element has a tooltip". Present in the user's font (JetBrains Mono), unlike the circled i. */
export const BADGE = "◆";

const isBlank = (c: { ch: string; role: string } | undefined) => !!c && c.ch === " " && c.role === "";
const isDash = (c: { ch: string; role: string } | undefined) =>
  !!c && (c.ch === "─" || c.ch === "╌") && /^(box|zone|card|tile|image|code|seq\.head|tip)/.test(c.role);

/**
 * Put the badge where a reader looks first: on the top border of a box (just before the corner), or right after
 * the last character of a one-line element. Returns false when there is no free cell, in which case the element is
 * still hoverable and the hint line under the widget still announces that tooltips exist.
 */
export function placeBadge(block: Block, a: Anchor, tone: string): boolean {
  const put = (x: number, y: number) => {
    const row = block.rows[y];
    if (!row || x < 0 || x >= row.length) return false;
    row[x] = cell(BADGE, `note.${tone}`);
    return true;
  };
  if (a.badge) {
    const c = block.rows[a.badge[1]]?.[a.badge[0]];
    if (a.badgeForce || isBlank(c) || isDash(c)) return put(a.badge[0], a.badge[1]);
  }
  const r = a.rect;
  const row = block.rows[r.y];
  if (!row) return false;
  let xe = Math.min(block.w, r.x + r.w) - 1;
  while (xe >= r.x && isBlank(row[xe])) xe--;
  if (xe < r.x) return r.w > 0 && put(r.x, r.y);
  if (/[╮┐]/.test(row[xe].ch) && isDash(row[xe - 1])) return put(xe - 1, r.y);
  if (xe + 1 < block.w && isBlank(row[xe + 1])) return put(xe + 1, r.y);
  if (r.x - 1 >= 0 && isBlank(row[r.x - 1])) return put(r.x - 1, r.y);
  return false;
}

export function tipPanel(label: string, tip: Tip, width: number): Block {
  const iw = Math.max(8, width - 4);
  const parts: Block[] = [];
  if (tip.hint) parts.push(fromRows(iw, wrapText(tip.hint, iw, "tip.hint")));
  if (tip.detail) parts.push(markdownBlock(tip.detail, iw));
  if (tip.code) parts.push(codeInner(tip.code, tip.language, iw));
  if (tip.link)
    parts.push(
      fromRows(iw, [
        textCells(
          truncateText(`↗ ${tip.linkLabel ?? tip.link}${tip.linkLabel ? ` — ${tip.link}` : ""}`, iw),
          "md.link",
        ),
      ]),
    );
  const inner = vstack(parts.length ? parts : [fromRows(iw, [textCells(label, "tip.hint")])], 1, iw);
  return boxed(inner, { role: `tip.${tip.tone ?? "info"}`, title: label });
}

/** The parts of a component a note can point at: one item per anchor (a row, a node, a point, a slice ...). */
export function registerItems(el: El, block: Block, anchors: Map<string, Anchor>, path: string, ctx: Ctx): void {
  if (!NOTEABLE.has(el.type) || WHOLE_ONLY.has(el.type)) return;
  for (const [key, a] of anchors) {
    const r = a.rect;
    if (!key || r.w <= 0 || r.h <= 0 || r.x + r.w <= 0 || r.x >= block.w) continue;
    const info = itemInfo(el, key, path, ctx.tabs, parentKeyOf(el.type, key));
    block.targets.push({
      id: itemId(path, key),
      kind: "item",
      rect: { ...r },
      label: info.short,
      item: info,
      ...(a.cells ? { cells: a.cells } : {}),
    });
  }
}

/** One item for the whole component, so there is always something to attach a note to. */
export function addWholeItem(el: El, block: Block, path: string, ctx: Ctx): void {
  if (!NOTEABLE.has(el.type) || !block.rows.length) return;
  const info = itemInfo(el, "", path, ctx.tabs);
  block.targets.push({
    id: itemId(path, ""),
    kind: "item",
    rect: { x: 0, y: 0, w: block.w, h: block.rows.length },
    label: info.short || el.type,
    item: info,
  });
}

export function withTips(
  el: El,
  block: Block,
  anchors: Map<string, Anchor>,
  path: string,
  ctx: Ctx,
  avail = block.w,
): Block {
  registerItems(el, block, anchors, path, ctx);
  const tips: El[] = el.props.tooltips ?? [];
  for (const t of tips) {
    const key = String(t.props.target ?? "");
    const a = anchors.get(key);
    if (!a) {
      ctx.warnings.push(`${el.type}: tooltip target "${key}" does not match anything in the component`);
      continue;
    }
    const tip: Tip = {
      hint: t.props.hint,
      detail: t.props.detail,
      code: t.props.code,
      language: t.props.language,
      tone: t.props.tone,
      link: t.props.link,
      linkLabel: t.props.linkLabel,
    };
    const existing = a.targetId ? block.targets.find((x) => x.id === a.targetId) : undefined;
    const id = existing ? existing.id : `tip:${path}:${key}`;
    if (existing) existing.tip = tip;
    else block.targets.push({ id, kind: "tip", rect: a.rect, label: key, tip, ...(a.cells ? { cells: a.cells } : {}) });
    if ((existing?.kind === "sort" || existing?.kind === "cell") && a.badge)
      block.targets.push({
        id: `tip:${path}:${key}`,
        kind: existing.kind === "cell" ? "cell" : "tip",
        rect: { x: a.badge[0], y: a.badge[1], w: 1, h: 1 },
        label: key,
        tip,
        copy: existing.copy,
      });
    placeBadge(block, a, tip.tone ?? "info");
  }
  const selected = block.targets.find((t) => t.id === ctx.state.tip);
  const panelWidth = Math.min(Math.max(avail, 20), Math.max(block.w, 44));
  const parts: Block[] = [];
  if (selected?.tip && (selected.tip.detail || selected.tip.code || selected.tip.link))
    parts.push(tipPanel(selected.label, selected.tip, panelWidth));
  if (selected?.kind === "cell") {
    const copyId = `copy:${selected.id}`;
    const value = codeBlock(
      selected.copy ?? "",
      undefined,
      selected.label,
      panelWidth,
      copyId,
      ctx.state.copied === copyId,
    );
    value.targets[0].label = "copy cell value";
    parts.push(value);
  }
  const panel = parts.length ? vstack(parts, 0, panelWidth) : null;
  return panel ? vstack([block, panel], 0, Math.max(block.w, panel.w)) : block;
}

export const shift = shiftTargets;

// pi-tui component for a compiled OpenUI tree.
import { truncateToWidth, type TuiMouseEvent } from "@earendil-works/pi-tui";
import { copyToClipboard, type Theme } from "@earendil-works/pi-coding-agent";
import { ACTION_STYLES, FEEDBACK_DURATION_MS } from "@dotfiles/pi-ui/action-feedback";
import { CodeBlockFrame, type CodeBlockBody, type CodeBlockBodyAppearance } from "@dotfiles/pi-ui/code-block";
import type { ReplyBlockHost, ReplyBlockScope } from "@dotfiles/pi-ui/prompt-editor";
import { drawBubble } from "../blocks/bubble.ts";
import { BADGE } from "../blocks/core.ts";
import { activate, describe, isLive, reveal, type Env } from "../blocks/interact.ts";
import { otherLength, setOther } from "../blocks/choice.ts";
import { buildMessage, cleanNote, NOTE_LIMITS, widgetTitle } from "../blocks/notes.ts";
import { renderTree } from "../blocks/render.ts";
import { newState, type Block, type Note, type Target, type WState } from "../blocks/types.ts";
import { cell, padRow, textCells } from "../blocks/util.ts";
import { rowsToLines, textWidth, truncateText, type Cell } from "../diagram/canvas.ts";
import { createPalette } from "../diagram/palette.ts";
import { LIMITS, type El } from "../lang/library.ts";
import { imageService } from "./images.ts";

const CHIP = "notes:chip";
const SERIES = ["accent", "success", "warning", "error", "syntaxKeyword", "syntaxString", "syntaxType", "syntaxNumber"];
const sgr = (open: string, close: string) => (s: string) => `\x1b[${open}m${s}\x1b[${close}m`;

export function themePalette(theme: any) {
  const f = (token: string) => (s: string) => theme.fg(token, s);
  const colors = theme.colors ?? {};
  return createPalette({
    action: (appearance) => (text) => theme.style(text, ACTION_STYLES[appearance]),
    good: f("success"),
    bad: f("error"),
    warn: f("warning"),
    accent: f("accent"),
    info: f("accent"),
    muted: f("muted"),
    dim: f("dim"),
    bold: sgr("1", "22"),
    italic: sgr("3", "23"),
    underline: sgr("4", "24"),
    strike: sgr("9", "29"),
    code: f("mdCode"),
    codeBorder: f("mdCodeBlockBorder"),
    keyword: f("syntaxKeyword"),
    string: f("syntaxString"),
    number: f("syntaxNumber"),
    comment: f("syntaxComment"),
    fn: f("syntaxFunction"),
    type: f("syntaxType"),
    link: f("mdLink"),
    series: SERIES.map(f),
    bubble: (s) => theme.bg("selectedBg", theme.fg("text", s)),
    pie: (a, b) => (s) => {
      if (a !== null && b !== null && typeof theme.style === "function" && colors[SERIES[b % 8]])
        return theme.style(s, { fg: SERIES[a % 8], bg: colors[SERIES[b % 8]] });
      return theme.fg(SERIES[((a ?? b) as number) % 8], s);
    },
  });
}

type WidgetDestination = { mode: "live" | "echo"; requestRender(): void } | ReplyBlockScope;
type WidgetHost = Pick<ReplyBlockHost, "theme" | "pointer">;

interface Entry {
  root: El;
  frame: CodeBlockFrame<Target>;
  theme: Theme;
  palette: ReturnType<typeof themePalette> | undefined;
  state: WState;
  destination: WidgetDestination;
  block: Block | null;
  body: CodeBlockBody<Target> | undefined;
  key: string;
  version: number;
  dialog: AbortController | null;
  notice: { text: string; tone: "info" | "error" } | null;
  noticeTimer: ReturnType<typeof setTimeout> | null;
  /** Messages awaiting Pi's acceptance, keyed by the button or answer bar that sent them. */
  pending: Map<string, { text: string; timer: ReturnType<typeof setTimeout> | null }>;
  feedbackTimers: Map<string, ReturnType<typeof setTimeout>>;
}

const entries = new Map<string, Entry>();

/** Services supplied by the Pi adapter. */
export const hooks = {
  sendLive: (_text: string) => {},
  sendEcho: (_text: string) => {},
  /** Is pi holding messages for later (the agent is busy and a follow-up is queued)? Then a message it has not shown yet is not lost. */
  queued: (): boolean => false,
  /** How long to wait for pi to say it took the answers before giving the widget back. */
  deliveryMs: 3000,
  /** Ask the user for the text of a note. Resolves undefined when they cancel. */
  prompt: async (_title: string, _initial: string): Promise<string | undefined> => undefined,
  /** Put text in pi's message box without sending it. */
  toEditor: (_text: string) => {},
  copy: copyToClipboard,
};

const overlayOf = (e: Entry) => ("mode" in e.destination ? undefined : e.destination);
const redraw = (e: Entry) => e.destination.requestRender();
function requestRender(e: Entry): void {
  e.frame.invalidate();
  redraw(e);
}

// pi converts JPEG, GIF and WebP lazily after startup: when a picture becomes drawable, lay every widget out again
imageService.onLate = () => {
  const renderHosts = new Set<() => void>();
  for (const e of entries.values()) {
    e.version++;
    e.frame.invalidate();
    renderHosts.add(e.destination.requestRender);
  }
  for (const render of renderHosts) render();
};

function getEntry(
  id: string,
  root: El,
  { theme, pointer }: WidgetHost,
  destination: WidgetDestination,
  source: string,
): Entry {
  const existing = entries.get(id);
  if (existing) return existing;
  const title = widgetTitle(root);
  const e: Entry = {
    root,
    theme,
    palette: undefined,
    frame: new CodeBlockFrame<Target>({
      title: title ? `OpenUI · ${title}` : "OpenUI",
      theme,
      pointer,
      source: () => source,
      copy: (text) => {
        if (e.notice) {
          setNotice(id, null);
          e.frame.invalidate();
        }
        return hooks.copy(text);
      },
      renderBody: (width) => layout(e, width),
      requestRender: () => redraw(e),
      interaction: {
        enabled: (target) => isLive(target, e.state.noteMode),
        activate: (target, shift) => {
          if (e.notice) setNotice(id, null);
          if (reveal(target, e.state)) e.version++;
          perform(id, e, target, shift);
          e.frame.invalidate();
        },
        viewChanged: () => {
          if (e.notice) setNotice(id, null);
          if (e.state.noteMode) {
            e.state.noteMode = false;
            e.version++;
          }
          e.frame.invalidate();
        },
        viewNotice: () => (e.notice ? noticeLine(e.theme, e.notice) : undefined),
      },
    }),
    state: newState(),
    destination,
    block: null,
    body: undefined,
    key: "",
    version: 0,
    dialog: null,
    notice: null,
    noticeTimer: null,
    pending: new Map(),
    feedbackTimers: new Map(),
  };
  entries.set(id, e);
  return e;
}

function sendFollowUp(id: string, e: Entry, overlay: ReplyBlockScope, text: string): boolean {
  try {
    if (overlay.sendFollowUp(text)) return true;
  } catch {
    // Keep the current selections or notes available to retry.
  }
  setNotice(id, "Could not start a side follow-up. Try again when the side conversation is ready.", "error");
  requestRender(e);
  return false;
}

function envFor(id: string, e: Entry): Env {
  return {
    send: (t, o) => {
      const targetId = o?.targetId ?? "send";
      const destination = e.destination;
      if (!("mode" in destination)) {
        if (!sendFollowUp(id, e, destination, t)) return false;
        if (o?.confirm && entries.get(id) === e) markDelivered(e, targetId);
        return true;
      }
      if (destination.mode === "echo") {
        hooks.sendEcho(t);
        if (o?.targetId) markDelivered(e, targetId);
        return true;
      }
      if (o?.confirm) watchDelivery(id, e, t, targetId);
      hooks.sendLive(t);
      if (e.pending.has(targetId) && hooks.queued()) {
        e.state.delivery[targetId] = "queued";
        e.version++;
      }
      return true;
    },
    copy: (text, targetId) => void copyTarget(id, e, targetId, text),
    note: (t) => setNotice(id, t),
  };
}

function setNotice(id: string, text: string | null, tone: "info" | "error" = "info") {
  const e = entries.get(id);
  if (!e) return;
  e.notice = text ? { text, tone } : null;
  if (e.noticeTimer) clearTimeout(e.noticeTimer);
  e.noticeTimer = text
    ? setTimeout(() => {
        e.notice = null;
        requestRender(e);
      }, 4000)
    : null;
  if (e.noticeTimer && typeof (e.noticeTimer as any).unref === "function") (e.noticeTimer as any).unref();
}

function resetFeedback(e: Entry, key: string, clear: () => void) {
  clearTimeout(e.feedbackTimers.get(key));
  e.feedbackTimers.set(
    key,
    setTimeout(() => {
      e.feedbackTimers.delete(key);
      clear();
      e.version++;
      requestRender(e);
    }, FEEDBACK_DURATION_MS),
  );
}

async function copyTarget(id: string, e: Entry, targetId: string, text: string) {
  try {
    await hooks.copy(text);
    if (entries.get(id) !== e) return;
    e.state.copied = targetId;
    resetFeedback(e, "copy", () => {
      e.state.copied = null;
    });
    e.version++;
  } catch {
    if (entries.get(id) !== e) return;
    setNotice(id, "Could not write to the clipboard. Check clipboard access and try again.", "error");
  }
  requestRender(e);
}

function markDelivered(e: Entry, targetId: string) {
  if (targetId === "send") {
    e.state.sending = false;
    e.state.delivery.send = "sent";
  } else {
    e.state.delivery[targetId] = "sent";
    resetFeedback(e, targetId, () => {
      delete e.state.delivery[targetId];
    });
  }
  e.version++;
  requestRender(e);
}

// ---------- delivery ----------

/**
 * sendUserMessage has no delivery receipt. message_start confirms acceptance; until then the action stays pending.
 * Wait while Pi holds queued messages. Otherwise report the failed delivery and allow another attempt.
 */
function watchDelivery(id: string, e: Entry, text: string, targetId: string): void {
  const previous = e.pending.get(targetId);
  if (previous?.timer) clearTimeout(previous.timer);
  clearTimeout(e.feedbackTimers.get(targetId));
  e.feedbackTimers.delete(targetId);
  if (targetId === "send") e.state.sending = true;
  e.state.delivery[targetId] = "sending";
  e.version++;
  const pending = { text, timer: null as ReturnType<typeof setTimeout> | null };
  e.pending.set(targetId, pending);
  const check = () => {
    if (e.pending.get(targetId) !== pending) return;
    let held = false;
    try {
      held = hooks.queued();
    } catch {
      /* pi's context went stale: treat it as not held */
    }
    if (held) {
      e.state.delivery[targetId] = "queued";
      e.version++;
      requestRender(e);
      pending.timer = later(check);
      return;
    }
    e.pending.delete(targetId);
    e.state.delivery[targetId] = "failed";
    if (targetId === "send") {
      e.state.sent = false;
      e.state.sending = false;
    }
    e.version++;
    setNotice(id, "Pi did not accept the message. Fix what it says above, then send again.", "error");
    requestRender(e);
  };
  pending.timer = later(check);
}
const later = (f: () => void) => {
  const t = setTimeout(f, Math.max(1, hooks.deliveryMs));
  if (typeof (t as any).unref === "function") (t as any).unref();
  return t;
};

/**
 * Each accepted user message confirms one pending action, so equal-text requests need separate acknowledgements.
 */
export function confirmDelivery(text: string): boolean {
  const want = text.trim();
  for (const e of entries.values()) {
    for (const [targetId, pending] of e.pending) {
      if (pending.text.trim() !== want) continue;
      if (pending.timer) clearTimeout(pending.timer);
      e.pending.delete(targetId);
      markDelivered(e, targetId);
      return true;
    }
  }
  return false;
}

// ---------- notes ----------

const chipLabel = (s: WState) =>
  s.noteMode ? "[ notes: on ]" : s.notes.length ? `[ notes ${s.notes.length} ]` : "[ notes ]";

async function prompt(id: string, e: Entry, title: string, initial: string): Promise<string | undefined> {
  const dialog = new AbortController();
  e.dialog = dialog;
  try {
    const overlay = overlayOf(e);
    return overlay ? await overlay.prompt(title, initial, dialog.signal) : await hooks.prompt(title, initial);
  } catch {
    if (entries.get(id) === e) {
      setNotice(id, "The editor could not be opened.");
      requestRender(e);
    }
    return undefined;
  } finally {
    if (e.dialog === dialog) e.dialog = null;
  }
}

function setMode(id: string, e: Entry, on: boolean) {
  e.state.noteMode = on;
  e.state.tip = null;
  e.version++;
  setNotice(id, null);
  requestRender(e);
}

async function writeNote(id: string, e: Entry, t: Target): Promise<void> {
  if (e.dialog || !t.item) return;
  const existing = e.state.notes.find((n) => n.itemId === t.id);
  if (!existing && e.state.notes.length >= NOTE_LIMITS.notes) {
    setNotice(id, `${NOTE_LIMITS.notes} notes is the most per widget.`);
    requestRender(e);
    return;
  }
  await ask(id, e, existing ?? null, { itemId: t.id, info: t.item });
}

async function editNote(id: string, e: Entry, noteId: number | undefined): Promise<void> {
  const n = e.state.notes.find((x) => x.id === noteId);
  if (n && !e.dialog) await ask(id, e, n, { itemId: n.itemId, info: n.info });
}

async function ask(
  id: string,
  e: Entry,
  existing: Note | null,
  about: { itemId: string; info: Note["info"] },
): Promise<void> {
  const text = await prompt(
    id,
    e,
    `Note on ${about.info.short}${about.info.tab ? ` [tab ${about.info.tab}]` : ""}${existing ? " (editing)" : ""}`,
    existing?.text ?? "",
  );
  if (text === undefined || entries.get(id) !== e) return;
  let clean = cleanNote(text);
  if (clean.length > NOTE_LIMITS.note) {
    clean = clean.slice(0, NOTE_LIMITS.note).trimEnd();
    setNotice(id, `Note shortened to ${NOTE_LIMITS.note} characters.`);
  }
  if (!clean) {
    // saving an empty note removes it
    if (existing) e.state.notes = e.state.notes.filter((n) => n !== existing);
  } else if (existing) existing.text = clean;
  else e.state.notes.push({ id: ++e.state.noteSeq, itemId: about.itemId, info: about.info, text: clean });
  e.version++;
  requestRender(e);
}

function addToMessage(id: string, e: Entry) {
  const n = e.state.notes.length;
  if (!n) return;
  const text = buildMessage(e.state.notes, widgetTitle(e.root));
  const overlay = overlayOf(e);
  if (overlay) {
    if (!sendFollowUp(id, e, overlay, text) || entries.get(id) !== e) return;
    setNotice(id, `Sent ${n} note${n === 1 ? "" : "s"}.`);
  } else {
    hooks.toEditor(text);
    setNotice(id, `Added ${n} note${n === 1 ? "" : "s"} to your message. Read it, edit it, then press Enter to send.`);
  }
  e.state.notes = [];
  e.state.noteMode = false;
  e.version++;
  requestRender(e);
}

function clearNotes(id: string, e: Entry) {
  e.state.notes = [];
  e.version++;
  setNotice(id, "Notes cleared.");
  requestRender(e);
}

// ---------- choices ----------

/** Saved text selects Other; saving an empty answer unselects it. */
async function editOther(id: string, e: Entry, t: Target): Promise<void> {
  const x = t.choice;
  if (!x || e.dialog || e.state.sent) return;
  const text = await prompt(
    id,
    e,
    `Your own answer: ${truncateText(x.of.question, 60)}`,
    e.state.answers[x.of.key]?.text ?? "",
  );
  if (text === undefined || entries.get(id) !== e || e.state.sent) return;
  if (otherLength(text) > LIMITS.other) setNotice(id, `Answer shortened to ${LIMITS.other} characters.`);
  e.state.answers = setOther(e.state.answers, x.of, text);
  e.version++;
  requestRender(e);
}

function perform(id: string, e: Entry, t: Target, shift: boolean): void {
  switch (t.kind) {
    case "notes":
      setMode(id, e, !e.state.noteMode);
      return;
    case "item": {
      // Shift picks the item one level up: a table row instead of its cell
      const up = shift && t.item?.parent ? e.block?.targets.find((x) => x.id === t.item!.parent) : undefined;
      void writeNote(id, e, up ?? t);
      return;
    }
    case "note":
      void editNote(id, e, t.noteId);
      return;
    case "noteadd":
      addToMessage(id, e);
      return;
    case "noteclear":
      clearNotes(id, e);
      return;
    case "choice":
      if (t.choice?.index === "other") {
        void editOther(id, e, t);
        return;
      }
      break;
  }
  if (activate(t, e.state, envFor(id, e))) e.version++;
}

// ---------- the component ----------

/** The originals belong to the cached layout and are shared by every redraw. */
const markHovered = (row: Cell[], x: number) => {
  const c = row[x];
  if (c.role.startsWith("note.")) return; // a tooltip badge or a pin keeps its own colour
  if (c.role) row[x] = cell(c.ch, `${c.role}.hover`);
  else if (c.ch.trim()) row[x] = cell(c.ch, "hover");
};

function layout(e: Entry, width: number): CodeBlockBody<Target> {
  const key = `${width}:${e.version}`;
  if (e.body && e.key === key) return e.body;
  const { block: rendered, warnings } = renderTree(
    e.root,
    Math.max(24, width),
    e.state,
    { images: imageService },
    !!overlayOf(e),
  );
  let block = rendered;
  if (block.targets.some((target) => target.kind === "item")) {
    const label = chipLabel(e.state);
    const chipWidth = textWidth(label);
    const x = Math.max(0, width - chipWidth);
    const chip = textCells(label, e.state.noteMode ? "notes.on" : "btn");
    const w = Math.max(block.w, width);
    block = {
      w,
      rows: [...block.rows, padRow(padRow([], x).concat(chip), w)],
      targets: [
        ...block.targets,
        { id: CHIP, kind: "notes", rect: { x, y: block.rows.length, w: chipWidth, h: 1 }, label: "notes" },
      ],
    };
  }
  e.block = block;
  e.key = key;
  e.body = {
    targets: block.targets,
    copyText: () => rowsToLines(rendered.rows).join("\n"),
    draw: (appearance) => draw(e, block, warnings, width, appearance),
  };
  return e.body;
}

function noticeLine(theme: Theme, { text, tone }: NonNullable<Entry["notice"]>): string {
  return theme.fg("accent", "▸ ") + theme.fg(tone === "error" ? "error" : "muted", text);
}

function draw(
  e: Entry,
  block: Block,
  warnings: string[],
  width: number,
  { hovered, control }: CodeBlockBodyAppearance<Target>,
) {
  const notes = e.state.noteMode;
  // Each cell belongs to the cached layout. Paint by replacing cells in a copy of the rows.
  const rows: Cell[][] = block.rows.map((r) => r.slice());
  const paintHover = (t: Target | undefined) => {
    if (!t) return;
    if (t.kind === "item" && t.item?.key === "") return;
    if (t.kind === "image" && t.image) {
      // The picture itself cannot be styled: light up the frame around it.
      const r = t.rect;
      for (let y = r.y - 1; y <= r.y + r.h; y++)
        for (let x = r.x - 1; x <= r.x + r.w; x++) {
          const onRing = y === r.y - 1 || y === r.y + r.h || x === r.x - 1 || x === r.x + r.w;
          if (onRing && rows[y]?.[x]) markHovered(rows[y], x);
        }
      return;
    }
    for (let y = t.rect.y; y < t.rect.y + t.rect.h; y++)
      for (let x = Math.max(0, t.rect.x); x < Math.min(block.w, t.rect.x + t.rect.w); x++)
        if (rows[y]?.[x]) markHovered(rows[y], x);
  };
  paintHover(hovered);
  if (hovered?.tip?.hint && !notes) drawBubble(rows, block.w, hovered);

  const footerCells = (): Cell[] => {
    if (e.notice)
      return [
        ...textCells("▸ ", "foot.mark"),
        ...textCells(e.notice.text, e.notice.tone === "error" ? "action.error" : "foot.text"),
      ];
    if (control) return [...textCells("▸ ", "foot.mark"), ...textCells(control.label, "foot.text")];
    if (hovered)
      return [...textCells("▸ ", "foot.mark"), ...textCells(describe(hovered, !!overlayOf(e), notes), "foot.text")];
    if (notes) return textCells("Click an element to add a note.", "foot.text");
    const tips = new Set(block.targets.flatMap((x) => (x.tip ? [x.tip] : []))).size;
    if (tips)
      return [
        ...textCells(BADGE, "foot.mark"),
        ...textCells(` ${tips} tooltip${tips === 1 ? "" : "s"}: hover for hints`, "foot.text"),
      ];
    return [];
  };
  const chipT = block.targets.find((t) => t.kind === "notes");
  const text = footerCells();
  if (chipT && rows[chipT.rect.y]) {
    const room = chipT.rect.x - 2;
    const cells = textCells(truncateText(text.map((c) => c.ch).join(""), Math.max(0, room)), "foot.text");
    text.forEach((c, i) => {
      if (cells[i]) cells[i] = cell(cells[i].ch, c.role);
    });
    cells.forEach((c, i) => {
      if (i < rows[chipT.rect.y].length) rows[chipT.rect.y][i] = c;
    });
  } else if (text.length) rows.push(padRow(text, Math.min(width, text.length)));

  const th = e.theme;
  const palette = (e.palette ??= themePalette(th));
  const lines = rowsToLines(rows, palette);
  const imageRows = new Set<number>();
  for (const t of block.targets) {
    if (t.kind !== "image" || !t.image) continue;
    const seq = imageService.sequence(t.image.key);
    const row = rows[t.rect.y];
    if (!seq || !row || t.rect.x < 0 || t.rect.x + t.image.cols > width) continue;
    const left = rowsToLines([row.slice(0, t.rect.x)], palette, false)[0] ?? "";
    const rightX = t.rect.x + t.image.cols;
    const right = truncateToWidth(rowsToLines([row.slice(rightX)], palette)[0] ?? "", width - rightX, "…", true);
    // Kitty leaves the cursor at the image's origin: step over it before drawing the rest of the row.
    lines[t.rect.y] = left + seq + `\x1b[${t.image.cols}C` + right;
    imageRows.add(t.rect.y);
  }
  for (const warning of warnings) lines.push(th.fg("warning", `! ${warning}`));
  return { lines, imageRows };
}

export class OpenUIWidget {
  private readonly entry: Entry;

  constructor(
    private readonly id: string,
    root: El,
    host: WidgetHost,
    destination: WidgetDestination,
    source: string,
  ) {
    this.entry = getEntry(id, root, host, destination, source);
  }

  render(width: number): string[] {
    return this.entry.frame.render(width);
  }

  copyRenderedText(): string | undefined {
    return this.entry.frame.copyRenderedText();
  }

  handleMouse(event: TuiMouseEvent) {
    if (this.entry.dialog) return undefined;
    return this.entry.frame.handleMouse(event);
  }

  dispose(): void {
    if (entries.get(this.id) === this.entry) releaseEntry(this.id, this.entry);
  }

  invalidate(): void {
    const e = this.entry;
    e.palette = undefined;
    e.frame.invalidate();
    if (!e.block?.targets.some((target) => target.kind === "image")) return;
    imageService.reset();
    e.version++;
  }
}

function releaseEntry(id: string, e: Entry) {
  entries.delete(id);
  e.frame.dispose();
  e.dialog?.abort();
  e.dialog = null;
  if (e.noticeTimer) clearTimeout(e.noticeTimer);
  for (const pending of e.pending.values()) if (pending.timer) clearTimeout(pending.timer);
  for (const timer of e.feedbackTimers.values()) clearTimeout(timer);
}

/** Release widget state when the session ends. */
export function resetAll() {
  for (const [id, e] of entries) releaseEntry(id, e);
  imageService.reset();
}

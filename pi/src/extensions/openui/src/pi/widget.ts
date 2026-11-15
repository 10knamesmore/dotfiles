// pi-tui component for a compiled OpenUI tree: themed colours, hover, click, keyboard, clipboard, pictures, notes.
import { Key, matchesKey, truncateToWidth } from "@earendil-works/pi-tui";
import { copyToClipboard } from "@earendil-works/pi-coding-agent";
import { ACTION_STYLES, COPIED_LABEL, COPY_LABEL, FEEDBACK_DURATION_MS } from "@dotfiles/pi-ui/action-feedback";
import { drawBubble } from "../blocks/bubble.ts";
import { BADGE } from "../blocks/core.ts";
import {
  activate,
  describe,
  nextTarget,
  readingOrder,
  reveal,
  targetAt,
  type Env,
  type NavKey,
} from "../blocks/interact.ts";
import { otherLength, setOther } from "../blocks/choice.ts";
import { buildMessage, cleanNote, NOTE_LIMITS, widgetTitle } from "../blocks/notes.ts";
import { renderTree } from "../blocks/render.ts";
import { codeBlock } from "../blocks/code.ts";
import { newState, type Block, type Note, type Target, type WState } from "../blocks/types.ts";
import { blank, cell, fromRows, padRow, textCells, vstack } from "../blocks/util.ts";
import { rowsToLines, textWidth, truncateText, type Cell } from "../diagram/canvas.ts";
import { createPalette } from "../diagram/palette.ts";
import { LIMITS, type El } from "../lang/library.ts";
import { imageService } from "./images.ts";

const GUTTER = 2;
/**
 * The last column of every line is left empty. pi's fullscreen transcript paints its scrollbar over that column after the
 * component has drawn (always with `fullscreenScrollbar: "always"`, while scrolling by default), so anything there is hidden:
 * the notes chip lost its `]`, a card its right border, a chart its last axis label. pi's own text keeps a margin for the same
 * reason.
 */
const RIGHT = 1;
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

export type Mode = "live" | "echo";

interface Entry {
  root: El;
  source: string;
  collapsed: boolean;
  sourceOpen: boolean;
  state: WState;
  mode: Mode;
  block: Block | null;
  key: string;
  version: number;
  warnings: string[];
  notice: { text: string; tone: "info" | "error" } | null;
  noticeTimer: ReturnType<typeof setTimeout> | null;
  /** Messages awaiting Pi's acceptance, keyed by the button or answer bar that sent them. */
  pending: Map<string, { text: string; timer: ReturnType<typeof setTimeout> | null }>;
  feedbackTimers: Map<string, ReturnType<typeof setTimeout>>;
  /** the finished lines and what they were drawn from: pi asks for them on every frame, so an unchanged widget answers from here */
  lines: string[] | null;
  lineKey: string;
}

const entries = new Map<string, Entry>();
export const active: { id: string | null } = { id: null };
/** A dialog (the note editor) is open: our key and mouse handlers stand down so its keys are its own. */
export const dialog = { open: false };

/** Services supplied by the Pi adapter. */
export const hooks = {
  requestRender: () => {},
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

export const entryOf = (id: string): Entry | undefined => entries.get(id);

// pi converts JPEG, GIF and WebP lazily after startup: when a picture becomes drawable, lay every widget out again
imageService.onLate = () => {
  for (const e of entries.values()) e.version++;
  hooks.requestRender();
};

function getEntry(id: string, root: El, mode: Mode, source: string): Entry {
  let e = entries.get(id);
  if (!e) {
    e = {
      root,
      source,
      collapsed: false,
      sourceOpen: false,
      state: newState(),
      mode,
      block: null,
      key: "",
      version: 0,
      warnings: [],
      notice: null,
      noticeTimer: null,
      pending: new Map(),
      feedbackTimers: new Map(),
      lines: null,
      lineKey: "",
    };
    entries.set(id, e);
  }
  return e;
}

function envFor(id: string, e: Entry): Env {
  return {
    send: (t, o) => {
      const targetId = o?.targetId ?? "send";
      if (e.mode !== "live") {
        hooks.sendEcho(t);
        if (o?.targetId) markDelivered(e, targetId);
        return;
      }
      if (o?.confirm) watchDelivery(id, e, t, targetId);
      hooks.sendLive(t);
      if (e.pending.has(targetId) && hooks.queued()) {
        e.state.delivery[targetId] = "queued";
        e.version++;
      }
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
        hooks.requestRender();
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
      hooks.requestRender();
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
  hooks.requestRender();
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
  hooks.requestRender();
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
      hooks.requestRender();
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
    hooks.requestRender();
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

function setMode(id: string, e: Entry, on: boolean) {
  e.state.noteMode = on;
  e.state.focus = null;
  e.state.hover = null;
  e.state.tip = null;
  e.version++;
  if (on) active.id = id;
  setNotice(id, null);
  hooks.requestRender();
}

async function writeNote(id: string, e: Entry, t: Target): Promise<void> {
  if (dialog.open || !t.item) return;
  const existing = e.state.notes.find((n) => n.itemId === t.id);
  if (!existing && e.state.notes.length >= NOTE_LIMITS.notes) {
    setNotice(id, `${NOTE_LIMITS.notes} notes is the most per widget.`);
    hooks.requestRender();
    return;
  }
  await ask(id, e, existing ?? null, { itemId: t.id, info: t.item });
}

async function editNote(id: string, e: Entry, noteId: number | undefined): Promise<void> {
  const n = e.state.notes.find((x) => x.id === noteId);
  if (n && !dialog.open) await ask(id, e, n, { itemId: n.itemId, info: n.info });
}

async function ask(
  id: string,
  e: Entry,
  existing: Note | null,
  about: { itemId: string; info: Note["info"] },
): Promise<void> {
  dialog.open = true;
  let text: string | undefined;
  try {
    text = await hooks.prompt(
      `Note on ${about.info.short}${about.info.tab ? ` [tab ${about.info.tab}]` : ""}${existing ? " (editing)" : ""}`,
      existing?.text ?? "",
    );
  } finally {
    dialog.open = false;
  }
  if (text === undefined) return; // cancelled
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
  hooks.requestRender();
}

function addToMessage(id: string, e: Entry) {
  const n = e.state.notes.length;
  if (!n) return;
  hooks.toEditor(buildMessage(e.state.notes, widgetTitle(e.root)));
  e.state.notes = [];
  e.state.noteMode = false;
  e.state.focus = null;
  // The next key is Enter, to send the message the user is about to read. It has to reach pi's editor, not this widget.
  if (active.id === id) active.id = null;
  e.version++;
  setNotice(id, `Added ${n} note${n === 1 ? "" : "s"} to your message. Read it, edit it, then press Enter to send.`);
  hooks.requestRender();
}

function clearNotes(id: string, e: Entry) {
  e.state.notes = [];
  e.state.focus = null;
  e.version++;
  setNotice(id, "Notes cleared.");
  hooks.requestRender();
}

// ---------- choices ----------

/**
 * The Other row of a Choice. The user types their own answer in pi's editor (the one Notes uses), never inside the widget: an
 * armed widget must not keep letters. Saved text selects Other, empty text unselects it, Esc changes nothing.
 */
async function editOther(id: string, e: Entry, t: Target): Promise<void> {
  const x = t.choice;
  if (!x || dialog.open || e.state.sent) return;
  dialog.open = true;
  let text: string | undefined;
  try {
    text = await hooks.prompt(
      `Your own answer: ${truncateText(x.of.question, 60)}`,
      e.state.answers[x.of.key]?.text ?? "",
    );
  } finally {
    dialog.open = false;
  }
  if (text === undefined || e.state.sent) return; // cancelled (the sent check is defence: nothing can send while the editor owns the keys)
  if (otherLength(text) > LIMITS.other) setNotice(id, `Answer shortened to ${LIMITS.other} characters.`);
  e.state.answers = setOther(e.state.answers, x.of, text);
  e.version++;
  hooks.requestRender();
}

/** One place for what a click or Enter does, so the mouse and the keyboard cannot drift apart. */
function perform(id: string, e: Entry, t: Target, shift = false): void {
  switch (t.kind) {
    case "fold":
      e.collapsed = !e.collapsed;
      e.state.hover = null;
      e.state.focus = t.id;
      e.state.noteMode = false;
      e.version++;
      return;
    case "source":
      e.sourceOpen = !e.sourceOpen;
      e.collapsed = false;
      e.state.hover = null;
      e.state.focus = t.id;
      e.state.noteMode = false;
      e.version++;
      return;
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
        void editOther(id, e, t).catch(() => {
          // the editor could not open (pi's context went stale): leave the answers as they were
          setNotice(id, "The editor could not be opened.");
          hooks.requestRender();
        });
        return;
      }
      break;
  }
  if (activate(t, e.state, envFor(id, e))) e.version++;
  // The button that was pressed is gone from the next picture. Focus must not be left pointing at it: Enter would find nothing
  // and fall through to the first element of the widget, and that can be a Button that sends a message.
  if (t.kind === "send" && e.state.sent) {
    // a locked widget has nothing left to hold the keyboard for: Enter is pi's again, to send the user's own message
    e.state.focus = null;
    if (active.id === id) active.id = null;
  } else if (t.kind === "clear" && e.block)
    e.state.focus = readingOrder(e.block).find((x) => x.kind === "choice")?.id ?? null;
}

// ---------- the component ----------

/** Restyle one cell for hover or focus. Replaces the cell: the originals belong to the cached layout and are shared by every redraw. */
const mark = (row: Cell[], x: number, suffix: string) => {
  const c = row[x];
  if (c.role.startsWith("note.")) return; // a tooltip badge or a pin keeps its own colour
  if (c.role) row[x] = cell(c.ch, `${c.role}.${suffix}`);
  else if (c.ch.trim()) row[x] = cell(c.ch, suffix);
};

export class OpenUIWidget {
  id: string;
  theme: any;
  private pal: ReturnType<typeof themePalette> | null = null;
  private root: El;
  private mode: Mode;
  private source: string;
  constructor(id: string, root: El, theme: any, mode: Mode, source: string) {
    this.id = id;
    this.theme = theme;
    this.root = root;
    this.mode = mode;
    this.source = source;
    getEntry(id, root, mode, source);
  }

  private header(e: Entry, width: number): Block {
    const controls: Array<{ text: string; role: string; target: Omit<Target, "rect"> }> = [
      {
        text: e.sourceOpen ? "-[widget]-" : "-[source]-",
        role: "action",
        target: { id: "widget:source", kind: "source", label: e.sourceOpen ? "show widget" : "view OpenUI source" },
      },
      {
        text: e.state.copied === "copy:source" ? COPIED_LABEL : COPY_LABEL,
        role: e.state.copied === "copy:source" ? "action.success" : "action",
        target: { id: "copy:source", kind: "copy", label: "copy OpenUI source", copy: e.source },
      },
      {
        text: e.collapsed ? "-[show]-" : "-[hide]-",
        role: "action",
        target: { id: "widget:fold", kind: "fold", label: e.collapsed ? "expand widget" : "collapse widget" },
      },
    ];
    const controlsWidth = controls.reduce((n, c) => n + textWidth(c.text), 0) + controls.length - 1;
    const inline = width >= controlsWidth + 12;
    const title = truncateText(` OpenUI · ${widgetTitle(e.root)} `, inline ? width - controlsWidth - 3 : width - 2);
    const rule = "─ ".repeat(Math.ceil(width / 2)).slice(0, width - 2);
    const rows = [textCells(`╭${rule}╮`, "widget.border")];
    textCells(title, "widget.border").forEach((c, i) => {
      rows[0][i + 1] = c;
    });
    const targets: Target[] = [];
    let x = inline ? width - controlsWidth - 1 : 1;
    let y = inline ? 0 : 1;
    for (const control of controls) {
      const cells = textCells(control.text, control.role);
      if (x + cells.length > width - 1) {
        x = 1;
        y++;
      }
      rows[y] ??= padRow([], width);
      if (x > 1) rows[y][x - 1] = cell(" ");
      cells.forEach((c, i) => {
        rows[y][x + i] = c;
      });
      targets.push({ ...control.target, rect: { x, y, w: cells.length, h: 1 } });
      x += cells.length + 1;
    }
    return fromRows(width, rows, targets);
  }

  private layout(width: number): Entry {
    // the entry is made again if the session's state was dropped while this widget was still on screen
    const e = getEntry(this.id, this.root, this.mode, this.source);
    const key = `${width}:${e.version}`;
    if (!e.block || e.key !== key) {
      const r = e.collapsed
        ? { block: blank(width, 0), warnings: [] }
        : e.sourceOpen
          ? { block: codeBlock(e.source, undefined, "OpenUI source", width, null, false), warnings: [] }
          : renderTree(e.root, width, e.state, { images: imageService });
      let block = r.block;
      // a footer row with the notes chip, when there is anything to attach a note to
      if (block.targets.some((t) => t.kind === "item")) {
        const w = Math.max(block.w, width);
        const label = chipLabel(e.state);
        const cw = textWidth(label);
        const chip: Cell[] = textCells(label, e.state.noteMode ? "notes.on" : "btn");
        const row = padRow([], w - cw).concat(chip);
        block = {
          w,
          rows: [...block.rows.map((x) => padRow(x, w)), row],
          targets: [
            ...block.targets,
            { id: CHIP, kind: "notes", rect: { x: w - cw, y: block.rows.length, w: cw, h: 1 }, label: "notes" },
          ],
        };
      }
      e.block = vstack([this.header(e, width), block], 0, Math.max(width, block.w));
      e.warnings = r.warnings;
      e.key = key;
    }
    return e;
  }

  /** the block as last laid out, for tests and the keyboard layer */
  lastBlock(): Block | null {
    return entries.get(this.id)?.block ?? null;
  }

  /**
   * pi calls this for every widget in the transcript on every frame (up to 60 a second, and again for each mouse event), so
   * the work is done once per distinct appearance. What changes the picture is in the key: the layout (width and version), the
   * pointer and keyboard focus, whether the widget is armed or in notes mode, the notice and an open tooltip. It never throws:
   * pi's render loop is not a place for an exception.
   */
  render(width: number): string[] {
    try {
      const avail = Math.max(24, width - GUTTER - RIGHT);
      const e = this.layout(avail);
      const s = e.state;
      const key = `${e.key}|${s.hover}|${s.focus}|${s.tip}|${s.copied}|${s.noteMode}|${s.notes.length}|${e.notice?.text}|${e.notice?.tone}|${active.id === this.id}`;
      if (e.lines && e.lineKey === key) return e.lines;
      const lines = this.draw(e, width, avail);
      e.lines = lines;
      e.lineKey = key;
      return lines;
    } catch (err) {
      const msg = String((err as Error)?.message ?? err)
        .replace(/\s+/g, " ")
        .slice(0, 120);
      return [truncateToWidth(this.theme.fg("error", `! the widget could not be drawn: ${msg}`), Math.max(1, width))];
    }
  }

  private draw(e: Entry, width: number, avail: number): string[] {
    const block = e.block!;
    const notes = e.state.noteMode;
    // the cells themselves are shared with the layout: a cell is replaced, never changed (see `mark`)
    const rows: Cell[][] = block.rows.map((r) => r.slice());
    const paint = (id: string | null, suffix: string) => {
      if (!id) return;
      const t = block.targets.find((x) => x.id === id);
      if (!t) return;
      if (t.kind === "item" && t.item?.key === "") return;
      if (t.kind === "image" && t.image) {
        // the picture itself cannot be styled: light up the frame around it
        const r = t.rect;
        for (let y = r.y - 1; y <= r.y + r.h; y++)
          for (let x = r.x - 1; x <= r.x + r.w; x++) {
            const onRing = y === r.y - 1 || y === r.y + r.h || x === r.x - 1 || x === r.x + r.w;
            if (onRing && rows[y]?.[x]) mark(rows[y], x, suffix);
          }
        return;
      }
      for (let y = t.rect.y; y < t.rect.y + t.rect.h; y++)
        for (let x = Math.max(0, t.rect.x); x < Math.min(block.w, t.rect.x + t.rect.w); x++)
          if (rows[y]?.[x]) mark(rows[y], x, suffix);
    };
    paint(e.state.hover, "hover");
    paint(e.state.focus, "focus");
    const current = block.targets.find((x) => x.id === (e.state.hover ?? e.state.focus));
    if (current?.tip?.hint && !notes) drawBubble(rows, block.w, current);

    // the footer: what the pointer or focus is on, a notice, or how to reach tooltips and notes
    const footerCells = (): Cell[] => {
      const tips = new Set(block.targets.flatMap((x) => (x.tip ? [x.tip] : []))).size;
      if (e.notice)
        return [
          ...textCells("▸ ", "foot.mark"),
          ...textCells(e.notice.text, e.notice.tone === "error" ? "action.error" : "foot.text"),
        ];
      if (current) return [...textCells("▸ ", "foot.mark"), ...textCells(describe(current), "foot.text")];
      if (notes)
        return [...textCells("Notes: click an element, or Tab to it and press Enter. Esc leaves.", "foot.text")];
      if (tips)
        return [
          ...textCells(BADGE, "foot.mark"),
          ...textCells(
            ` ${tips} element${tips === 1 ? " has a" : "s have"} tooltip${tips === 1 ? "" : "s"}: hover ${tips === 1 ? "it" : "one"}, or press Tab then Enter`,
            "foot.text",
          ),
        ];
      return [];
    };
    const chipT = block.targets.find((t) => t.kind === "notes");
    const text = footerCells();
    if (chipT && rows[chipT.rect.y]) {
      const room = chipT.rect.x - 2;
      const cells = textCells(truncateText(text.map((c) => c.ch).join(""), Math.max(0, room)), "foot.text");
      text.forEach((c, i) => {
        if (cells[i]) cells[i] = cell(cells[i].ch, c.role); // cells are shared: build another, never change one
      });
      cells.forEach((c, i) => {
        if (i < rows[chipT.rect.y].length) rows[chipT.rect.y][i] = c;
      });
    } else if (text.length && (!e.collapsed || e.notice))
      rows.push(padRow(text, Math.min(avail, Math.max(text.length, 1))));

    const th = this.theme;
    const basePalette = (this.pal ??= themePalette(th));
    const borderStyle = { fg: active.id === this.id ? "accent" : "dim", dim: active.id !== this.id };
    const palette: typeof basePalette = (role) =>
      role === "widget.border" ? (text) => th.style(text, borderStyle) : basePalette(role);
    const lines = rowsToLines(rows, palette);
    // pictures: the rows are reserved and framed; the first row of each carries the terminal's image command
    const imageRows = new Set<number>();
    for (const t of block.targets) {
      if (t.kind !== "image" || !t.image) continue;
      const seq = imageService.sequence(t.image.key);
      const row = rows[t.rect.y];
      if (!seq || !row) continue;
      const left = rowsToLines([row.slice(0, t.rect.x)], palette, false)[0] ?? "";
      const right = rowsToLines([row.slice(t.rect.x + t.image.cols)], palette)[0] ?? "";
      // Kitty places the picture at the cursor and leaves the cursor there: step over it before the rest of the row
      lines[t.rect.y] = left + seq + `\x1b[${t.image.cols}C` + right;
      imageRows.add(t.rect.y);
    }
    for (const w of e.warnings) lines.push(th.fg("warning", `! ${w}`));
    const indent = " ".repeat(GUTTER);
    const content = lines.map((l, i) => (imageRows.has(i) ? indent + l : truncateToWidth(indent + l, width)));
    const ruleWidth = Math.max(0, width - GUTTER - RIGHT - 2);
    const rule = "─ ".repeat(Math.ceil(ruleWidth / 2)).slice(0, ruleWidth);
    return e.collapsed ? content : [...content, indent + th.style(`╰${rule}╯`, borderStyle)];
  }

  handleMouse(ev: any) {
    try {
      return this.mouse(ev);
    } catch {
      return undefined; // pi's input handling is not a place for an exception either
    }
  }

  private mouse(ev: any) {
    const e = entries.get(this.id);
    if (!e || !e.block || dialog.open) return undefined;
    const t = targetAt(e.block, ev.x - GUTTER, ev.y, e.state.noteMode);
    if (ev.type === "move") {
      const id = t?.id ?? null;
      if (id === e.state.hover) return undefined;
      e.state.hover = id;
      return { handled: !!t, render: true };
    }
    if (ev.type === "click" && ev.button === "left" && t) {
      active.id = this.id;
      e.state.focus = t.id;
      setNotice(this.id, null);
      perform(this.id, e, t, !!ev.shift);
      if (reveal(t, e.state)) e.version++;
      hooks.requestRender();
      return { handled: true, render: true };
    }
    return undefined;
  }

  /** pi calls this when the theme, the cell size or a converter changes: the colours (and a picture's size) are stale. */
  invalidate() {
    this.pal = null;
    const e = entries.get(this.id);
    if (!e) return;
    e.lines = null;
    if (e.block?.targets.some((t) => t.kind === "image")) {
      imageService.reset();
      e.version++;
    }
  }
}

/** Keyboard layer while a widget is armed. Returns true when the key was used. Never throws. */
export function handleKey(data: string): boolean {
  try {
    return keyInner(data);
  } catch {
    return false;
  }
}

function keyInner(data: string): boolean {
  if (!active.id || dialog.open) return false;
  const id = active.id;
  const e = entries.get(id);
  if (!e || !e.block) return false;
  const block = e.block;
  const notes = e.state.noteMode;
  if (matchesKey(data, Key.escape)) {
    if (e.state.tip) {
      e.state.tip = null;
      e.version++;
    } else if (notes) {
      setMode(id, e, false);
      return true;
    } else {
      active.id = null;
      e.state.focus = null;
    }
    hooks.requestRender();
    return true;
  }
  let nav: NavKey | null = null;
  if (matchesKey(data, Key.tab)) nav = "next";
  else if (matchesKey(data, Key.shift("tab"))) nav = "prev";
  else if (matchesKey(data, Key.left)) nav = "left";
  else if (matchesKey(data, Key.right)) nav = "right";
  else if (matchesKey(data, Key.up)) nav = "up";
  else if (matchesKey(data, Key.down)) nav = "down";
  if (nav) {
    const t = nextTarget(block, e.state.focus, nav, notes);
    if (!t) return false;
    e.state.focus = t.id;
    e.state.hover = null; // the keyboard is in charge now: a stale pointer hover must not describe another target
    setNotice(id, null);
    if (reveal(t, e.state)) e.version++;
    hooks.requestRender();
    return true;
  }
  if (matchesKey(data, Key.enter) || matchesKey(data, Key.shift("enter"))) {
    const focused = e.state.focus ? block.targets.find((x) => x.id === e.state.focus) : undefined;
    if (e.state.focus && !focused) {
      // the focused element is gone (a button removes itself when it is pressed): never press another one in its place
      e.state.focus = null;
      e.state.hover = null;
      hooks.requestRender();
      return true;
    }
    const t = focused ?? nextTarget(block, null, "next", notes);
    if (!t) return false;
    e.state.focus = t.id;
    e.state.hover = null;
    perform(id, e, t, matchesKey(data, Key.shift("enter")));
    hooks.requestRender();
    return true;
  }
  // Delete (or Backspace) on a note in the tray removes it
  if (matchesKey(data, Key.delete) || matchesKey(data, Key.backspace)) {
    const t = block.targets.find((x) => x.id === e.state.focus);
    if (t?.kind === "note") {
      e.state.notes = e.state.notes.filter((n) => n.id !== t.noteId);
      e.state.focus = null;
      e.version++;
      hooks.requestRender();
      return true;
    }
  }
  // The user started typing (a letter, a space, a paste): the keyboard belongs to the editor again. Without this, Enter
  // would keep re-firing the focused button instead of sending the message being typed.
  if ([...data].some((ch) => ch >= " " && ch !== "\x7f")) {
    active.id = null;
    e.state.focus = null;
    e.state.hover = null;
    if (e.state.noteMode) {
      e.state.noteMode = false;
      e.version++;
    }
    hooks.requestRender();
  }
  return false;
}

/** Forget every widget's state (notes, focus, layouts). Called when a session ends: pi processes live for days. */
export function resetAll() {
  for (const e of entries.values()) {
    if (e.noticeTimer) clearTimeout(e.noticeTimer);
    for (const pending of e.pending.values()) if (pending.timer) clearTimeout(pending.timer);
    for (const timer of e.feedbackTimers.values()) clearTimeout(timer);
  }
  entries.clear();
  imageService.reset();
  active.id = null;
  dialog.open = false;
}

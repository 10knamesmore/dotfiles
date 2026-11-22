// Interaction logic, independent of any terminal: hit testing, activation, keyboard navigation, pan reveal.
import { buildMessage, progress, toggleOption } from "./choice.ts";
import type { Block, Target, WState } from "./types.ts";

export interface Env {
  /** send text to the assistant as the user's next turn. `confirm`: the sender will lock something until the host says it took the text */
  send(text: string, opts?: { confirm?: boolean; targetId?: string }): void;
  /** Write to the clipboard; the host updates feedback after the write completes. */
  copy(text: string, targetId: string): void;
  /** show a short notice under the widget */
  note(text: string): void;
}

const area = (t: Target) => t.rect.w * t.rect.h;
export const WIDGET_COPY_ID = "widget:copy";

/**
 * Which targets answer to the mouse and keyboard. Outside notes mode everything except note items. In notes mode: the note
 * items, the notes tray, and tabs (so you can reach content on another tab). Buttons, choices and the like do not fire.
 */
export function isLive(t: Target, notes: boolean): boolean {
  if (!notes) return t.kind !== "item";
  return (
    t.kind === "item" ||
    t.kind === "tab" ||
    t.kind === "note" ||
    t.kind === "noteadd" ||
    t.kind === "noteclear" ||
    t.kind === "notes" ||
    t.kind === "fold" ||
    t.kind === "source" ||
    t.id === WIDGET_COPY_ID
  );
}

/** The most specific target at a point: an exact cell match (a pie wedge) beats a rectangle, a small rectangle beats a big one. */
export function targetAt(block: Block, x: number, y: number, notes = false): Target | null {
  let best: Target | null = null;
  let bestScore = Infinity;
  for (const t of block.targets) {
    if (!isLive(t, notes)) continue;
    const r = t.rect;
    const inRect = x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h && r.x + r.w > 0 && r.x < block.w;
    const inCells = !!t.cells && t.cells.some(([cx, cy]) => cx === x && cy === y);
    if (!inRect && !inCells) continue;
    const score = inCells ? 0 : area(t);
    if (score < bestScore) {
      bestScore = score;
      best = t;
    }
  }
  return best;
}

export function readingOrder(block: Block, notes = false): Target[] {
  return block.targets
    .filter((t) => isLive(t, notes))
    .sort((a, b) => a.rect.y - b.rect.y || a.rect.x - b.rect.x || a.id.localeCompare(b.id));
}

export type NavKey = "next" | "prev" | "left" | "right" | "up" | "down";

/** The neighbour of `cur` in a direction among `order`, or `cur` when there is none. Rectangles must be separated along the axis. */
function spatial(order: Target[], cur: Target, key: "left" | "right" | "up" | "down"): Target {
  const cx = (t: Target) => t.rect.x + t.rect.w / 2;
  const cy = (t: Target) => t.rect.y + t.rect.h / 2;
  const horizontal = key === "left" || key === "right";
  // "right of" means the rectangles are separated along that axis: a full-width row below is below, not to the right
  const inDirection = order.filter((t) => {
    if (t === cur) return false;
    const r = t.rect;
    const c = cur.rect;
    return key === "left"
      ? r.x + r.w <= c.x
      : key === "right"
        ? r.x >= c.x + c.w
        : key === "up"
          ? r.y + r.h <= c.y
          : r.y >= c.y + c.h;
  });
  // targets that share the row (sideways) or the column (up and down) win over anything nearer on another line
  const overlaps = (t: Target) =>
    horizontal
      ? t.rect.y < cur.rect.y + cur.rect.h && cur.rect.y < t.rect.y + t.rect.h
      : t.rect.x < cur.rect.x + cur.rect.w && cur.rect.x < t.rect.x + t.rect.w;
  const aligned = inDirection.filter(overlaps);
  // a target that has neighbours on its row stops at the end of the row instead of jumping to another line. Neighbours means
  // sharing most of the row (a row of buttons), not touching one line of a tall box (a graph node).
  const shared = (t: Target) => {
    const a0 = horizontal ? t.rect.y : t.rect.x;
    const a1 = a0 + (horizontal ? t.rect.h : t.rect.w);
    const b0 = horizontal ? cur.rect.y : cur.rect.x;
    const b1 = b0 + (horizontal ? cur.rect.h : cur.rect.w);
    return Math.min(a1, b1) - Math.max(a0, b0) >= 0.5 * Math.min(a1 - a0, b1 - b0);
  };
  const hasRowMates = order.some((t) => t !== cur && shared(t));
  if (!aligned.length && hasRowMates) return cur;
  const pool = aligned.length ? aligned : inDirection;
  let best: Target | null = null;
  let bestScore = Infinity;
  for (const t of pool) {
    const along = Math.abs(horizontal ? cx(t) - cx(cur) : cy(t) - cy(cur));
    const across = Math.abs(horizontal ? cy(t) - cy(cur) : cx(t) - cx(cur));
    // cells are about twice as tall as wide, so a line of difference weighs more than a column
    const score = along + across * (horizontal ? 3 : 2);
    if (score < bestScore) {
      bestScore = score;
      best = t;
    }
  }
  return best ?? cur;
}

/**
 * Notes mode. Tab hops between components (and the tray, and tabs). Arrows move among the parts of the component you are in:
 * from the whole component they step into its first or last part; past the last part they move on to the next component.
 */
function notesNav(pool: Target[], currentId: string | null, key: NavKey): Target | null {
  const order = [...pool].sort((a, b) => a.rect.y - b.rect.y || a.rect.x - b.rect.x || a.id.localeCompare(b.id));
  // one stop per component: its whole item; plus the tray rows, buttons and tabs
  const ring = order.filter((t) => t.kind !== "item" || t.item?.key === "");
  if (!ring.length) return null;
  const cur = order.find((t) => t.id === currentId) ?? null;
  const ringStop = (t: Target): Target =>
    t.kind === "item" && t.item!.key !== ""
      ? (ring.find((r) => r.kind === "item" && r.item!.host === t.item!.host && r.item!.key === "") ?? ring[0])
      : t;
  const step = (from: Target, d: 1 | -1): Target => {
    const stop = ringStop(from);
    const i = ring.indexOf(stop);
    return ring[(i + d + ring.length) % ring.length];
  };
  if (!cur) return key === "prev" || key === "up" || key === "left" ? ring[ring.length - 1] : ring[0];
  if (key === "next") return step(cur, 1);
  if (key === "prev") return step(cur, -1);
  const fwd = key === "down" || key === "right";
  if (cur.kind === "item") {
    const leaves = order.filter((t) => t.kind === "item" && t.item!.host === cur.item!.host && t.item!.key !== "");
    if (cur.item!.key === "") {
      if (!leaves.length) return step(cur, fwd ? 1 : -1);
      return fwd ? leaves[0] : leaves[leaves.length - 1];
    }
    const to = spatial(leaves, cur, key);
    if (to !== cur) return to;
    return step(cur, fwd ? 1 : -1);
  }
  // tray rows, its buttons, tabs, the chip. Up and down go to the next line in reading order and land on its first stop (so
  // down from a note reaches "Add", never "Clear"); left and right move along the line.
  if (key === "up" || key === "down") {
    const below = key === "down";
    const lines = ring.filter((t) => (below ? t.rect.y >= cur.rect.y + cur.rect.h : t.rect.y + t.rect.h <= cur.rect.y));
    if (!lines.length) return step(cur, below ? 1 : -1);
    const y = below ? Math.min(...lines.map((t) => t.rect.y)) : Math.max(...lines.map((t) => t.rect.y));
    return lines.filter((t) => t.rect.y === y).sort((a, b) => a.rect.x - b.rect.x)[0];
  }
  const same = order.filter((t) => t.kind !== "item");
  return spatial(same, cur, key);
}

export function nextTarget(block: Block, currentId: string | null, key: NavKey, notes = false): Target | null {
  const pool = block.targets.filter((t) => isLive(t, notes));
  if (notes) return notesNav(pool, currentId, key);
  const order = [...pool].sort((a, b) => a.rect.y - b.rect.y || a.rect.x - b.rect.x || a.id.localeCompare(b.id));
  if (!order.length) return null;
  const cur = order.find((t) => t.id === currentId) ?? null;
  if (key === "next" || key === "prev") {
    if (!cur) return key === "next" ? order[0] : order[order.length - 1];
    const i = order.indexOf(cur);
    return order[(i + (key === "next" ? 1 : -1) + order.length) % order.length];
  }
  if (!cur) return order[0];
  const to = spatial(order, cur, key);
  // Down from the options reaches Send answers. The nearest centre would be Clear, and one Enter on it drops every answer.
  if ((key === "down" || key === "up") && to.kind === "clear" && cur.kind !== "send" && cur.kind !== "clear")
    return order.find((t) => t.kind === "send") ?? to;
  return to;
}

const lastColon = (id: string) => id.lastIndexOf(":");

/** Apply a target's action. Returns true when the layout changed and the block must be rebuilt. */
export function activate(t: Target, state: WState, env: Env): boolean {
  switch (t.kind) {
    case "sort": {
      const sort = t.sort!;
      const current = state.tableSort[sort.table];
      state.tableSort[sort.table] = {
        column: sort.column,
        direction: current?.column === sort.column && current.direction === "asc" ? "desc" : "asc",
      };
      return true;
    }
    case "cell":
      state.tip = state.tip === t.id ? null : t.id;
      return true;
    case "tab": {
      const k = lastColon(t.id);
      const path = t.id.slice(4, k);
      const i = Number(t.id.slice(k + 1));
      if (state.tab[path] === i) return false;
      state.tab[path] = i;
      state.tip = null;
      return true;
    }
    case "faq":
      state.faq[t.id] = !state.faq[t.id];
      return true;
    case "choice": {
      // a click only stages the answer. The Other row opens pi's editor, which the widget handles (it is asynchronous).
      const x = t.choice;
      if (!x || state.sent || x.index === "other") return false;
      state.answers = toggleOption(state.answers, x.of, x.index);
      return true;
    }
    case "send": {
      // built now, from the answers as they are: an answer staged since the last frame is in the message, a cleared one is not
      const x = t.bar;
      if (!x || state.sent || !progress(x.choices, state.answers).answered) return false;
      env.send(buildMessage(x.choices, state.answers, x.title), { confirm: true });
      state.sent = true;
      return true;
    }
    case "clear":
      if (state.sent) return false;
      state.answers = {};
      return true;
    case "button":
      if (state.delivery[t.id] === "sending" || state.delivery[t.id] === "queued") return false;
      if (t.send) env.send(t.send, { confirm: true, targetId: t.id });
      if (t.openUrl) {
        env.copy(t.openUrl, t.id);
      }
      return false;
    case "copy":
      if (t.copy !== undefined) env.copy(t.copy, t.id);
      return false;
    case "node":
    case "image":
    case "tip": {
      const tip = t.tip;
      if (!tip || !(tip.detail || tip.code || tip.link)) return false;
      state.tip = state.tip === t.id ? null : t.id;
      return true;
    }
  }
  return false;
}

/** Scroll a wide diagram so the target is visible. Returns true when the pan changed. */
export function reveal(t: Target, state: WState): boolean {
  const p = t.pan;
  if (!p) return false;
  const cur = state.pan[p.key] ?? 0;
  let next = cur;
  if (p.x < cur) next = Math.max(0, p.x - 2);
  else if (p.x + p.w > cur + p.view) next = Math.min(p.total - p.view, p.x + p.w - p.view + 2);
  if (next === cur) return false;
  state.pan[p.key] = next;
  return true;
}

/** The one-line explanation shown for a hovered or focused target. */
export function describe(t: Target): string {
  const bits: string[] = [];
  switch (t.kind) {
    case "sort":
      bits.push(`${t.label}: click to sort`);
      break;
    case "cell":
      bits.push(`${t.label}: view and copy the full value`);
      break;
    case "fold":
    case "source":
      bits.push(t.label);
      break;
    case "button":
      bits.push(
        `[${t.label}]`,
        t.send
          ? `sends "${t.send.length > 70 ? t.send.slice(0, 69) + "…" : t.send}"`
          : t.openUrl
            ? `link ${t.openUrl}`
            : "",
      );
      break;
    case "choice":
      bits.push(
        t.label,
        t.choice?.index === "other"
          ? "Enter to type your own answer"
          : t.choice?.of.multiple
            ? "Enter to toggle"
            : "Enter to select",
      );
      break;
    case "send":
      bits.push("send all the answers to the agent as one message");
      break;
    case "clear":
      bits.push("clear every answer");
      break;
    case "tab":
      bits.push(`tab ${t.label}`);
      break;
    case "faq":
      bits.push(t.label, "Enter to open or close");
      break;
    case "copy":
      bits.push(t.label);
      break;
    case "image":
      bits.push(t.label);
      break;
    case "item":
      bits.push(t.label, "Enter to write a note");
      break;
    case "note":
      bits.push(`${t.label}`, "Enter to edit, Delete to remove");
      break;
    case "notes":
      bits.push("notes: click an element, or Tab to it and press Enter, to write a note about it");
      break;
    case "noteadd":
      bits.push("put the notes in your message box (nothing is sent)");
      break;
    case "noteclear":
      bits.push("discard all notes on this widget");
      break;
    default:
      bits.push(t.label);
  }
  if (t.tip?.hint) bits.push(t.tip.hint);
  if (t.kind !== "sort" && t.tip && (t.tip.detail || t.tip.code || t.tip.link)) bits.push("Enter for details");
  return bits.filter(Boolean).join("  ·  ");
}

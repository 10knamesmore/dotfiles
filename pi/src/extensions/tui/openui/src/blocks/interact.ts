import { buildMessage, progress, toggleOption } from "./choice.ts";
import type { Target, WState } from "./types.ts";

export interface Env {
  /** Returns whether the destination accepted or queued the text. `confirm` requests delivery feedback for the target. */
  send(text: string, opts?: { confirm?: boolean; targetId?: string }): boolean;
  /** Write to the clipboard; the host updates feedback after the write completes. */
  copy(text: string, targetId: string): void;
  /** show a short notice under the widget */
  note(text: string): void;
}

/**
 * In notes mode, only annotatable items, the notes tray, tabs and panning respond to the mouse.
 */
export function isLive(t: Target, notes: boolean): boolean {
  if (!notes) return t.kind !== "item";
  return (
    t.kind === "item" ||
    t.kind === "tab" ||
    t.kind === "pan" ||
    t.kind === "note" ||
    t.kind === "noteadd" ||
    t.kind === "noteclear" ||
    t.kind === "notes"
  );
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
    case "pan": {
      const { key, offset } = t.panTo!;
      state.pan[key] = offset;
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
      if (!env.send(buildMessage(x.choices, state.answers, x.title), { confirm: true })) return false;
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

/** Pan a wide component to reveal the clicked target. */
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

export function describe(t: Target, sendNotes: boolean, noteMode: boolean): string {
  const bits: string[] = [];
  switch (t.kind) {
    case "sort":
      bits.push(`${t.label}: click to sort`);
      break;
    case "cell":
      bits.push(`${t.label}: view and copy the full value`);
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
          ? "click to type your own answer"
          : t.choice?.of.multiple
            ? "click to toggle"
            : "click to select",
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
      bits.push(t.label, "click to open or close");
      break;
    case "copy":
      bits.push(t.label);
      break;
    case "image":
      bits.push(t.label);
      break;
    case "item":
      bits.push(t.label, "click to write a note");
      break;
    case "note":
      bits.push(t.label, "click to edit; save empty to remove");
      break;
    case "notes":
      bits.push(noteMode ? "leave notes mode" : "enter notes mode");
      break;
    case "noteadd":
      bits.push(
        sendNotes ? "send the notes as a side follow-up" : "put the notes in your message box (nothing is sent)",
      );
      break;
    case "noteclear":
      bits.push("discard all notes on this widget");
      break;
    default:
      bits.push(t.label);
  }
  if (t.tip?.hint) bits.push(t.tip.hint);
  if (t.kind !== "sort" && t.tip && (t.tip.detail || t.tip.code || t.tip.link)) bits.push("click to toggle details");
  return bits.filter(Boolean).join("  ·  ");
}

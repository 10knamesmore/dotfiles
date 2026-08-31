/**
 * Pure interaction logic for the navigator: status filter cycling, child
 * ordering, key-to-action mapping, and footer-hint composition. Kept free of any
 * pi-tui or filesystem dependency so it unit tests directly.
 *
 * Pause is deliberately absent from the verb set: the phase-3 runner exposes no
 * pause/resume seam (a child Handle offers only abort/steer, the
 * workflow VM runs to completion, and the semaphore gates concurrency, not
 * suspension). Faking it is disallowed, so `p` maps to no action.
 */

import type { SubagentStatus } from "../../types.js";
import type { ThemeLike } from "../format.js";
import type { ChildRow, RunDetail } from "./store-read.js";

/** The three navigator levels. */
export type Level = "runs" | "run" | "agent";

/** Status filter applied at the run-detail level. */
export type FilterMode = "all" | "running" | "completed" | "failed";

export const FILTERS: readonly FilterMode[] = ["all", "running", "completed", "failed"];

/** Advance to the next filter in the cycle, wrapping at the end. */
export function cycleFilter(current: FilterMode): FilterMode {
  const index = FILTERS.indexOf(current);
  return FILTERS[(index + 1) % FILTERS.length]!;
}

/** Whether a child status passes the active filter. */
export function passesFilter(status: SubagentStatus, filter: FilterMode): boolean {
  switch (filter) {
    case "running":
      return status === "running" || status === "pending";
    case "completed":
      return status === "completed";
    case "failed":
      return status === "failed" || status === "aborted";
    default:
      return true;
  }
}

/**
 * Children in navigation order: workflow runs group by phase order, then
 * alphabetically. Filtered by status.
 */
export function orderedChildren(detail: RunDetail, filter: FilterMode): ChildRow[] {
  const filtered = detail.children.filter((child) => passesFilter(child.status, filter));
  // Alphabetical by label (id tiebreak) so the list is stable while a run
  // executes: start-time ordering floated queued children (no startedAt)
  // above running ones and reshuffled rows as workers launched.
  const alphabetical = (a: ChildRow, b: ChildRow): number =>
    a.label.localeCompare(b.label) || a.id.localeCompare(b.id);
  if (detail.kind !== "workflow") return [...filtered].sort(alphabetical);
  const order = detail.phases.map((phase) => phase.title);
  const rank = (child: ChildRow): number => {
    const index = child.phase ? order.indexOf(child.phase) : -1;
    return index < 0 ? order.length : index;
  };
  return [...filtered].sort((a, b) => rank(a) - rank(b) || alphabetical(a, b));
}

export interface RunActionAvailability {
  canStop: boolean;
  canSave: boolean;
}

/** The shared eligibility rules for run-detail hints and their key handlers. */
export function runActionAvailability(
  detail: RunDetail,
  runIsLive: boolean,
  handleStatuses: readonly SubagentStatus[],
  saveAvailable: boolean,
): RunActionAvailability {
  return {
    canStop: runIsLive || handleStatuses.some((status) => status === "running" || status === "pending"),
    canSave: saveAvailable && !detail.corrupt && detail.kind === "workflow" && detail.status === "completed" && detail.hasScript,
  };
}

type NavAction =
  | { type: "move"; delta: number }
  | { type: "pageMove"; delta: number }
  | { type: "cycleLive"; delta: 1 | -1 }
  | { type: "drill" }
  | { type: "back" }
  | { type: "close" }
  | { type: "stop" }
  | { type: "filter" }
  | { type: "save" }
  | { type: "steer" }
  | { type: "none" };

/** Map a parsed key id to an action for the given level. */
export function keyToAction(keyId: string | undefined, level: Level): NavAction {
  switch (keyId) {
    case "up":
    case "k":
      return { type: "move", delta: -1 };
    case "down":
    case "j":
      return { type: "move", delta: 1 };
    case "pageup":
      return { type: "pageMove", delta: -1 };
    case "pagedown":
      return { type: "pageMove", delta: 1 };
    case "tab":
      return { type: "cycleLive", delta: 1 };
    case "shift+tab":
      return { type: "cycleLive", delta: -1 };
    case "enter":
    case "return":
      if (level === "agent") return { type: "steer" };
      return { type: "drill" };
    case "right":
      return level === "agent" ? { type: "none" } : { type: "drill" };
    case "escape":
    case "esc":
    case "left":
      return { type: "back" };
    case "x":
      return { type: "stop" };
    case "f":
      return level === "run" ? { type: "filter" } : { type: "none" };
    case "s":
      return { type: "save" };
    default:
      return { type: "none" };
  }
}

interface FooterState {
  level: Level;
  /** Whether at least two live runs are available to cycle between. */
  canCycle?: boolean;
  /** Run-detail only: the active status filter, shown in the hint. */
  filter?: FilterMode;
  /** Whether the selected run or agent can currently be stopped. */
  canStop?: boolean;
  /** Whether the selected workflow can be saved. */
  canSave?: boolean;
  /** Agent-detail only: whether the steering composer is available. */
  canSteer?: boolean;
  /** Agent-detail only: whether a persisted child can start a follow-up thread. */
  canMessage?: boolean;
  /** Whether a same-view, second-press stop confirmation is armed. */
  stopArmed?: boolean;
}

/** Compose the footer key-hint line for a level. Matches pi's dim selector footers. */
export function footerHint(state: FooterState, theme: ThemeLike): string {
  const parts: string[] = ["↑↓ select"];
  if (state.level === "runs") {
    parts.push("enter open", "esc close");
    if (state.canCycle) parts.push("tab next live");
    if (state.canStop) parts.push(state.stopArmed ? "x again to STOP" : "x stop");
    if (state.canSave) parts.push("s save");
  } else if (state.level === "run") {
    parts.push("enter open", "esc back", `f filter: ${state.filter ?? "all"}`);
    if (state.canCycle) parts.push("tab next live");
    if (state.canStop) parts.push(state.stopArmed ? "x again to STOP" : "x stop");
    if (state.canSave) parts.push("s save");
  } else {
    parts.length = 0;
    parts.push("↑↓ scroll", "shift+↑↓ page");
    if (state.canSteer) parts.push("enter steer");
    else if (state.canMessage) parts.push("enter message");
    if (state.canCycle) parts.push("tab next live");
    if (state.canStop) parts.push(state.stopArmed ? "x again to STOP" : "x stop");
    parts.push("esc back");
  }
  return theme.fg("dim", parts.join(" · "));
}

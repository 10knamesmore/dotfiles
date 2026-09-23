import type {
  ExtensionAPI,
  ExtensionContext,
} from "@earendil-works/pi-coding-agent";
import { cloneTodoPhases, parseTodoPhases, type TodoPhase } from "./model.js";

/** Custom entry type used to persist parent-session todo snapshots. */
export const TODO_STATE_ENTRY_TYPE = "dotfiles.pi.todo-state";

/** Current serialized snapshot format. */
export const TODO_STATE_VERSION = 1;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function phasesFromContainer(value: unknown): TodoPhase[] | undefined {
  if (!isRecord(value) || value.version !== TODO_STATE_VERSION)
    return undefined;
  return parseTodoPhases(value.phases);
}

/**
 * Owns canonical in-memory todo state and branch-local reconstruction.
 *
 * Tool mutations persist in their successful result details. Custom entries were
 * written by the removed manual `/todo edit` command and are still read back for
 * compatibility with sessions created before that command was dropped.
 */
export class TodoSessionStore {
  private phases: TodoPhase[] = [];

  public constructor(private readonly pi: ExtensionAPI) {}

  /** Return a detached snapshot that callers may safely mutate. */
  public snapshot(): TodoPhase[] {
    return cloneTodoPhases(this.phases);
  }

  /** Install a validated state that the caller will return in a successful tool result. */
  public install(phases: readonly TodoPhase[]): void {
    const next = parseTodoPhases(phases);
    if (next === undefined)
      throw new Error(
        "Refusing to install invalid or non-normalized todo state.",
      );
    this.phases = next;
  }

  /**
   * Restore the newest valid snapshot on the active branch.
   *
   * Custom entries cover the removed manual `/todo edit` command. Tool-result
   * details provide the same branch-local state and remain a recovery source
   * for ordinary calls.
   */
  public restore(ctx: ExtensionContext): string | undefined {
    const branch = ctx.sessionManager.getBranch();
    let invalidSnapshots = 0;
    for (let index = branch.length - 1; index >= 0; index -= 1) {
      const entry = branch[index];
      if (entry === undefined) continue;
      let restored: TodoPhase[] | undefined;
      if (
        entry.type === "custom" &&
        entry.customType === TODO_STATE_ENTRY_TYPE
      ) {
        restored = phasesFromContainer(entry.data);
        if (restored === undefined) invalidSnapshots += 1;
      } else if (
        entry.type === "message" &&
        entry.message.role === "toolResult" &&
        entry.message.toolName === "todo" &&
        !entry.message.isError
      ) {
        restored = phasesFromContainer(entry.message.details);
        if (restored === undefined) invalidSnapshots += 1;
      }
      if (restored !== undefined) {
        this.phases = restored;
        return invalidSnapshots > 0
          ? `Ignored ${invalidSnapshots} invalid newer todo snapshot${invalidSnapshots === 1 ? "" : "s"}.`
          : undefined;
      }
    }
    this.phases = [];
    return invalidSnapshots > 0
      ? `No valid todo snapshot remained after ignoring ${invalidSnapshots} invalid entr${invalidSnapshots === 1 ? "y" : "ies"}.`
      : undefined;
  }
}

import type { AssistantMessage, AssistantMessageEvent, Message } from "@earendil-works/pi-ai";
import type { UsageSummary } from "../types.js";

/** Events consumed at the subprocess child-session seam. Pi coding-agent custom,
 * bash-execution, branch-summary, and compaction-summary messages are intentionally
 * outside this runner contract and are ignored by the RPC adapter. */
export interface OpaqueAssistantMessageUpdate {
  readonly opaqueType: string;
}

export type ChildSessionEvent =
  | { type: "tool_execution_start"; toolCallId: string; toolName: string; args: unknown }
  | { type: "tool_execution_end"; toolCallId: string; toolName: string; result: unknown; isError: boolean }
  | { type: "turn_end"; message: Message }
  | { type: "message_start"; message: Message }
  | { type: "message_update"; message: AssistantMessage; assistantMessageEvent: AssistantMessageEvent }
  | { type: "message_update"; opaqueAssistantMessageUpdate: OpaqueAssistantMessageUpdate }
  | { type: "message_end"; message: Message }
  | { type: "agent_start" }
  | { type: "agent_settled" };

/**
 * The exact subprocess-child session surface the runner consumes. The RPC
 * adapter keeps process transport and containment outside the orchestration
 * layer while exposing only the child state the runner needs.
 */
export interface ChildSession {
  subscribe(listener: (event: ChildSessionEvent) => void): () => void;
  prompt(text: string): Promise<void>;
  steer(text: string): Promise<void>;
  abort(): Promise<void>;
  /** Assistant message currently streaming, retained through message_end until
   * agent_settled hands rendering back to the persisted session file. */
  readonly currentAssistant: AssistantMessage | undefined;
  /** Newest finalized assistant message, used for terminal result extraction. */
  readonly latestAssistant: AssistantMessage | undefined;
  /** Drop the retained message after the runner copies it into a terminal result. */
  clearLatestAssistant?(): void;
  /** Cumulative assistant usage folded at message arrival so totals remain
   * monotonic across pi-side transcript compaction. */
  readonly usage: UsageSummary;
  /** Child-written session file reported by the subprocess at startup. */
  readonly sessionFile: string | undefined;
  /** Asynchronous subprocess disposal: SIGTERM the group, escalate to a group
   * SIGKILL only while the leader is still alive at the grace deadline, then
   * await real exit. Group signals are never sent after the leader is reaped
   * (the numeric group id can be recycled), so a SIGTERM-ignoring descendant
   * that outlives a fast-exiting leader is an accepted bounded orphan. */
  dispose(): Promise<void>;
}

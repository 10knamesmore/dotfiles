/**
 * ChildSession adapter for one `pi --mode rpc` subprocess. It translates the
 * runner lifecycle into RPC commands, folds bounded result and usage state,
 * and delegates process-group termination to the transport.
 */

import type { AssistantMessage, AssistantMessageEvent, Message } from "@earendil-works/pi-ai";
import type { UsageSummary } from "../../types.js";
import { isRecord } from "../../util.js";
import type { ChildSession, ChildSessionEvent } from "../child-session.js";
import type { ChildRpc } from "./rpc-transport.js";

/** SIGTERM-to-SIGKILL escalation window on dispose. */
const DISPOSE_KILL_GRACE_MS = 2_000;
/** Deadline on control-plane requests (get_state, steer); the turn itself is unbounded. */
const CONTROL_REQUEST_TIMEOUT_MS = 15_000;
/** Deadline on the prompt preflight ack (not the turn): bounds a child wedged
 * before it even accepts the prompt, so it cannot hold the semaphore forever. */
const PROMPT_ACK_TIMEOUT_MS = 60_000;
/** How long a prompt with no agent run may stay unconfirmed before failing. */
const IMMEDIATE_COMPLETION_POLL_MS = 60;
const IMMEDIATE_COMPLETION_MAX_POLLS = 40;

interface RpcState {
  isStreaming: boolean;
  isCompacting: boolean;
  pendingMessageCount: number;
  sessionFile?: string;
}

function isRpcState(value: unknown): value is RpcState {
  return isRecord(value)
    && typeof value.isStreaming === "boolean"
    && typeof value.isCompacting === "boolean"
    && typeof value.pendingMessageCount === "number"
    && Number.isSafeInteger(value.pendingMessageCount)
    && value.pendingMessageCount >= 0
    && (value.sessionFile === undefined || typeof value.sessionFile === "string");
}

const KNOWN_ASSISTANT_UPDATE_TYPES = new Set([
  "start", "text_start", "text_delta", "text_end", "thinking_start", "thinking_delta", "thinking_end",
  "toolcall_start", "toolcall_delta", "toolcall_end", "done", "error",
]);

function isRunActivity(event: ChildSessionEvent): boolean {
  if (event.type === "message_start" || event.type === "message_end") {
    return event.message.role === "assistant" || event.message.role === "user" || event.message.role === "toolResult";
  }
  return event.type === "message_update"
    || event.type === "turn_end"
    || event.type === "tool_execution_start"
    || event.type === "tool_execution_end";
}

/** Narrow the untyped JSONL transport to the events this repository consumes. */
function childSessionEvent(event: Record<string, unknown>): ChildSessionEvent | undefined {
  if (typeof event.type !== "string") return undefined;
  switch (event.type) {
    case "agent_start": return { type: "agent_start" };
    case "agent_settled": return { type: "agent_settled" };
    case "tool_execution_start":
      return typeof event.toolCallId === "string"
        && typeof event.toolName === "string"
        && Object.hasOwn(event, "args")
        ? { type: "tool_execution_start", toolCallId: event.toolCallId, toolName: event.toolName, args: event.args }
        : undefined;
    case "tool_execution_end":
      return typeof event.toolCallId === "string"
        && typeof event.toolName === "string"
        && Object.hasOwn(event, "result")
        && typeof event.isError === "boolean"
        ? { type: "tool_execution_end", toolCallId: event.toolCallId, toolName: event.toolName, result: event.result, isError: event.isError }
        : undefined;
    case "turn_end":
    case "message_start":
    case "message_end":
      return isRecord(event.message) && typeof event.message.role === "string"
        ? { type: event.type, message: event.message as unknown as Message }
        : undefined;
    case "message_update": {
      const update = event.assistantMessageEvent;
      if (!isRecord(update) || typeof update.type !== "string") return undefined;
      if (!KNOWN_ASSISTANT_UPDATE_TYPES.has(update.type)) {
        return isRecord(event.message) && event.message.role === "assistant"
          ? { type: "message_update", opaqueAssistantMessageUpdate: { opaqueType: update.type } }
          : undefined;
      }
      return isRecord(event.message) && event.message.role === "assistant"
        ? {
          type: "message_update",
          message: event.message as unknown as AssistantMessage,
          assistantMessageEvent: update as unknown as AssistantMessageEvent,
        }
        : undefined;
    }
    default:
      return undefined;
  }
}

type KnownAssistantMessageUpdate = Extract<ChildSessionEvent, { type: "message_update"; message: AssistantMessage }>;

function knownAssistantMessageUpdate(event: ChildSessionEvent): KnownAssistantMessageUpdate | undefined {
  return event.type === "message_update" && Object.hasOwn(event, "message")
    ? event as KnownAssistantMessageUpdate
    : undefined;
}

interface FoldedAssistantUsage {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  cost: number;
}

function foldedAssistantUsage(message: Message): FoldedAssistantUsage | undefined {
  if (message.role !== "assistant" || !isRecord(message.usage) || !isRecord(message.usage.cost)) return undefined;
  const { input, output, cacheRead, cacheWrite } = message.usage;
  const cost = message.usage.cost.total;
  return typeof input === "number"
    && typeof output === "number"
    && typeof cacheRead === "number"
    && typeof cacheWrite === "number"
    && typeof cost === "number"
    ? { input, output, cacheRead, cacheWrite, cost }
    : undefined;
}

export type PromptAttemptEvent =
  | { type: "started"; attempt: number }
  | { type: "event"; attempt: number; event: ChildSessionEvent }
  | { type: "settled"; attempt: number }
  | { type: "discarded"; attempt: number };

/** One explicit acknowledgement/settlement state machine per prompt attempt. */
class PromptTurn {
  runStarted = false;
  done = false;
  readonly completion: Promise<void>;
  private acknowledgement: "pending" | "accepted" = "pending";
  private settlementObserved = false;
  private resolveCompletion!: () => void;
  private rejectCompletion!: (error: Error) => void;

  constructor(
    readonly attempt: number,
    private readonly finalizeAttempt: (outcome: "settled" | "discarded") => void,
  ) {
    this.completion = new Promise((resolve, reject) => {
      this.resolveCompletion = resolve;
      this.rejectCompletion = reject;
    });
    // prompt() can throw on acknowledgement before it awaits completion.
    void this.completion.catch(() => undefined);
  }

  acknowledge(): void {
    if (this.done) return;
    this.acknowledgement = "accepted";
    this.finalizeIfReady();
  }

  observeAcceptance(): void {
    if (this.done) return;
    this.runStarted = true;
  }

  /** Event-based settlement requires this turn's own run evidence, so a stale
   * agent_settled from a previous turn can never finish a queued next prompt. */
  observeSettlement(): void {
    if (this.done || !this.runStarted) return;
    this.settlementObserved = true;
    this.finalizeIfReady();
  }

  /** Unlike observeSettlement, no runStarted check: the poller's own repeated
   * fully-idle observations are the evidence that no run will ever start. */
  observeImmediateCompletion(): void {
    if (this.done) return;
    this.settlementObserved = true;
    this.finalizeIfReady();
  }

  fail(error: Error): void {
    if (this.done) return;
    this.done = true;
    this.finalizeAttempt("discarded");
    this.rejectCompletion(error);
  }

  private finalizeIfReady(): void {
    if (this.done || !this.settlementObserved || this.acknowledgement !== "accepted") return;
    this.done = true;
    this.finalizeAttempt("settled");
    this.resolveCompletion();
  }
}

interface RpcChildSessionInit {
  /** The child-written session file, from get_state at startup. */
  sessionFile: string | undefined;
  lifecycleEventsEnabled?: boolean;
}

export class RpcChildSession implements ChildSession {
  private assistantInProgress: AssistantMessage | undefined;
  /** Folding at message_end - not turn_end - means a process that dies
   * mid-turn still exposes its newest result and counts its billed usage. */
  latestAssistant: AssistantMessage | undefined;
  readonly usage: UsageSummary = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0, turns: 0 };
  private readonly listeners = new Set<(event: ChildSessionEvent) => void>();
  private readonly promptAttemptListeners = new Set<(event: PromptAttemptEvent) => void>();
  private startupSessionFile: string | undefined;
  private turn: PromptTurn | undefined;
  private promptAttempt = 0;
  private lifecycleEventsEnabled: boolean;
  private exitError: Error | undefined;
  private disposal?: Promise<void>;

  constructor(private readonly rpc: ChildRpc, init: RpcChildSessionInit) {
    this.startupSessionFile = init.sessionFile;
    this.lifecycleEventsEnabled = init.lifecycleEventsEnabled ?? true;
    rpc.onEvent((event) => {
      // Frames before the validated idle snapshot belong to startup and must not
      // contaminate the first prompt's assistant, usage, attempts, or subscribers.
      if (!this.lifecycleEventsEnabled) return;
      const classified = childSessionEvent(event);
      if (classified) this.handleEvent(classified);
    });
    rpc.onExit((exit) => {
      this.assistantInProgress = undefined;
      this.exitError = new Error(`Child pi process exited before settling (code ${exit.code ?? "null"}, signal ${exit.signal ?? "null"}). Stderr: ${rpc.stderrTail() || "(empty)"}`);
      // A prompt awaiting its turn must fail, never hang, on child death.
      this.turn?.fail(this.exitError);
    });
  }

  /** Attach lifecycle observation before inspecting startup state. */
  static async start(rpc: ChildRpc): Promise<RpcChildSession> {
    const session = new RpcChildSession(rpc, {
      sessionFile: undefined,
      lifecycleEventsEnabled: false,
    });
    const state = await rpc.request({ type: "get_state" }, { timeoutMs: CONTROL_REQUEST_TIMEOUT_MS });
    if (!isRpcState(state)) throw new Error("Child RPC startup get_state response was malformed");
    if (state.isStreaming || state.isCompacting || state.pendingMessageCount > 0) {
      throw new Error("Child RPC startup state was not idle");
    }
    if (session.exitError) throw session.exitError;
    session.startupSessionFile = state.sessionFile;
    session.lifecycleEventsEnabled = true;
    return session;
  }

  get sessionFile(): string | undefined {
    return this.startupSessionFile;
  }

  get currentAssistant(): AssistantMessage | undefined {
    return this.assistantInProgress;
  }

  subscribe(listener: (event: ChildSessionEvent) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  onPromptAttempt(listener: (event: PromptAttemptEvent) => void): () => void {
    this.promptAttemptListeners.add(listener);
    return () => this.promptAttemptListeners.delete(listener);
  }

  async prompt(text: string): Promise<void> {
    if (this.exitError) throw this.exitError;
    if (this.turn) throw new Error("Child already has a prompt running");
    const attempt = ++this.promptAttempt;
    const turn = new PromptTurn(attempt, (outcome) => this.emitPromptAttempt({ type: outcome, attempt }));
    this.turn = turn;
    this.emitPromptAttempt({ type: "started", attempt });
    try {
      // The prompt response is the preflight ack, not turn completion; the turn
      // ends at agent_settled (unbounded - the runner's timeout/dispose bound it).
      await this.rpc.request({ type: "prompt", message: text }, { timeoutMs: PROMPT_ACK_TIMEOUT_MS });
      turn.acknowledge();
      // A prompt handled immediately (e.g. a slash command) never runs the
      // agent and never emits agent_settled; the poller settles that case.
      if (!turn.done) void this.confirmImmediateCompletion(turn);
      await turn.completion;
    } catch (error) {
      turn.fail(error instanceof Error ? error : new Error(String(error)));
      throw error;
    } finally {
      if (this.turn === turn) this.turn = undefined;
    }
  }

  async steer(text: string): Promise<void> {
    await this.rpc.request({ type: "steer", message: text }, { timeoutMs: CONTROL_REQUEST_TIMEOUT_MS });
  }

  async abort(): Promise<void> {
    if (this.exitError) return; // Aborting a dead child is complete by definition.
    await this.rpc.request({ type: "abort" }, { timeoutMs: CONTROL_REQUEST_TIMEOUT_MS });
  }

  clearLatestAssistant(): void {
    this.latestAssistant = undefined;
  }

  /** SIGTERM the group, escalate to a group SIGKILL only while the leader is
   * still alive at the grace deadline, and await real exit. A descendant that
   * outlives a fast-exiting leader is a bounded orphan. */
  async dispose(): Promise<void> {
    if (this.disposal) return this.disposal;
    const disposal = Promise.resolve().then(async () => {
      this.rpc.kill("SIGTERM");
      const escalate = setTimeout(() => this.rpc.kill("SIGKILL"), DISPOSE_KILL_GRACE_MS);
      escalate.unref?.();
      try {
        await this.rpc.exited;
      } finally {
        clearTimeout(escalate);
      }
    });
    this.disposal = disposal;
    return disposal;
  }

  /**
   * Settle a turn whose prompt was handled WITHOUT an agent run (a slash
   * command or input handler): those never emit agent_settled, so waiting for
   * it would hold the semaphore slot forever. Run activity on the ordered
   * event stream stands this poller down; only agent_settled or child death
   * may then end the turn.
   */
  private async confirmImmediateCompletion(turn: PromptTurn): Promise<void> {
    let idleObservations = 0;
    for (let poll = 0; poll < IMMEDIATE_COMPLETION_MAX_POLLS; poll++) {
      await new Promise((resolve) => { const timer = setTimeout(resolve, IMMEDIATE_COMPLETION_POLL_MS); timer.unref?.(); });
      if (turn.done || turn.runStarted) return;
      let state: RpcState;
      try {
        const response = await this.rpc.request({ type: "get_state" }, {
          timeoutMs: CONTROL_REQUEST_TIMEOUT_MS,
          onResponse: (candidate) => {
            if (isRpcState(candidate) && candidate.isStreaming) turn.observeAcceptance();
          },
        });
        if (turn.done || turn.runStarted) return;
        if (!isRpcState(response)) throw new Error("response did not contain boolean streaming/compaction state, a nonnegative safe pending count, and an optional string session file");
        state = response;
      } catch (error) {
        if (!turn.runStarted && !turn.done) turn.fail(new Error(`Child state read failed while confirming prompt completion: ${error instanceof Error ? error.message : String(error)}`));
        return;
      }
      if (turn.done || turn.runStarted) return;
      // Streaming proves this prompt started even if the child omitted agent_start.
      if (state.isStreaming) {
        turn.observeAcceptance();
        return;
      }
      if (state.isCompacting) {
        idleObservations = 0;
        poll -= 1;
        continue;
      }
      if (state.pendingMessageCount > 0) {
        idleObservations = 0;
        continue;
      }
      idleObservations += 1;
      if (idleObservations >= 2) { turn.observeImmediateCompletion(); return; }
    }
    turn.fail(new Error(`Child neither started an agent run nor settled within ${IMMEDIATE_COMPLETION_MAX_POLLS} polls of the prompt ack`));
  }

  private emitPromptAttempt(event: PromptAttemptEvent): void {
    for (const listener of this.promptAttemptListeners) listener(event);
  }

  private handleEvent(event: ChildSessionEvent): void {
    const activeTurn = this.turn;
    const assistantUpdate = knownAssistantMessageUpdate(event);
    if (activeTurn) {
      this.emitPromptAttempt({ type: "event", attempt: activeTurn.attempt, event });
      if (event.type === "agent_start" || isRunActivity(event)) activeTurn.observeAcceptance();
    }
    // The retained assistant survives non-assistant lifecycle events (tool
    // results in a multi-step turn); only a newer assistant message replaces
    // it, and only agent_settled or process exit clears it.
    if (assistantUpdate) {
      this.assistantInProgress = assistantUpdate.message;
    } else if (
      (event.type === "message_start" || event.type === "message_end")
      && event.message.role === "assistant"
    ) {
      this.assistantInProgress = event.message;
    }
    if (event.type === "message_end" && event.message.role === "assistant") {
      this.latestAssistant = event.message;
      const folded = foldedAssistantUsage(event.message);
      if (folded) {
        this.usage.input += folded.input;
        this.usage.output += folded.output;
        this.usage.cacheRead += folded.cacheRead;
        this.usage.cacheWrite += folded.cacheWrite;
        this.usage.cost += folded.cost;
        this.usage.turns += 1;
      }
    }
    if (event.type === "agent_settled") {
      this.assistantInProgress = undefined;
      this.turn?.observeSettlement();
    }
    // Promise continuations run in a later microtask, so subscribers observe
    // agent_settled before prompt() resumes.
    for (const listener of this.listeners) listener(event);
  }
}

import { randomUUID } from "node:crypto";
import { readFileSync } from "node:fs";
import { isAbsolute, join } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { loadChildExtensionEnvironment, resolveModel, type ParentContext } from "../runner/child.js";
import { assertInlineWorktreePatch, isInlineWorktreePatch } from "../runner/inline-patch.js";
import { subagentRunner, type SubagentRunner } from "../runner/runner.js";
import { readRunSnapshot } from "../store/run-snapshot.js";
import { RunStore } from "../store/run-store.js";
import { sanitizeTerminalText } from "../ui/sanitize.js";
import { validateWorkflowAgentOptions } from "../subagent-spec.js";
import type { SubagentHandle, SubagentResult, SubagentSpec } from "../types.js";
import { reportDiagnostic } from "../diagnostics.js";
import { bindAbort, errorMessage, isRecord } from "../util.js";
import {
  CALL_FINGERPRINT_VERSION,
  describeFingerprintDrift,
  hashAgentPayload,
  isInCausalTail,
  journalCallKey,
  readJournal,
  type CallFingerprint,
  type JournalEntry,
  type WorkflowJournal,
} from "./journal.js";
import { parseWorkflowScript, type ParsedWorkflow } from "./parser.js";
import { resolveRunDir } from "./run-dir.js";
import { executeWorkflowBody, type WorkflowCallIdentity, type WorkflowVmApi } from "./vm.js";

export const WORKFLOW_AGENT_CAP = 200;
const MAX_WORKFLOW_AGENT_CAP = 1_000;

interface WorkflowRunInput {
  readonly script: string;
  readonly args?: unknown;
  readonly resumeRunId?: string;
  readonly rerunChildIds?: readonly string[];
}

interface ParsedWorkflowRunInput {
  readonly workflow: ParsedWorkflow;
  readonly args?: unknown;
  readonly resumeRunId?: string;
  /** Persisted childIds whose journal entries may re-execute despite environment drift. */
  readonly rerunChildIds?: readonly string[];
}

export interface WorkflowRunResult {
  runId: string;
  runDir: string;
  generation?: number;
  meta: ParsedWorkflow["meta"];
  result: unknown;
  failedChildren: SubagentResult[];
  /** Set when run persistence degraded mid-run; resume may re-execute completed agents. */
  persistenceWarning?: string;
}

export class WorkflowRunError extends Error {
  constructor(message: string, readonly runId: string, readonly runDir: string, readonly persistenceWarning?: string,
    readonly failedChildren: SubagentResult[] = [], readonly generation?: number,
    readonly status: "failed" | "aborted" = "failed") {
    super(message);
    this.name = "WorkflowRunError";
  }
}

interface WorkflowRunnerOptions {
  runner?: SubagentRunner;
  rootDir?: string;
  onLog?: (message: string) => void;
  signal?: AbortSignal;
}

export async function runWorkflow(input: WorkflowRunInput, parent: ParentContext, options: WorkflowRunnerOptions = {}): Promise<WorkflowRunResult> {
  const workflow = parseWorkflowScript(input.script);
  const runner = options.runner ?? subagentRunner;
  const started = startParsedWorkflow(
    { workflow, args: input.args, resumeRunId: input.resumeRunId, rerunChildIds: input.rerunChildIds },
    parent,
    { ...options, runner },
  );
  try {
    return await started.execution;
  } finally {
    runner.releaseRunActivity?.(started.runId);
  }
}

export function startParsedWorkflow(
  input: ParsedWorkflowRunInput,
  parent: ParentContext,
  options: WorkflowRunnerOptions = {},
): { runId: string; runDir: string; execution: Promise<WorkflowRunResult> } {
  const { workflow } = input;
  validateRerunChildIds(input.rerunChildIds, input.resumeRunId);
  const rerunAuthorized = new Set(input.rerunChildIds ?? []);
  const normalizedInputArgs = normalizeArgs(input.args);
  if (!input.resumeRunId || input.args !== undefined) validateWorkflowArgs(normalizedInputArgs);
  const runner = options.runner ?? subagentRunner;
  const root = options.rootDir ?? join(getAgentDir(), "subagent-workflow", "runs");
  const runId = input.resumeRunId ?? `workflow-${Date.now().toString(36)}-${randomUUID().replaceAll("-", "").slice(0, 16)}`;
  // RunStore guards ownership at the OS level. Keep this session check for the
  // more actionable stop-from-/agents error before touching the run directory.
  if (input.resumeRunId && runner.isRunActive(runId)) {
    throw new Error(`Cannot resume workflow: run ${runId} is still active in this session; stop it from /agents first`);
  }
  const existingRunDir = input.resumeRunId ? resolveResumeRunDir(root, parent.ctx.cwd, input.resumeRunId) : undefined;
  const existingSnapshot = existingRunDir ? readRunSnapshot(existingRunDir) : undefined;
  // A run-owned controller lets navigator stop cancel live children and keep
  // the workflow loop from starting more work.
  const runController = new AbortController();
  const store = new RunStore(runId, parent.ctx.cwd, parent.ctx.sessionManager.getSessionId(), parent.ctx.sessionManager.getSessionFile(), {
    rootDir: root,
    kind: "workflow",
    phases: workflow.meta.phases,
    ...(!existingRunDir ? { maxAgentsPerWorkflow: WORKFLOW_AGENT_CAP } : {}),
    existingRunDir,
    existingSnapshot,
  });
  // Deliberately uncached: the comparison contract is "what would this call
  // resolve to NOW", and a per-call disk scan (milliseconds) is noise next to
  // a live model call while never serving a stale environment to a later
  // replay decision in a long generation.
  const resolveCallFingerprint = async (spec: SubagentSpec): Promise<CallFingerprint> => {
    const { model, thinking } = resolveModel(spec, parent.ctx, parent.thinkingLevel);
    const cwd = spec.cwd ?? parent.ctx.cwd;
    const environment = await loadChildExtensionEnvironment(cwd);
    return {
      version: CALL_FINGERPRINT_VERSION,
      provider: model.provider,
      modelId: model.id,
      thinkingLevel: thinking,
      cwd,
      extensionTools: environment.extensionTools,
    };
  };
  let args: unknown;
  let journal: WorkflowJournal;
  /** Prior terminal status to reinstate when a resume is refused before mutating replay state. */
  let priorTerminalStatus: "completed" | "failed" | "aborted" | undefined;
  /**
   * Restoring terminal state after a refusal is only sound when the refused
   * generation changed nothing canonical: with an edited script or explicit
   * args the generation commit has already replaced those files, and
   * reinstating "completed" would pair the OLD result with the NEW inputs.
   */
  let canonicalInputsUnchanged = false;
  /** True once this generation invalidated journal entries or spawned a child. */
  let generationMutated = false;
  // Error identity does not survive the VM worker boundary (the sandbox
  // re-wraps errors as plain Error), so refusals raised host-side in
  // api.agent are tracked here instead of via instanceof at the catch.
  let replayRefused = false;
  let generation = 0;
  let generationInputs: { args?: { value: unknown }; rerunChildIds?: readonly string[] };
  // Every actual spawn becomes a persisted child. Seed from the store so the
  // run's lifetime cap remains a lifetime cap across repeated resumes.
  let agentAttemptCount = 0;
  let agentCap = WORKFLOW_AGENT_CAP;
  try {
    agentAttemptCount = store.childCount;
    if (existingRunDir) {
      // The limit is persisted run policy. Legacy runs use the historical
      // fixed limit when no policy was captured.
      agentCap = store.maxAgentsPerWorkflow ?? WORKFLOW_AGENT_CAP;
      validateAgentCap(agentCap, `Cannot resume workflow ${runId}: persisted maxAgentsPerWorkflow`);
      args = input.args === undefined ? readPersistedArgs(existingRunDir) : normalizedInputArgs;
      generationInputs = {
        ...(input.args !== undefined ? { args: { value: args } } : {}),
        ...(input.rerunChildIds?.length ? { rerunChildIds: input.rerunChildIds } : {}),
      };
    } else {
      args = normalizedInputArgs;
      generationInputs = { args: { value: args } };
    }
    // Every fallible read needed to prepare replay happens before the
    // generation transaction can replace script.js or any other canonical
    // input. A bad journal therefore leaves a completed generation intact.
    journal = readJournal(join(store.runDir, "journal.jsonl"));
    if (rerunAuthorized.size > 0) {
      // An authorization that matches nothing must fail here, before the
      // generation commits, so no phantom grant is ever durably recorded.
      const journaled = new Set([...journal.entries.values()].map((entry) => entry.childId));
      const unknown = [...rerunAuthorized].filter((id) => !journaled.has(id));
      if (unknown.length > 0) {
        throw new Error(`Cannot resume workflow ${runId}: rerunChildIds ${JSON.stringify(unknown)} match no journaled call; journaled childIds: ${[...journaled].sort().join(", ") || "none"}`);
      }
    }
    const persistedStatus = (store.resumeSnapshot?.status as { status?: unknown } | undefined)?.status;
    if (persistedStatus === "completed" || persistedStatus === "failed" || persistedStatus === "aborted") {
      priorTerminalStatus = persistedStatus;
    }
    if (existingRunDir) {
      const persistedScript = readOptionalScript(join(existingRunDir, "script.js"));
      canonicalInputsUnchanged = persistedScript === workflow.script && input.args === undefined;
    }
    if (options.signal?.aborted) throw new WorkflowAbortedError();
    store.startWorkflowGeneration(workflow.script, workflow.meta.phases, generationInputs, {
      requireExistingScript: existingRunDir !== undefined,
    });
    generation = store.deliveryIdentity?.generation ?? 0;
    if (generation < 1) throw new Error(`Workflow run ${runId} has no active delivery generation`);
  } catch (error) {
    // Resume setup happens after atomic ownership acquisition. If validation or
    // input loading fails, release that ownership without changing prior status.
    store.releaseOwnership();
    throw error;
  }
  let currentPhase = workflow.meta.phases?.[0]?.title;
  const failedChildren: Array<{ key: string; result: SubagentResult }> = [];
  const orderedFailedChildren = (): SubagentResult[] => [...failedChildren]
    .sort((left, right) => compareCodeUnits(left.key, right.key))
    .map((entry) => entry.result);
  const liveHandles = new Set<SubagentHandle>();
  const activeAgentActivity = new Map<string, { label?: string; description: string; updatedAt: number }>();
  // A run-owned controller so a navigator "stop run" cancels the whole loop -
  // not just the children currently in flight. An external signal (a wait-mode
  // turn's Esc) is folded in. The runner registers it under the run id.
  const abortSignal = runController.signal;
  let abortingChildren: Promise<void> | undefined;
  const abortLiveChildren = (): Promise<void> => abortingChildren ??= (async () => {
    const handles = [...liveHandles];
    await Promise.allSettled(handles.map(async (handle) => {
      try {
        await handle.abort();
      } finally {
        await handle.result;
      }
    }));
    // Let agent() continuations remove their handles and durably append any
    // result that won the race with cancellation before ownership is released.
    await Promise.resolve();
  })();
  const onAbort = () => {
    void abortLiveChildren();
  };
  const forwardExternalAbort = () => runController.abort();
  let unbindAbort = (): void => {};
  let unbindExternalAbort = (): void => {};
  try {
    unbindAbort = bindAbort(abortSignal, onAbort);
    unbindExternalAbort = bindAbort(options.signal, forwardExternalAbort);
  } catch (error) {
    // Setup failures must not strand any listener or SQLite ownership acquired
    // before the main execution guard begins.
    try { unbindExternalAbort(); } catch {}
    try { unbindAbort(); } catch {}
    store.releaseOwnership();
    throw error;
  }
  const api: WorkflowVmApi = {
    args,
    signal: abortSignal,
    describeActiveAgents: () => [...activeAgentActivity.entries()]
      .sort(([left], [right]) => compareCodeUnits(left, right))
      .map(([id, activity]) => {
        const ageSeconds = Math.max(0, Math.round((Date.now() - activity.updatedAt) / 100) / 10);
        const detail = `last activity ${ageSeconds}s ago: ${sanitizeTerminalText(activity.description)}`;
        return activity.label
          ? `${id} (${sanitizeTerminalText(activity.label)}; ${detail})`
          : `${id} (${detail})`;
      }),
    phase: (title: string) => {
      if (typeof title !== "string") throw new TypeError("phase(title) requires a string");
      currentPhase = title;
      store.recordPhase(title);
      runner.foldRunProjection?.(runId, { type: "phase", title });
    },
    diagnostic: (message: string) => reportDiagnostic(`[subagent-workflow] ${message}`),
    log: (message: string) => {
      // Sanitize workflow-authored log text before it reaches a terminal (the
      // tool row via onLog, and the persisted event stream the navigator reads).
      const text = sanitizeTerminalText(String(message));
      store.recordLog(text);
      runner.foldRunProjection?.(runId, { type: "log", message: text });
      try {
        options.onLog?.(text);
      } catch (error) {
        // Live tool-row narration is observational and may outlive its UI row.
        // A stale callback must not turn a valid durable workflow into failure.
        reportDiagnostic(`[subagent-workflow] workflow log update failed: ${errorMessage(error)}`);
      }
    },
    agent: async (prompt: string, suppliedOptions: unknown, call: WorkflowCallIdentity) => {
      if (abortSignal.aborted) throw new WorkflowAbortedError();
      if (typeof prompt !== "string" || !prompt.trim()) throw new TypeError("agent(prompt, opts?) requires a non-empty prompt string");
      // Validate the VM-origin value before it can affect call identity, replay,
      // or spawning. This is the same runtime contract used by the direct tool.
      const rawOptions = validateWorkflowAgentOptions(suppliedOptions);
      const phase = rawOptions.phase ?? currentPhase;
      const { phase: _phase, ...optionsWithoutPhase } = rawOptions;
      const spec: SubagentSpec = { prompt, ...optionsWithoutPhase, phase };
      const hash = hashAgentPayload({ prompt, opts: optionsWithoutPhase, phase });
      const key = journalCallKey(call);
      const cached = journal.entries.get(key);
      let fingerprint: CallFingerprint;
      try {
        fingerprint = await resolveCallFingerprint(spec);
      } catch (error) {
        // For a cached call this is a replay decision, and an unresolvable
        // current environment (a pinned model no longer registered, a failed
        // extension scan) must refuse recoverably rather than fail the run.
        if (cached?.hash === hash) {
          replayRefused = true;
          throw new WorkflowReplayRefusedError(
            `Cannot replay workflow call ${cached.childId}${callLabel(spec)}: the current execution environment cannot be resolved: ${errorMessage(error)}. ${REPLAY_RECOVERY_OPTIONS(cached.childId)}`,
          );
        }
        throw error;
      }
      // The extension scan suspends; a stop arriving during it must be seen
      // before any replay decision, invalidation, or attempt accounting - an
      // aborted rerun that already invalidated its entry would otherwise turn
      // into an unauthorized cache miss on the next resume.
      if (abortSignal.aborted) throw new WorkflowAbortedError();
      if (cached?.hash === hash) {
        // Drift permission is enforced on every hash match BEFORE the replay
        // safety gate below: a drifted entry with an unreplayable worktree
        // result must still refuse, not silently rerun.
        const drift = describeFingerprintDrift(cached.fingerprint, fingerprint);
        if (drift.length > 0 && !rerunAuthorized.has(cached.childId)) {
          // Re-executing a completed call can repeat side effects, so drift
          // needs a childId-scoped authorization on this resume request. Fail
          // closed; sibling branches that already replayed are unaffected.
          replayRefused = true;
          throw new WorkflowReplayRefusedError(
            `Cannot replay workflow call ${cached.childId}${callLabel(spec)}: its execution environment changed: ${drift.join("; ")}. ${REPLAY_RECOVERY_OPTIONS(cached.childId)}`,
          );
        }
        if (drift.length === 0 && isReplaySafeAgentResult(cached.result, spec)) {
          return cached.result;
        }
      }

      const repairTornTail = journal.tornTail;
      journal.tornTail = false;
      const invalidated = invalidateJournalTail(journal, call);
      if (invalidated.length > 0 || repairTornTail) store.rewriteJournal(sortedJournalEntries(journal));
      generationMutated = true;
      agentAttemptCount += 1;
      if (agentAttemptCount > agentCap) {
        throw new Error(`Workflow agent lifetime cap is ${agentCap}; attempted call count ${agentAttemptCount}`);
      }
      const handle = runner.spawnRun(spec, parent, { runId, store });
      liveHandles.add(handle);
      // A stop can arrive between the pre-spawn abort check and handle
      // registration. Do not leave that child outside the earlier snapshot.
      if (abortSignal.aborted) {
        await handle.abort();
        liveHandles.delete(handle);
        throw new WorkflowAbortedError();
      }
      activeAgentActivity.set(handle.id, {
        label: handle.spec.label,
        description: "child admitted; waiting for model or tool activity",
        updatedAt: Date.now(),
      });
      const unsubscribeActivity = handle.subscribe((event) => {
        if (event.type !== "activity") return;
        const activity = activeAgentActivity.get(handle.id);
        if (!activity) return;
        activity.description = event.description;
        activity.updatedAt = Date.now();
      });
      let child: SubagentResult;
      try {
        child = await handle.result;
      } finally {
        unsubscribeActivity();
        activeAgentActivity.delete(handle.id);
        liveHandles.delete(handle);
      }
      if (child.status === "failed") failedChildren.push({ key, result: child });
      const result = child.status === "completed" ? workflowAgentResult(child, spec) : null;
      // Only journal successful calls. A failed/aborted call left unjournaled
      // becomes a cache miss on resume and re-executes - which is what the
      // advertised "resume to recover" recovery path promises. Journaling its
      // null would replay the failure verbatim, making recovery a no-op.
      if (child.status === "completed") {
        // The journaled fingerprint IS the prospective scan: resume recomputes
        // that same scan, so recording anything else would manufacture drift.
        // Its declared boundary: the scan reads the authored cwd, so a
        // worktree-isolated child of a dirty checkout can differ from it -
        // repository contents are deliberately outside the fingerprint.
        const entry: JournalEntry = { v: 4, call, hash, fingerprint, result, childId: child.id };
        journal.entries.set(key, entry);
        store.appendJournal(entry);
      }
      return result;
    },
  };
  runner.adoptRunProjection?.(runId, store, readRunSnapshot(store.runDir), workflow.meta.name);
  const execution = deferWorkflowExecution(async () => {
    let resultWritten = false;
    try {
      const raw = await executeWorkflowBody(workflow.body, workflow.meta.name, api);
      // The body can return normally even after a stop if the aborted child was
      // its last call; record the run as aborted, not completed, in that case.
      if (abortSignal.aborted) {
        throw new WorkflowRunError("Workflow stopped", runId, store.runDir, persistenceWarningFor(store), [], generation, "aborted");
      }
      // Normalize the VM-realm return value to a plain JSON value BEFORE marking
      // the run completed. A cyclic object, function, Proxy, or throwing toJSON()
      // would otherwise pass here and blow up later in JSON.stringify at delivery,
      // after completion was already committed - stranding the result.
      const result = normalizeVmResult(raw);
      // Persist the result atomically before the completed status commits: once a
      // reader sees the run completed, result.json is guaranteed present, so a
      // truncated or failed background delivery can always recover the full value.
      store.writeWorkflowResult(result);
      resultWritten = true;
      store.workflowFinished("completed");
      runner.foldRunProjection?.(runId, { type: "workflow_status", status: "completed" });
      // A stop can race the final body-level check and completion commit.
      if (abortSignal.aborted) {
        throw new WorkflowRunError("Workflow stopped", runId, store.runDir, persistenceWarningFor(store), [], generation, "aborted");
      }
      return { runId, runDir: store.runDir, generation, meta: workflow.meta, result, failedChildren: orderedFailedChildren(), persistenceWarning: persistenceWarningFor(store) };
    } catch (error) {
      // Worker watchdog failures and host callback errors can terminate execution
      // while an agent request is still outstanding. Keep ownership until those
      // children are stopped and settled.
      await abortLiveChildren();
      // A replay refusal that never mutated replay state must not tear down a
      // terminal run: the refused generation keeps the prior result.json and
      // reinstates the prior status, so declining to authorize a rerun costs
      // nothing. Once anything spawned or invalidated, the failure is real.
      const refusedIntact = replayRefused && !generationMutated && priorTerminalStatus !== undefined && canonicalInputsUnchanged;
      if (!resultWritten && !refusedIntact) {
        try { store.writeWorkflowResult(undefined); } catch {}
      }
      if (error instanceof WorkflowAbortedError || abortSignal.aborted) {
        store.workflowFinished("aborted", "Workflow stopped");
        runner.foldRunProjection?.(runId, { type: "workflow_status", status: "aborted" });
        throw new WorkflowRunError("Workflow stopped", runId, store.runDir, persistenceWarningFor(store), [], generation, "aborted");
      }
      const message = errorMessage(error);
      if (refusedIntact) {
        store.restoreTerminalStatus(priorTerminalStatus!, message);
        runner.foldRunProjection?.(runId, { type: "workflow_resume_refused", error: message });
        runner.foldRunProjection?.(runId, { type: "workflow_status", status: priorTerminalStatus! });
        throw new WorkflowRunError(message, runId, store.runDir, persistenceWarningFor(store), [], generation);
      }
      store.workflowFinished("failed", message);
      runner.foldRunProjection?.(runId, { type: "workflow_status", status: "failed" });
      // Carry the failed children: a script error like "cannot read properties
      // of null" is usually CAUSED by a failed agent() resolving to null, and
      // the child's own error is the actionable part.
      throw new WorkflowRunError(message, runId, store.runDir, persistenceWarningFor(store), orderedFailedChildren(), generation);
    } finally {
      // Execution settlement releases in-memory runner resources. Durable parent
      // delivery is tracked separately through message acknowledgement.
      runner.markDelivered(runId);
      runner.unregisterRunController(runId);
      unbindAbort();
      unbindExternalAbort();
    }
  });
  try {
    runner.registerRunController(
      runId,
      runController,
      parent.ctx.sessionManager.getSessionId(),
      execution,
      (error) => error instanceof WorkflowRunError && error.status === "aborted",
    );
  } catch (error) {
    runController.abort();
    try { unbindExternalAbort(); } catch {}
    try { unbindAbort(); } catch {}
    store.releaseOwnership();
    runner.releaseRunActivity?.(runId);
    void execution.catch(() => undefined);
    throw error;
  }
  return { runId, runDir: store.runDir, execution };
}

function validateRerunChildIds(rerunChildIds: readonly string[] | undefined, resumeRunId: string | undefined): void {
  if (rerunChildIds === undefined) return;
  if (!resumeRunId) throw new TypeError("rerunChildIds authorizes re-execution of persisted journal entries and requires resumeRunId");
  if (!Array.isArray(rerunChildIds) || rerunChildIds.length === 0 || !rerunChildIds.every((id) => typeof id === "string" && id.trim().length > 0)) {
    throw new TypeError("rerunChildIds must be a non-empty array of persisted childId strings");
  }
}

function readOptionalScript(path: string): string | undefined {
  try {
    return readFileSync(path, "utf8");
  } catch {
    return undefined;
  }
}

async function deferWorkflowExecution(execute: () => Promise<WorkflowRunResult>): Promise<WorkflowRunResult> {
  // The first turn gets startup identity back to launchWorkflow. The second
  // lets its awaiting caller observe the run before VM logs or settlement.
  await Promise.resolve();
  await Promise.resolve();
  return execute();
}

function validateAgentCap(value: number, context: string): void {
  if (!Number.isInteger(value) || value < 1 || value > MAX_WORKFLOW_AGENT_CAP) {
    throw new TypeError(`${context} must be an integer from 1 to ${MAX_WORKFLOW_AGENT_CAP}`);
  }
}

function validateWorkflowArgs(args: unknown): void {
  try {
    if (JSON.stringify(args) === undefined) throw new TypeError("value serializes to undefined");
  } catch (error) {
    throw new TypeError(`Workflow args must be JSON-serializable: ${errorMessage(error)}`);
  }
}

function invalidateJournalTail(
  journal: WorkflowJournal,
  miss: WorkflowCallIdentity,
): JournalEntry[] {
  const invalidated: JournalEntry[] = [];
  for (const [key, entry] of journal.entries) {
    if (!isInCausalTail(entry.call, miss)) continue;
    journal.entries.delete(key);
    invalidated.push(entry);
  }
  return invalidated;
}

function sortedJournalEntries(journal: WorkflowJournal): JournalEntry[] {
  return [...journal.entries.entries()].sort(([left], [right]) => compareCodeUnits(left, right)).map(([, entry]) => entry);
}

function compareCodeUnits(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

/** Worktree metadata must survive the workflow bridge for explicit review/apply. */
function workflowAgentResult(child: SubagentResult, spec: SubagentSpec): unknown {
  const value = child.structured ?? child.text;
  if (spec.isolation !== "worktree") return value;
  const patch = child.patch ?? "";
  // Production collection enforces this before cleaning the worktree. Keep the
  // bridge check as defense in depth so no alternate runner can poison the
  // journal or post an oversized value into the bounded workflow worker.
  assertInlineWorktreePatch(patch);
  return { value, patch, changed: child.changed ?? [] };
}

function isReplaySafeAgentResult(result: unknown, spec: SubagentSpec): boolean {
  if (spec.isolation !== "worktree") return true;
  if (!isRecord(result)) return false;
  return isInlineWorktreePatch(result.patch);
}

function readPersistedArgs(runDir: string): unknown {
  const path = join(runDir, "args.json");
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    const reason = errorMessage(error);
    throw new Error(`Cannot resume workflow: invalid args.json in ${runDir}: ${reason}`);
  }
}

/** A one-line warning when any run write failed; the resume contract is then not guaranteed. */
function persistenceWarningFor(store: RunStore): string | undefined {
  const reason = store.persistenceDegraded;
  return reason ? `Run persistence degraded (${reason}); the on-disk journal may be incomplete and resuming this run may re-execute completed agents` : undefined;
}

/** Force the workflow's return value to a plain JSON value; throws if it cannot serialize. */
function normalizeVmResult(value: unknown): unknown {
  if (value === undefined) return undefined;
  try {
    return JSON.parse(JSON.stringify(value));
  } catch (error) {
    throw new Error(`Workflow return value is not JSON-serializable: ${errorMessage(error)}`);
  }
}

/** Thrown by agent() when the run was stopped; distinguishes a stop from a genuine failure. */
class WorkflowAbortedError extends Error {
  constructor() {
    super("Workflow stopped");
    this.name = "WorkflowAbortedError";
  }
}

/**
 * A replay decision that refused to proceed. Distinguished from a genuine
 * failure so a refusal that mutated nothing can leave the run's prior
 * terminal state intact.
 */
class WorkflowReplayRefusedError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "WorkflowReplayRefusedError";
  }
}

function callLabel(spec: SubagentSpec): string {
  return spec.label ? ` (${spec.label})` : "";
}

function REPLAY_RECOVERY_OPTIONS(childId: string): string {
  return `Options: resume with rerunChildIds: [${JSON.stringify(childId)}] to authorize re-running it once, or run the workflow fresh without resumeRunId.`;
}

export function normalizeArgs(args: unknown): unknown {
  if (typeof args !== "string") return args ?? null;
  try {
    return JSON.parse(args);
  } catch {
    return args;
  }
}

export function readAbsoluteScript(path: string): string {
  if (!isAbsolute(path)) throw new Error("workflow scriptPath must be absolute");
  return readFileSync(path, "utf8");
}

function resolveResumeRunDir(root: string, cwd: string, runId: string): string {
  try {
    return resolveRunDir(cwd, runId, root);
  } catch (error) {
    const reason = errorMessage(error);
    throw new Error(`Cannot resume workflow: ${reason}`);
  }
}

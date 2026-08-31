import { randomUUID } from "node:crypto";
import { cpus } from "node:os";
import { performance } from "node:perf_hooks";
import type { AssistantMessage } from "@earendil-works/pi-ai";
import type { FollowUpReference, ResolvedSpec, SubagentEvent, SubagentHandle, SubagentResult, SubagentSpec, SubagentStatus, UsageSummary } from "../types.js";
import { cloneActivityFold, type RunActivityFold } from "../store/activity-fold.js";
import { cloneRunProjection, foldRunProjection as foldProjection, projectRunSnapshot, type RunProjection, type RunProjectionEvent } from "../store/run-projection.js";
import { readRunSnapshot, type RunSnapshot } from "../store/run-snapshot.js";
import { EMPTY_USAGE, RunStore } from "../store/run-store.js";
import { writeSessionClosedMarker } from "../store/session-closed-marker.js";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage } from "../util.js";
import { followUpSpawn, submittedSpec, type ChildSpawnSpec, type ParentContext } from "./child.js";
import { spawnSubprocessChild } from "./subprocess/spawn-child.js";
import type { ChildSession } from "./child-session.js";
import { STRUCTURED_REPAIR_PROMPT } from "./schema-tool.js";
import type { SchemaCapture } from "./schema-tool.js";
import { Semaphore } from "./semaphore.js";
import { cleanupWorktree, collectWorktree, createWorktree, WorktreeCollectionError, type Worktree } from "./worktree.js";

// Anchored on globalThis: pi re-evaluates this module per cwd generation
// (moduleCache: false), so a plain module-level semaphore would let each
// cwd's children run under their own cap. One process-wide gate keeps the
// concurrency ceiling honest across every module instance. Its version suffix
// changes only when the semaphore's shared-state shape changes.
// Resizing adds mutable capacity state, so this generation cannot safely adopt
// the fixed-capacity instance left by an older hot-loaded extension.
const SEMAPHORE_STATE_VERSION = "v2";
const SEMAPHORE_KEY = `__piSubagentWorkflowSemaphore_${SEMAPHORE_STATE_VERSION}__`;
const globalScope = globalThis as unknown as Record<string, Semaphore | undefined>;
const globalSemaphore: Semaphore = globalScope[SEMAPHORE_KEY] ??= new Semaphore(Math.max(1, Math.min(16, cpus().length - 2)));

type ChildBuilder = typeof spawnSubprocessChild;
type StoreBuilder = (runId: string, parent: ParentContext, storeOptions?: { directDelivery?: boolean }) => RunStore;
type Delay = (ms: number) => Promise<void>;
interface SpawnRunOptions {
  runId?: string;
  store?: RunStore;
  /** Results go directly to a human; persisted in run.json before any child starts. */
  directDelivery?: boolean;
}

const SETTLE_GRACE_MS = 15_000;

function defaultDelay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, ms);
    timer.unref?.();
  });
}

/** Observational handoff emitted after a child is registered but before it starts. */
export interface SpawnedRun {
  runId: string;
  runDir: string;
  parentSessionId: string;
  handle: SubagentHandle;
}

/** Child telemetry without a Handle reference, safe for long-lived aggregate observers. */
export interface ChildRunEvent {
  runId: string;
  runDir: string;
  parentSessionId: string;
  resolved?: ResolvedSpec;
  event: SubagentEvent;
}

class Handle implements SubagentHandle {
  readonly spec: SubagentSpec;
  readonly forkSessionFile: string | undefined;
  readonly followUpOf: FollowUpReference | undefined;
  resolved: ResolvedSpec | undefined;
  status: SubagentStatus = "pending";
  readonly startedAt = Date.now();
  readonly result: Promise<SubagentResult>;
  session?: ChildSession;
  schemaCapture?: SchemaCapture;
  private resolveResult!: (result: SubagentResult) => void;
  private listeners = new Set<(event: SubagentEvent) => void>();
  private steering: string[] = [];
  private terminal?: SubagentResult;
  private eventUnsubscribe?: () => void;
  private sessionDisposal?: Promise<void>;
  private constructionAbort?: Promise<void>;
  private aborting?: Promise<void>;
  private externalCancellationRequested = false;
  private startupDetached = false;
  /** Set while the initial run is executing runPrompt. */
  inFlightPrompt?: Promise<SubagentResult>;
  /** Idempotent release for the initial semaphore admission. */
  releaseAdmission?: () => void;
  /** Set while the initial run is queued for semaphore admission. */
  pendingInitialAdmission?: AbortController;
  /** Initial admission, construction, prompt, and cleanup task. */
  private startup?: Promise<void>;
  constructor(readonly id: string, spawnSpec: ChildSpawnSpec, readonly runId: string, readonly runDir: string,
    readonly generation: number, private store: RunStore, private runner: SubagentRunner, readonly parent: ParentContext) {
    this.spec = submittedSpec(spawnSpec);
    const followUp = followUpSpawn(spawnSpec);
    this.forkSessionFile = followUp?.forkSessionFile;
    this.followUpOf = followUp?.followUpOf;
    this.result = new Promise((resolve) => { this.resolveResult = resolve; });
  }
  emit(event: SubagentEvent): void {
    this.store.recordEvent(event);
    this.runner.publishChildEvent({
      runId: this.runId,
      runDir: this.runDir,
      parentSessionId: this.parent.ctx.sessionManager.getSessionId(),
      resolved: this.resolved,
      event,
    });
    for (const listener of this.listeners) {
      try {
        listener(event);
      } catch (error) {
        // UI/status subscribers are observational. One broken listener must
        // not prevent result settlement, delivery, or the remaining listeners.
        this.listeners.delete(listener);
        reportDiagnostic(`[subagent-workflow] child event listener failed and was detached: ${errorMessage(error)}`);
      }
    }
  }
  setStatus(status: SubagentStatus): void { this.status = status; this.emit({ type: "status", id: this.id, status }); }
  finish(result: SubagentResult): void {
    if (this.terminal) return;
    this.terminal = result;
    // The result event already carries the terminal status. Persist and emit it
    // once so RunStore can release ownership only after the complete result is
    // durable. A separate terminal status event would close the store too early.
    this.status = result.status;
    this.emit({ type: "result", id: this.id, result });
    this.resolveResult(result);
    this.runner.childFinished(this.runId);
  }
  async steer(text: string): Promise<void> { if (this.session) return this.session.steer(text); this.steering.push(text); }
  async flushSteering(): Promise<void> { for (const text of this.steering.splice(0)) await this.session?.steer(text); }
  get isTerminal(): boolean { return this.terminal !== undefined; }
  /** Whether the handle is doing live work right now. */
  get isLive(): boolean { return this.status === "pending" || this.status === "running"; }
  get isDisposingSession(): boolean { return this.sessionDisposal !== undefined; }
  get wasExternallyCancelled(): boolean { return this.externalCancellationRequested; }
  markExternalCancellation(): void { this.externalCancellationRequested = true; }
  abort(): Promise<void> {
    this.markExternalCancellation();
    if (this.aborting) return this.aborting;
    const aborting = this.abortOnce();
    this.aborting = aborting;
    void aborting.then(
      () => { if (this.aborting === aborting) this.aborting = undefined; },
      () => { if (this.aborting === aborting) this.aborting = undefined; },
    );
    return aborting;
  }
  private async abortOnce(): Promise<void> {
    if (this.pendingInitialAdmission) {
      this.pendingInitialAdmission.abort();
      await this.waitForStartup();
      return;
    }
    // The in-flight task stays authoritative through session shutdown and
    // worktree collection. Abort the session when it is still attached, then
    // let that task settle the result instead of synthesizing one here.
    const inFlightPrompt = this.inFlightPrompt;
    if (inFlightPrompt) {
      const abortAndSettle = (async () => {
        try {
          await this.session?.abort();
        } finally {
          await inFlightPrompt.catch(() => undefined);
          await this.waitForStartup();
        }
      })();
      const graceElapsed = await Promise.race([
        abortAndSettle.then(() => false),
        this.runner.waitForSettleGrace().then(() => true),
      ]);
      if (graceElapsed) {
        this.runner.abandonChild(this, `Child did not settle within ${SETTLE_GRACE_MS / 1_000}s of abort; terminated after the grace period`);
      }
      return;
    }
    if (this.terminal) {
      await this.waitForStartup();
      await this.runner.retire(this);
      return;
    }
    // Construction window: start() owns teardown so it can release admission
    // before awaiting extension shutdown.
    this.finish(this.runner.abortedResult(this));
    if (this.session) {
      this.constructionAbort ??= this.session.abort();
      await this.constructionAbort;
    }
    await this.waitForStartup();
  }
  /** Terminal or not, release the child session and its recursion-guard mark. */
  async dispose(): Promise<void> {
    if (!this.terminal || this.inFlightPrompt) await this.abort();
    await this.runner.finishDisposal(this);
  }
  setEventUnsubscribe(unsubscribe: () => void): void {
    this.eventUnsubscribe?.();
    this.eventUnsubscribe = unsubscribe;
  }
  clearEventSubscription(): void {
    this.eventUnsubscribe?.();
    this.eventUnsubscribe = undefined;
  }
  trackStartup(startup: Promise<void>): void {
    this.startup = startup.catch(() => undefined);
  }
  async waitForStartup(): Promise<void> {
    if (!this.startupDetached) await this.startup;
  }
  detachStartup(): void { this.startupDetached = true; }
  async waitForConstructionAbort(): Promise<void> {
    await this.constructionAbort?.catch(() => undefined);
  }
  async disposeSession(): Promise<void> {
    this.clearEventSubscription();
    if (this.sessionDisposal) return this.sessionDisposal;
    const session = this.session;
    if (!session) return;
    // Clear the public session reference before awaiting extension shutdown so
    // no navigator action can reuse a session being torn down.
    this.session = undefined;
    // Defer shutdown by one microtask so reentrant shutdown handlers always see
    // sessionDisposal and cannot retire this handle before teardown completes.
    const disposal = Promise.resolve().then(async () => {
      try {
        // Awaits real process exit, so callers that run after disposal
        // (worktree collection) never race a live child.
        await session.dispose();
        try {
          // Terminal result publication can already have released run ownership,
          // so continuation safety uses its own durable per-child publication.
          writeSessionClosedMarker(this.runDir, this.id);
        } catch (error) {
          reportDiagnostic(`[subagent-workflow] child session closure marker failed: ${errorMessage(error)}`);
        }
      } catch (error) {
        reportDiagnostic(`[subagent-workflow] child session disposal failed: ${errorMessage(error)}`);
      } finally {
        try {
          this.store.disposed();
        } catch (error) {
          reportDiagnostic(`[subagent-workflow] child disposal persistence failed: ${errorMessage(error)}`);
        }
      }
    });
    this.sessionDisposal = disposal;
    try {
      await disposal;
    } finally {
      if (this.sessionDisposal === disposal) this.sessionDisposal = undefined;
    }
  }
  subscribe(listener: (event: SubagentEvent) => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
}

export class SubagentRunner {
  private handles = new Map<string, Handle>();
  private stores = new Map<string, RunStore>();
  private projections = new Map<string, {
    projection: RunProjection;
    retainAfterDelivery: boolean;
    ownsRun: () => boolean;
  }>();
  private runControllers = new Map<string, {
    controller: AbortController;
    parentSessionId: string;
    execution: Promise<unknown>;
    isExpectedStop: (error: unknown) => boolean;
  }>();
  private childCounter = 0;
  private finalizedRuns = new Set<string>();
  private deliveredRuns = new Set<string>();
  private agentTimeoutMinutes = 0;
  private spawnListeners = new Set<(run: SpawnedRun) => void>();
  private childEventListeners = new Set<(event: ChildRunEvent) => void>();
  // Per-process nonce so child IDs never collide across a process restart. On
  // resume a fresh process restarts childCounter at 0; without the nonce the
  // first newly-spawned child would reuse an existing persisted child's ID and
  // corrupt the run record. Child IDs are not part of the resume-matching
  // contract (that is async-lineage identity + payload hash), so process
  // uniqueness is all they need.
  private readonly nonce = randomUUID().replaceAll("-", "").slice(0, 8);
  constructor(
    private buildChild: ChildBuilder = spawnSubprocessChild,
    private semaphore: Semaphore = globalSemaphore,
    private buildStore: StoreBuilder = (runId, parent, storeOptions) =>
      new RunStore(runId, parent.ctx.cwd, parent.ctx.sessionManager.getSessionId(), parent.ctx.sessionManager.getSessionFile(), storeOptions),
    private delay: Delay = defaultDelay,
  ) {}

  /** Apply a process-wide admission limit. Existing agents are never cancelled. */
  setMaxConcurrentAgents(capacity: number): void { this.semaphore.resize(capacity); }

  /**
   * Set the wall timeout for newly admitted child prompts. Zero disables
   * timeouts for later admissions. Active prompts keep their original timeout.
   */
  setAgentTimeoutMinutes(minutes: number): void {
    if (!Number.isFinite(minutes) || minutes < 0) throw new Error("Agent timeout minutes must be a non-negative number");
    this.agentTimeoutMinutes = minutes;
  }

  /** Observe new child fan-outs. Telemetry listeners cannot affect runner lifecycle. */
  subscribeSpawns(listener: (run: SpawnedRun) => void): () => void {
    this.spawnListeners.add(listener);
    return () => this.spawnListeners.delete(listener);
  }

  /** Observe child telemetry without retaining child handles or sessions. */
  subscribeChildEvents(listener: (event: ChildRunEvent) => void): () => void {
    this.childEventListeners.add(listener);
    return () => this.childEventListeners.delete(listener);
  }

  /** @internal Handle-to-runner telemetry handoff. */
  publishChildEvent(event: ChildRunEvent): void {
    const projection = this.projections.get(event.runId)?.projection;
    if (projection) foldProjection(projection, { ...event.event, timestamp: Date.now() } as RunProjectionEvent);
    for (const listener of this.childEventListeners) {
      try {
        listener(event);
      } catch (error) {
        reportDiagnostic(`[subagent-workflow] child observer failed: ${errorMessage(error)}`);
      }
    }
  }

  /** @internal Handle-to-runner grace timer handoff. */
  waitForSettleGrace(): Promise<void> { return this.delay(SETTLE_GRACE_MS); }

  /** @internal Settle a child whose RPC process ignored cancellation, then terminate it. */
  abandonChild(handle: Handle, reason: string): void {
    if (handle.isTerminal) return;
    // Settle the result immediately; disposal below kills the child process.
    handle.finish(this.failure(handle, reason));
    handle.releaseAdmission?.();
    handle.clearEventSubscription();
    void handle.disposeSession().catch((error: unknown) => {
      reportDiagnostic(`[subagent-workflow] abandoned child ${handle.id} session disposal failed: ${errorMessage(error)}`);
    });
    reportDiagnostic(`[subagent-workflow] child ${handle.id} was abandoned after the grace period: ${reason}`);
  }

  /** @internal Bound post-abort startup and session disposal. */
  async finishDisposal(handle: Handle): Promise<void> {
    const disposal = (async () => {
      await handle.waitForStartup();
      await handle.disposeSession();
    })();
    const graceElapsed = await Promise.race([
      disposal.then(() => false),
      this.waitForSettleGrace().then(() => true),
    ]);
    if (!graceElapsed) return;
    handle.detachStartup();
    void disposal.catch((error: unknown) => {
      reportDiagnostic(`[subagent-workflow] child ${handle.id} late disposal failed: ${errorMessage(error)}`);
    });
    reportDiagnostic(`[subagent-workflow] child ${handle.id} disposal did not settle within ${SETTLE_GRACE_MS / 1_000}s; continuing shutdown`);
  }

  spawn(spec: SubagentSpec, parent: ParentContext): SubagentHandle {
    return this.spawnRun(spec, parent);
  }

  spawnRun(spec: ChildSpawnSpec, parent: ParentContext, options: SpawnRunOptions = {}): SubagentHandle {
    const runId = options.runId ?? `run-${Date.now().toString(36)}-${randomUUID().replaceAll("-", "").slice(0, 16)}`;
    const existingStore = this.stores.get(runId);
    const store = options.store ?? existingStore
      ?? this.buildStore(runId, parent, options.directDelivery ? { directDelivery: true } : undefined);
    const ownsUnregisteredStore = options.store === undefined && existingStore === undefined;
    const identity = store.deliveryIdentity;
    if (!identity || identity.generation < 1) {
      if (ownsUnregisteredStore) store.releaseOwnership();
      throw new Error(`Run ${runId} has no active delivery generation`);
    }
    // Strip the kind prefix, whatever it is: slice(4) assumed "run-" and
    // turned workflow run ids into "subagent-flow-..." child ids.
    const id = `subagent-${runId.replace(/^[a-z]+-/, "")}-${this.nonce}-${(++this.childCounter).toString(36)}`;
    const handle = new Handle(id, spec, runId, store.runDir, identity.generation, store, this, parent);
    // Persist the child before it starts. If ownership was lost, the
    // unreturned child cannot run as an orphan and no fenced store is
    // retained in the runner registry.
    try {
      store.addChild(handle.id, handle.spec, handle.followUpOf);
    } catch (error) {
      if (ownsUnregisteredStore) store.releaseOwnership();
      throw error;
    }
    const tracked = this.projections.get(runId);
    if (tracked) {
      tracked.retainAfterDelivery ||= options.store !== undefined;
      tracked.ownsRun = () => store.ownsRun;
      foldProjection(tracked.projection, {
        type: "child",
        id: handle.id,
        spec: handle.spec,
        followUpOf: handle.followUpOf,
      });
    } else {
      this.projections.set(runId, {
        projection: projectRunSnapshot(readRunSnapshot(store.runDir), runId),
        retainAfterDelivery: options.store !== undefined,
        ownsRun: () => store.ownsRun,
      });
    }
    this.stores.set(runId, store);
    this.handles.set(handle.id, handle);
    this.notifySpawn({
      runId,
      runDir: store.runDir,
      parentSessionId: parent.ctx.sessionManager.getSessionId(),
      handle,
    });
    handle.trackStartup(this.start(handle, store));
    return handle;
  }

  get(id: string): SubagentHandle | undefined { return this.handles.get(id); }
  /** Live handles belonging to a run, for the navigator's live overlay. */
  runHandles(runId: string): SubagentHandle[] { return [...this.handles.values()].filter((handle) => handle.runId === runId); }
  /** Seed a workflow projection from durable history before live execution resumes. */
  adoptRunProjection(runId: string, store: RunStore, snapshot: RunSnapshot, workflowLabel: string): void {
    this.projections.set(runId, {
      projection: projectRunSnapshot(snapshot, runId, { workflowLabel }),
      retainAfterDelivery: true,
      ownsRun: () => store.ownsRun,
    });
  }
  /** Fold workflow narration and lifecycle events that do not originate from a child handle. */
  foldRunProjection(runId: string, event: RunProjectionEvent): void {
    const projection = this.projections.get(runId)?.projection;
    if (projection) foldProjection(projection, event);
  }
  /** Full in-memory view for a run currently owned by this process. */
  runProjection(runId: string): RunProjection | undefined {
    const tracked = this.projections.get(runId);
    return tracked?.ownsRun() ? cloneRunProjection(tracked.projection) : undefined;
  }
  /** Incremental activity retained through workflow result delivery. */
  runActivityFold(runId: string): RunActivityFold | undefined {
    const projection = this.projections.get(runId)?.projection;
    return projection ? cloneActivityFold(projection.activity) : undefined;
  }
  /** Release a retained workflow projection after its result has been delivered. */
  releaseRunActivity(runId: string): void { this.projections.delete(runId); }
  /** Run ids with a live child or an active workflow controller in this process. */
  liveRunIds(): string[] {
    const ids = new Set(this.runControllers.keys());
    for (const handle of this.handles.values()) if (handle.isLive) ids.add(handle.runId);
    return [...ids];
  }
  /** The RPC-backed session for a live child, for transcript following. Undefined once disposed. */
  liveSession(childId: string): ChildSession | undefined { return this.handles.get(childId)?.session; }
  childFinished(runId: string): void {
    const runHandles = [...this.handles.values()].filter((handle) => handle.runId === runId);
    if (runHandles.length > 0 && runHandles.every((handle) => handle.isTerminal)) {
      // No store write here: statuses/usage were already persisted per result
      // event, and full results live in events.jsonl - copying them into
      // run.json would double-store the largest payloads (worktree patches).
      this.finalizedRuns.add(runId);
      this.dropDeliveredStore(runId);
    }
  }
  /** Read the finalized run warning before queueing its parent message. */
  finalizedRunWarning(runId: string): string | undefined {
    return this.stores.get(runId)?.persistenceDegraded;
  }
  /**
   * Release finalized in-memory resources after inline handoff or background
   * queueing. The historical method name does not indicate durable delivery.
   */
  markDelivered(runId: string): string | undefined {
    const store = this.stores.get(runId);
    // A workflow with no agent() calls never registers a store with the runner.
    // There is nothing to retain, and recording its id forever would leak one
    // deliveredRuns entry per zero-agent workflow.
    if (!store && ![...this.handles.values()].some((handle) => handle.runId === runId)) return undefined;
    this.deliveredRuns.add(runId);
    this.dropDeliveredStore(runId);
    if (!this.projections.get(runId)?.retainAfterDelivery) this.projections.delete(runId);
    return store?.persistenceDegraded;
  }
  abortedResult(handle: Handle): SubagentResult {
    return { id: handle.id, generation: handle.generation, status: "aborted", ...sessionFileFrom(handle.session), text: "", usage: usageFrom(handle.session), resolved: this.fallbackResolved(handle) };
  }
  /** A workflow run registers a controller so a run-level stop cancels its loop, not just its current children. */
  registerRunController(runId: string, controller: AbortController, parentSessionId: string, execution: Promise<unknown>,
    isExpectedStop: (error: unknown) => boolean = () => false): void {
    this.runControllers.set(runId, { controller, parentSessionId, execution, isExpectedStop });
  }
  unregisterRunController(runId: string): void { this.runControllers.delete(runId); }
  /** Stop an entire run and wait for its workflow teardown or direct children. */
  async stopRun(runId: string): Promise<void> {
    const registered = this.runControllers.get(runId);
    if (registered) {
      registered.controller.abort();
      await registered.execution.catch(() => undefined);
      return;
    }
    const live = [...this.handles.values()].filter((handle) => handle.runId === runId && handle.isLive);
    await Promise.all(live.map((handle) => handle.abort()));
  }
  /** Whether the run has live work or a registered workflow loop in THIS process; guards same-process duplicate resume. */
  isRunActive(runId: string): boolean {
    if (this.runControllers.has(runId)) return true;
    for (const handle of this.handles.values()) if (handle.runId === runId && handle.isLive) return true;
    return false;
  }
  async disposeForSession(parentSessionId: string): Promise<void> {
    // Abort workflow loops first. Otherwise a background workflow can spawn a
    // new child after the handle snapshot below and keep the run owned after
    // the parent session that owns it has shut down.
    const executions: Promise<unknown>[] = [];
    for (const { controller, parentSessionId: owner, execution, isExpectedStop } of this.runControllers.values()) {
      if (owner !== parentSessionId) continue;
      controller.abort();
      executions.push(execution.catch((error: unknown) => {
        if (isExpectedStop(error)) return;
        throw error;
      }));
    }
    const tasks = [
      ...executions,
      ...[...this.handles.values()].filter((handle) => handle.parent.ctx.sessionManager.getSessionId() === parentSessionId).map(async (handle) => {
        await handle.dispose();
        await this.retire(handle);
      }),
    ];
    const results = await Promise.allSettled(tasks);
    const failure = results.find((result): result is PromiseRejectedResult => result.status === "rejected");
    if (failure) throw failure.reason;
  }

  private notifySpawn(run: SpawnedRun): void {
    for (const listener of this.spawnListeners) {
      try {
        listener(run);
      } catch (error) {
        reportDiagnostic(`[subagent-workflow] run observer failed: ${errorMessage(error)}`);
      }
    }
  }

  private async start(handle: Handle, store: RunStore): Promise<void> {
    const admission = new AbortController();
    handle.pendingInitialAdmission = admission;
    let release: () => void;
    const agentTimeoutMinutes = this.agentTimeoutMinutes;
    try {
      release = await this.semaphore.acquire(admission.signal);
    } catch (error) {
      const result = admission.signal.aborted ? this.abortedResult(handle) : this.failure(handle, error);
      handle.finish(result);
      return;
    } finally {
      handle.pendingInitialAdmission = undefined;
    }
    let released = false;
    const releaseAdmission = (): void => {
      if (released) return;
      released = true;
      release();
    };
    handle.releaseAdmission = releaseAdmission;
    const disposeSessionAndRetire = async (): Promise<void> => {
      // Terminal work no longer needs model capacity. Release it before waiting
      // for construction abort or extension shutdown, either of which can stall.
      releaseAdmission();
      await handle.waitForConstructionAbort();
      await handle.disposeSession();
      void this.retire(handle);
    };
    let worktree: { sourceCwd: string; tree: Worktree } | undefined;
    let retainWorktree = false;
    try {
      if (admission.signal.aborted && !handle.isTerminal) handle.finish(this.abortedResult(handle));
      if (handle.isTerminal) return;
      handle.setStatus("running");
      let childSpec = handle.spec;
      if (handle.spec.isolation === "worktree") {
        const sourceCwd = handle.spec.cwd ?? handle.parent.ctx.cwd;
        const tree = await createWorktree(sourceCwd, `${store.runDir}/worktrees/${handle.id}`);
        worktree = { sourceCwd, tree };
        childSpec = { ...handle.spec, cwd: tree.cwd };
      }
      if (handle.isTerminal) {
        releaseAdmission();
        return;
      }
      const child = await this.buildChild(childSpec, handle.parent, {
        sessionsDir: store.sessionsDir,
        forkSessionFile: handle.forkSessionFile,
      });
      handle.session = child.session;
      if (handle.isTerminal) {
        await disposeSessionAndRetire();
        return;
      }
      handle.resolved = child.resolved; handle.schemaCapture = child.schemaCapture;
      if (worktree) handle.resolved.worktreePath = worktree.tree.path;
      store.resolveChild(handle.id, child.resolved, child.session.sessionFile);
      const projection = this.projections.get(handle.runId)?.projection;
      if (projection) foldProjection(projection, {
        type: "resolved",
        id: handle.id,
        resolved: child.resolved,
        sessionFile: child.session.sessionFile,
      });
      this.subscribe(handle);
      await handle.flushSteering();
      if (handle.isTerminal) {
        await disposeSessionAndRetire();
        return;
      }
      // Track the run so abort() can defer to this path (which collects the
      // worktree patch and yields an aborted result) instead of racing it.
      const runAndFinish = (async (): Promise<SubagentResult> => {
        const result = await this.runPrompt(handle, handle.spec.prompt, agentTimeoutMinutes);
        releaseAdmission();
        // Shutdown hooks can make final worktree writes. Dispose the session
        // before collecting so those writes are included in the patch.
        await handle.disposeSession();
        if (worktree) {
          try {
            const changes = await collectWorktree(worktree.tree);
            result.patch = changes.patch;
            result.changed = changes.changed;
          } catch (error) {
            // Collection failed (e.g. a diff too large to buffer). The child's
            // work is still in the worktree - keep it and report where, rather
            // than deleting the only copy in the finally below.
            retainWorktree = true;
            const path = error instanceof WorktreeCollectionError ? error.worktreePath : worktree.tree.path;
            result.status = "failed";
            result.error = `${errorMessage(error)} - worktree retained at ${path}`;
          }
        }
        handle.finish(result);
        return result;
      })();
      handle.inFlightPrompt = runAndFinish;
      try { await runAndFinish; } finally { handle.inFlightPrompt = undefined; }
    } catch (error) {
      handle.finish(this.failure(handle, error));
      await disposeSessionAndRetire();
    } finally {
      if (worktree && !retainWorktree) {
        try {
          await cleanupWorktree(worktree.sourceCwd, worktree.tree.path);
        } catch (error) {
          reportDiagnostic(`[subagent-workflow] ${errorMessage(error)}`);
        }
      }
      releaseAdmission();
    }
  }

  async runPrompt(handle: Handle, prompt: string, agentTimeoutMinutes: number): Promise<SubagentResult> {
    const session = handle.session;
    const capture = handle.schemaCapture;
    if (!session || !handle.resolved) return this.failure(handle, "Child session is unavailable");
    const clearTimeout = this.trackPromptTimeout(handle, session, agentTimeoutMinutes);
    try {
      // session.prompt() runs a full agent loop and resolves when settled.
      await session.prompt(prompt);
      let message = this.extractLatestAssistant(session);
      // A schema child self-terminates through report_result -> session.abort(),
      // so a captured result may override an aborted assistant only when that
      // capture was the sole cancellation source. User abort, parent shutdown,
      // and timeout provenance stays authoritative even if capture publishes late.
      if (handle.wasExternallyCancelled) return this.resultFor(handle, "aborted", message);
      if (capture?.called) return { ...this.resultFor(handle, "completed", message), structured: capture.value };
      // Never turn an aborted or failed model run into a fresh repair run. In
      // particular, a timeout has already fired and cannot bound a second
      // prompt, so repair here would defeat the configured wall limit.
      if (message?.stopReason === "aborted") return this.resultFor(handle, "aborted", message);
      if (message?.stopReason === "error") return this.failure(handle, message.errorMessage ?? "Child model request failed", message);
      if (capture) {
        await session.prompt(STRUCTURED_REPAIR_PROMPT);
        message = this.extractLatestAssistant(session);
        if (handle.wasExternallyCancelled) return this.resultFor(handle, "aborted", message);
        if (capture.called) return { ...this.resultFor(handle, "completed", message), structured: capture.value };
        if (message?.stopReason === "aborted") return this.resultFor(handle, "aborted", message);
        if (message?.stopReason === "error") return this.failure(handle, message.errorMessage ?? "Child model request failed", message);
        return this.failure(handle, "Child did not call report_result after one repair attempt", message);
      }
      return this.resultFor(handle, "completed", message);
    } catch (error) {
      if (handle.wasExternallyCancelled) {
        const message = this.extractLatestAssistant(session);
        return this.resultFor(handle, "aborted", message);
      }
      return this.failure(handle, error);
    } finally { clearTimeout(); }
  }

  private subscribe(handle: Handle): void {
    const session = handle.session;
    const unsubscribe = session?.subscribe((event) => {
      if (event.type === "tool_execution_start") {
        const activityEvent = { type: "activity" as const, id: handle.id, description: summarizeCall(event.toolName, event.args) };
        handle.emit(activityEvent);
      }
      if (event.type === "turn_end" && event.message.role === "assistant") {
        handle.emit({ type: "usage", id: handle.id, usage: { ...session.usage } });
      }
    });
    if (unsubscribe) handle.setEventUnsubscribe(unsubscribe);
  }
  private extractLatestAssistant(session: ChildSession): AssistantMessage | undefined {
    const message = session.latestAssistant;
    session.clearLatestAssistant?.();
    return message;
  }
  private resultFor(handle: Handle, status: "completed" | "aborted", message?: AssistantMessage): SubagentResult {
    return { id: handle.id, generation: handle.generation, status, ...sessionFileFrom(handle.session), text: assistantText(message), usage: usageFrom(handle.session), resolved: handle.resolved! };
  }
  private failure(handle: Handle, error: unknown, message?: AssistantMessage): SubagentResult {
    const fallback = this.fallbackResolved(handle);
    const extracted = message ?? (handle.session ? this.extractLatestAssistant(handle.session) : undefined);
    return { id: handle.id, generation: handle.generation, status: "failed", ...sessionFileFrom(handle.session), text: assistantText(extracted), error: errorMessage(error), usage: usageFrom(handle.session), resolved: fallback };
  }
  private fallbackResolved(handle: Handle): ResolvedSpec {
    return handle.resolved ?? { provider: "unknown", modelId: "unknown", thinkingLevel: "off", tools: [], cwd: handle.spec.cwd ?? handle.parent.ctx.cwd, label: handle.spec.label ?? "Subagent" };
  }
  private trackPromptTimeout(handle: Handle, session: ChildSession, agentTimeoutMinutes: number): () => void {
    if (agentTimeoutMinutes === 0) return () => {};
    let cancelled = false;
    const timer = setTimeout(() => {
      handle.markExternalCancellation();
      void session.abort().catch((error: unknown) => {
        reportDiagnostic(`[subagent-workflow] timed-out child abort failed: ${errorMessage(error)}`);
      });
      void this.waitForSettleGrace().then(() => {
        if (cancelled || handle.isTerminal) return;
        this.abandonChild(handle, `Child timed out and did not settle within ${SETTLE_GRACE_MS / 1_000}s grace; terminated`);
      }).catch((error: unknown) => {
        reportDiagnostic(`[subagent-workflow] timed-out child grace wait failed: ${errorMessage(error)}`);
      });
    }, agentTimeoutMinutes * 60_000);
    timer.unref?.();
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }
  /** Drop a handle from the registry once it is terminal, its session is gone, and its run was delivered. */
  async retire(handle: Handle): Promise<void> {
    await handle.waitForStartup();
    if (handle.isTerminal && !handle.session && !handle.isDisposingSession && this.deliveredRuns.has(handle.runId)) {
      this.handles.delete(handle.id);
      this.cleanupRunTracking(handle.runId);
    }
  }
  private dropDeliveredStore(runId: string): void {
    if (!this.finalizedRuns.has(runId) || !this.deliveredRuns.has(runId)) return;
    this.stores.delete(runId);
    // Session disposal can finish just after delivery. Retire every handle that
    // is already clean, then drop the per-run markers once the last one leaves.
    for (const handle of [...this.handles.values()]) {
      if (handle.runId === runId) void this.retire(handle);
    }
    this.cleanupRunTracking(runId);
  }
  private cleanupRunTracking(runId: string): void {
    if (this.stores.has(runId)) return;
    if ([...this.handles.values()].some((handle) => handle.runId === runId)) return;
    this.finalizedRuns.delete(runId);
    this.deliveredRuns.delete(runId);
  }
}

function assistantText(message?: AssistantMessage): string { return message?.content.filter((part) => part.type === "text").map((part) => part.text).join("\n") ?? ""; }
function usageFrom(session?: ChildSession): UsageSummary {
  if (!session) return EMPTY_USAGE();
  return { ...session.usage };
}
function sessionFileFrom(session?: ChildSession): Pick<SubagentResult, "sessionFile"> {
  return session?.sessionFile ? { sessionFile: session.sessionFile } : {};
}
function summarizeCall(name: string, args: unknown): string {
  const summary = JSON.stringify(args) ?? "";
  return `${name} ${summary.replace(/\s+/g, " ").slice(0, 120)}`.trim();
}

// Also a process singleton (see globalSemaphore) so every module instance,
// and the tools they register, share one handle/store registry.
// Bump whenever Handle/SubagentRunner state or behavior changes incompatibly.
// A separate key from the semaphore lets a hot reload adopt the new runner
// without temporarily creating a second concurrency pool.
const RUNNER_STATE_VERSION = "v17";
const RUNNER_KEY = `__piSubagentWorkflowRunner_${RUNNER_STATE_VERSION}__`;
const runnerScope = globalThis as unknown as Record<string, SubagentRunner | undefined>;
export const subagentRunner: SubagentRunner = runnerScope[RUNNER_KEY] ??= new SubagentRunner();

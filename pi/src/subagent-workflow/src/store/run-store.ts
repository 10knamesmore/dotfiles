import { createHash } from "node:crypto";
import { appendFileSync, closeSync, existsSync, fsyncSync, mkdirSync, openSync, readFileSync, readSync, rmSync, statSync, writeSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import type { FollowUpReference, ResolvedSpec, SubagentEvent, SubagentSpec, SubagentStatus, UsageSummary, WorkflowPhase } from "../types.js";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage, isRecord } from "../util.js";
import { commitAtomicFile, discardAtomicFile, replaceAtomicFile, stageAtomicFile, syncDirectoryDurably } from "./atomic-file.js";
import {
  DELIVERED_FILE,
  DELIVERY_PROTOCOL_VERSION,
  parseRunDeliveryIdentity,
  type RunDeliveryIdentity,
} from "./delivery-marker.js";
import {
  acquireRunOwnership,
  RunOwnershipConflictError,
  type RunOwnership,
} from "./lease.js";
import type { RunProjection } from "./run-projection.js";
import { readRunSnapshot, type FrozenJson, type RunSnapshot } from "./run-snapshot.js";
import { writeSessionClosedMarker } from "./session-closed-marker.js";

export const EMPTY_USAGE = (): UsageSummary => ({ input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0, turns: 0 });

export function sumUsage(usages: Iterable<UsageSummary>): UsageSummary {
  const total = EMPTY_USAGE();
  for (const usage of usages) {
    total.input += usage.input; total.output += usage.output; total.cacheRead += usage.cacheRead;
    total.cacheWrite += usage.cacheWrite; total.cost += usage.cost; total.turns += usage.turns;
  }
  return total;
}

export const reconcileProjectionWrites = {
  writeStatus(path: string, contents: string): void {
    replaceAtomicFile(path, contents, {
      mode: 0o600,
      fsync: true,
      syncParentDirectory: true,
    });
  },
};

/** Persist a dead-owner projection while the caller holds run ownership. */
export function persistReconciledProjection(
  snapshot: RunSnapshot,
  projection: RunProjection,
  generation: number,
  interruptedChildIds: readonly string[],
  writeStatus: (path: string, contents: string) => void = reconcileProjectionWrites.writeStatus,
): void {
  const projectedStatus = isTerminalStatus(projection.summary.status) ? projection.summary.status : undefined;
  const status: unknown = structuredClone(snapshot.status);
  if (!projectedStatus || !isRecord(status)) throw new Error("run status is not reconcilable");
  const persistedChildren = isRecord(status.children) ? status.children : {};
  status.children = Object.fromEntries(projection.detail.children.map((child) => {
    const persisted = persistedChildren[child.id];
    return [child.id, {
      status: isTerminalStatus(child.status) ? child.status : "aborted",
      usage: isRecord(persisted) && isUsageSummary(persisted.usage) ? structuredClone(persisted.usage) : EMPTY_USAGE(),
    }];
  }));
  status.status = projectedStatus;

  const hasCrashEvent = snapshot.events.some((value) => {
    return isRecord(value) && value.type === "crash_reconciled" && value.generation === generation;
  });
  if (!hasCrashEvent) {
    if (snapshot.rawEvents === undefined) throw new Error("events.jsonl is not readable");
    const separator = snapshot.rawEvents.length > 0 && !snapshot.rawEvents.endsWith("\n") ? "\n" : "";
    const event = {
      timestamp: new Date().toISOString(),
      type: "crash_reconciled",
      generation,
      status: projectedStatus,
      interruptedChildIds,
    };
    replaceAtomicFile(join(snapshot.runDir, "events.jsonl"), `${snapshot.rawEvents}${separator}${JSON.stringify(event)}\n`, {
      mode: 0o600,
      fsync: true,
      syncParentDirectory: true,
    });
  }
  writeStatus(join(snapshot.runDir, "status.json"), `${JSON.stringify(status, null, 2)}\n`);
}

interface ChildRecord {
  id: string;
  spec: SubagentSpec;
  resolved?: ResolvedSpec;
  sessionFile?: string;
  followUpOf?: FollowUpReference;
  /** Legacy records stored phase beside spec. */
  phase?: string;
}

interface RunRecord {
  v?: 2 | 3;
  runId: string;
  kind: "subagent" | "workflow";
  createdAt: string;
  parent: { sessionId: string; sessionFile?: string };
  children: ChildRecord[];
  phases?: Array<{ title: string; detail?: string }>;
  workflowPolicy?: { maxAgentsPerWorkflow: number };
  delivery?: RunDeliveryIdentity;
  /** Results go directly to a human (navigator follow-up); catch-up must never
   * queue them to the model. If a third delivery mode ever appears, replace
   * this boolean with a delivery-policy value rather than adding a sibling. */
  directDelivery?: true;
}

interface RunStatus {
  status: SubagentStatus;
  children: Record<string, { status: SubagentStatus; usage: UsageSummary }>;
}

interface RunStoreOptions {
  rootDir?: string;
  now?: () => Date;
  kind?: "subagent" | "workflow";
  phases?: readonly Readonly<{ title: string; detail?: string }>[];
  maxAgentsPerWorkflow?: number;
  /** Persisted in run.json before any child starts, so a crash can never leave the run catch-up eligible. */
  directDelivery?: boolean;
  existingRunDir?: string;
  /** Policy-neutral pre-lock read supplied by a caller that already resolved the run. */
  existingSnapshot?: RunSnapshot;
}

interface StagingFileOperations {
  open(path: string, flags: string, mode: number): number;
  write(fd: number, buffer: Uint8Array, offset: number, length: number): number;
  close(fd: number): void;
  remove(path: string): void;
}

const STAGING_FILE_OPERATIONS: StagingFileOperations = {
  open: (path, flags, mode) => openSync(path, flags, mode),
  write: (fd, buffer, offset, length) => writeSync(fd, buffer, offset, length),
  close: (fd) => closeSync(fd),
  remove: (path) => rmSync(path, { force: true }),
};

const GENERATION_PENDING_FILE = "generation.pending";

/** Create a complete staging file without ever truncating a colliding path. */
export function stageFileExclusive(
  path: string,
  content: string,
  operations: StagingFileOperations = STAGING_FILE_OPERATIONS,
): string {
  mkdirSync(dirname(path), { recursive: true });
  return stageAtomicFile(path, content, { mode: 0o600, operations });
}

export class RunStoreOwnershipError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "RunStoreOwnershipError";
  }
}

export class GenerationPendingError extends Error {
  constructor(readonly runId: string, readonly runDir: string) {
    super(`Cannot resume run ${runId}: ${GENERATION_PENDING_FILE} was left by a previous generation commit that crashed, so the run is inconsistent and cannot be resumed. Re-run the workflow fresh; delete the run directory to clean it up: ${runDir}`);
    this.name = "GenerationPendingError";
  }
}

export function encodeCwd(cwd: string): string {
  const resolved = resolve(cwd);
  // A readable slug for humans browsing the runs directory, plus a short hash
  // of the full path so distinct projects never collide: without it "/a/b",
  // "/a-b", and "/a:b" all slug to the same "a-b".
  const slug = resolved.replace(/^[/\\]/, "").replace(/[/\\:]/g, "-");
  // 64-bit hash: wide enough that distinct paths sharing a slug do not collide
  // in practice (8 hex digits left a birthday collision reachable with crafted
  // paths).
  const hash = createHash("sha256").update(resolved).digest("hex").slice(0, 16);
  return `--${slug}--${hash}`;
}

/** Owns all writes in a run directory. Initial layout is mandatory; later writes are best-effort. */
export class RunStore {
  readonly runDir: string;
  readonly sessionsDir: string;
  private record: RunRecord;
  private status: RunStatus = { status: "pending", children: {} };
  private readonly now: () => Date;
  private readonly runId: string;
  private openedSnapshot?: RunSnapshot;
  private runOwnership?: RunOwnership;
  /** Ownership acquisition is attempted exactly once during construction. */
  private ownershipAttempted = false;
  /** Set at any ownership release point; afterwards writes stay closed. */
  private writesClosed = false;
  private degradedReason?: string;

  constructor(runId: string, parentCwd: string, parentSessionId: string, parentSessionFile?: string, options: RunStoreOptions = {}) {
    this.now = options.now ?? (() => new Date());
    this.runId = runId;
    const root = options.rootDir ?? join(getAgentDir(), "subagent-workflow", "runs");
    this.runDir = options.existingRunDir ?? join(root, encodeCwd(parentCwd), runId);
    this.sessionsDir = join(this.runDir, "sessions");
    if (options.existingRunDir) {
      // An atomically-created directory is the new-run reservation. Validate
      // that initialization reached its metadata commit before contending for
      // ownership, so a resume cannot seize the tiny initialization window.
      const initialSnapshot = options.existingSnapshot ?? readRunSnapshot(this.runDir);
      this.validateResumeSnapshot(initialSnapshot);
      this.ensureOwnership();
      try {
        // Close the check-to-lock race with a generation commit whose owner
        // exited after publishing its marker.
        const currentSnapshot = readRunSnapshot(this.runDir);
        if (currentSnapshot.generationPending) {
          throw new GenerationPendingError(this.runId, this.runDir);
        }
        const { record, status } = this.validateResumeSnapshot(currentSnapshot);
        this.record = cloneFrozenJson(record as unknown as FrozenJson) as unknown as RunRecord;
        this.status = cloneFrozenJson(status as unknown as FrozenJson) as unknown as RunStatus;
        let resumed: { event: FrozenJson; line: string } | undefined;
        this.writeOwned(() => { resumed = this.appendLifecycle("resumed"); }, { ownershipRequired: true });
        this.openedSnapshot = withAppendedEvent(currentSnapshot, resumed!);
      } catch (error) {
        this.releaseOwnership();
        throw error;
      }
      return;
    }
    const kind = options.kind ?? "subagent";
    this.record = {
      v: 3,
      runId,
      kind,
      createdAt: this.now().toISOString(),
      parent: { sessionId: parentSessionId, sessionFile: parentSessionFile },
      children: [],
      phases: options.phases?.map((phase) => ({ ...phase })),
      delivery: {
        protocol: DELIVERY_PROTOCOL_VERSION,
        generation: kind === "workflow" ? 0 : 1,
      },
      ...(kind === "workflow" && options.maxAgentsPerWorkflow !== undefined
        ? { workflowPolicy: { maxAgentsPerWorkflow: options.maxAgentsPerWorkflow } }
        : {}),
      ...(options.directDelivery ? { directDelivery: true as const } : {}),
    };
    this.validateWorkflowPolicy(this.record);
    // Reserve the run id before touching metadata. A repeated id must never
    // overwrite an existing run, regardless of whether that run still has a
    // live owner.
    mkdirSync(dirname(this.runDir), { recursive: true, mode: 0o700 });
    try {
      mkdirSync(this.runDir, { mode: 0o700 });
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "EEXIST") {
        throw new Error(`Run ${runId} already exists at ${this.runDir}; generate a new run id`);
      }
      throw error;
    }
    try {
      this.ensureOwnership();
      mkdirSync(this.sessionsDir, { mode: 0o700 });
      this.writeJson("run.json", this.record);
      this.writeJson("status.json", this.status);
      this.appendLifecycle("created");
    } catch (error) {
      this.releaseOwnershipHandle();
      // Keep the exclusively-reserved directory on a partial initialization.
      // Removing it after releasing ownership would race a recovery process
      // that validated the metadata and acquired ownership in between.
      throw error;
    }
  }

  /** Whether this store still holds the process's ownership transaction. */
  get ownsRun(): boolean {
    return this.runOwnership !== undefined && !this.writesClosed;
  }

  /** First persistence failure after initialization, if any. The run is then not reliably resumable. */
  get persistenceDegraded(): string | undefined {
    return this.degradedReason;
  }

  /** Run kind is immutable and lets the shared runner apply lifecycle policy without guessing from the id. */
  get kind(): RunRecord["kind"] {
    return this.record.kind;
  }

  /** Protocol identity captured by completion results and delivery acknowledgements. */
  get deliveryIdentity(): RunDeliveryIdentity | undefined {
    const identity = parseRunDeliveryIdentity(this.record);
    return identity ? { ...identity } : undefined;
  }

  /** Immutable admission policy captured when a workflow run is created. */
  get maxAgentsPerWorkflow(): number | undefined {
    return this.record.workflowPolicy?.maxAgentsPerWorkflow;
  }

  /** Number of children admitted over the run's full persisted lifetime. */
  get childCount(): number {
    return this.record.children.length;
  }

  /** Fresh post-lock state used to prepare a workflow resume without rereading canonical files. */
  get resumeSnapshot(): RunSnapshot | undefined {
    return this.openedSnapshot;
  }

  addChild(id: string, spec: SubagentSpec, followUpOf?: FollowUpReference): void {
    if (!isSafeChildId(id)) throw new TypeError("Child id must be a non-empty safe object key");
    if (this.record.children.some((child) => child.id === id)) throw new TypeError(`Child id ${JSON.stringify(id)} already exists in this run`);
    // spec.phase rides inside the spec; the navigator reads it from there
    // (with a child.phase fallback kept for runs written before this change).
    this.writeOwned(() => {
      this.record.children.push({ id, spec, ...(followUpOf ? { followUpOf } : {}) });
      this.status.children[id] = { status: "pending", usage: EMPTY_USAGE() };
      if (this.record.kind === "subagent") this.status.status = "pending";
      this.writeJson("run.json", this.record);
      this.writeJson("status.json", this.status);
      this.appendLifecycle("child_added", { id });
    }, { ownershipRequired: true });
  }

  /**
   * Publish one prepared execution generation behind a durable pending marker.
   * Callers must finish journal, usage, and persisted-input reads before
   * entering this commit.
   */
  startWorkflowGeneration(
    script: string,
    phases: readonly Readonly<WorkflowPhase>[] | undefined,
    inputs: { args?: { value: unknown }; rerunChildIds?: readonly string[] } = {},
    options: { requireExistingScript?: boolean } = {},
  ): void {
    this.writeOwned(() => {
      if (this.record.kind !== "workflow") throw new Error(`Run ${this.runId} is not a workflow`);

      // Read and serialize every source before replacing any canonical file.
      const scriptPath = join(this.runDir, "script.js");
      const originalScript = this.readOptionalText(scriptPath);
      if (options.requireExistingScript && originalScript === undefined) {
        throw new Error(`Cannot resume: workflow run directory does not contain script.js: ${this.runDir}`);
      }
      const runPath = join(this.runDir, "run.json");
      const statusPath = join(this.runDir, "status.json");
      const eventsPath = join(this.runDir, "events.jsonl");
      let originalRun: string;
      let originalEvents: string;
      const persistedEvents = this.openedSnapshot?.events ?? [];
      if (this.openedSnapshot) {
        originalRun = this.openedSnapshot.rawRecord ?? readFileSync(runPath, "utf8");
        originalEvents = this.openedSnapshot.rawEvents ?? readFileSync(eventsPath, "utf8");
        this.openedSnapshot = undefined;
      } else {
        originalRun = readFileSync(runPath, "utf8");
        originalEvents = readFileSync(eventsPath, "utf8");
      }

      const priorIdentity = parseRunDeliveryIdentity(this.record);
      const nextGeneration = (priorIdentity?.generation ?? 0) + 1;
      const nextRecord: RunRecord = {
        ...this.record,
        v: 3,
        delivery: {
          protocol: DELIVERY_PROTOCOL_VERSION,
          generation: nextGeneration,
        },
      };
      const reconciledPhases = reconcileResumePhases(phases, this.record.phases, this.record.children);
      if (reconciledPhases === undefined) delete nextRecord.phases;
      else nextRecord.phases = reconciledPhases;
      // A resumed process cannot own children that belonged to the previous
      // process. Preserve their usage and history, but terminalize inherited
      // live states in the same generation commit that publishes "running".
      // Clone every child so a failed staging operation cannot mutate the
      // pre-commit in-memory status through a shared nested object.
      const inheritedLiveChildren: string[] = [];
      const nextChildren: RunStatus["children"] = Object.fromEntries(Object.entries(this.status.children).map(([id, child]) => {
        return [id, {
          ...child,
          usage: { ...child.usage },
        }];
      }));
      // Events can be newer than status.json, but the append itself is not a
      // durability barrier. They may promote a live child to terminal and add
      // cumulative usage, never regress already-terminal durable state.
      reconcilePersistedChildState(nextChildren, persistedEvents);
      // addChild() publishes run.json before status.json. A process crash in
      // that window leaves a durable child with no status entry. Treat that
      // record-only child as an inherited aborted attempt instead of letting
      // readers default it to pending forever.
      for (const child of this.record.children) {
        if (Object.hasOwn(nextChildren, child.id)) continue;
        inheritedLiveChildren.push(child.id);
        nextChildren[child.id] = { status: "aborted", usage: EMPTY_USAGE() };
      }
      for (const [id, child] of Object.entries(nextChildren)) {
        if (child.status !== "pending" && child.status !== "running") continue;
        inheritedLiveChildren.push(id);
        child.status = "aborted";
      }
      const nextStatus: RunStatus = { status: "running", children: nextChildren };
      const startedAt = this.now().toISOString();
      // Rerun authorizations are recorded on the generation they applied to,
      // so a post-hoc reader can tell an authorized re-execution from replay.
      const reconciliationEvents = inheritedLiveChildren.map((id) => JSON.stringify({
        timestamp: startedAt,
        type: "status",
        id,
        status: "aborted",
        reason: "superseded by workflow resume",
      })).join("\n");
      const startedEvent = `${reconciliationEvents ? `${reconciliationEvents}\n` : ""}${JSON.stringify({
        timestamp: startedAt,
        type: "workflow_started",
        generation: nextGeneration,
        ...(inputs.rerunChildIds?.length ? { rerunChildIds: [...inputs.rerunChildIds] } : {}),
      })}\n`;

      type FileChange = { path: string; content: string };
      const changes: FileChange[] = [
        // Commit running first so a crash cannot expose a new script as a
        // completed generation.
        { path: statusPath, content: this.jsonText(nextStatus) },
      ];
      if (originalScript !== script) changes.push({ path: scriptPath, content: script });
      const nextRun = this.jsonText(nextRecord);
      if (nextRun !== originalRun) changes.push({ path: runPath, content: nextRun });
      if (inputs.args) {
        const path = join(this.runDir, "args.json");
        changes.push({ path, content: this.jsonText(inputs.args.value) });
      }
      changes.push({ path: eventsPath, content: `${originalEvents}${jsonlSeparator(originalEvents)}${startedEvent}` });
      originalEvents = "";

      let archivePath: string | undefined;
      if (originalScript !== undefined && originalScript !== script) {
        let generation = 1;
        while (existsSync(join(this.runDir, `script.resumed-${generation}.js`))) generation += 1;
        archivePath = join(this.runDir, `script.resumed-${generation}.js`);
      }

      type StagedChange = FileChange & { temporary?: string };
      const staged: StagedChange[] = changes.map((change) => ({ ...change }));
      let archiveTemporary: string | undefined;

      if (this.record.v !== 3) {
        for (const child of this.record.children) writeSessionClosedMarker(this.runDir, child.id);
      }

      // Once this intent is visible, every failed stage or rename quarantines
      // the run instead of attempting a fallible multi-file rollback.
      const markerPath = join(this.runDir, GENERATION_PENDING_FILE);
      const markerTemporary = stageAtomicFile(markerPath, this.jsonText({ v: 1, startedAt, reason: "generation-commit" }), {
        mode: 0o600,
        fsync: true,
      });
      commitAtomicFile(markerTemporary, markerPath);
      this.syncRunDirectory();
      try {
        for (const change of staged) {
          change.temporary = this.stageFile(change.path, change.content, { durable: true });
        }
        if (archivePath && originalScript !== undefined) {
          archiveTemporary = this.stageFile(archivePath, originalScript, { durable: true });
          this.replaceStagedFile(archiveTemporary, archivePath);
          archiveTemporary = undefined;
        }
        for (const change of staged) {
          this.replaceStagedFile(change.temporary!, change.path);
          change.temporary = undefined;
        }
        // The prior generation's delivery marker must disappear in the same
        // publication transaction. While generation.pending remains durable,
        // readers cannot mistake the new running state for delivered work.
        rmSync(join(this.runDir, DELIVERED_FILE), { force: true });
        // First make every canonical rename and marker removal durable while
        // the quarantine marker is still durable. Only then publish the
        // generation by removing the marker and syncing that removal.
        this.syncRunDirectory();
        rmSync(markerPath);
        this.syncRunDirectory();
        this.record = nextRecord;
        this.status = nextStatus;
      } finally {
        for (const change of staged) {
          if (change.temporary) discardAtomicFile(change.temporary);
        }
        if (archiveTemporary) discardAtomicFile(archiveTemporary);
      }
    }, { ownershipRequired: true });
  }

  appendJournal(value: unknown): void {
    this.writeOwned(() => {
      this.appendDurableJsonl(join(this.runDir, "journal.jsonl"), value);
      // Deterministic crash boundary for durability and resume testing: the
      // entry is fsynced but nothing after it has happened yet.
      if (process.env.PI_SUBAGENT_WORKFLOW_CRASH_AFTER_JOURNAL_APPEND === "1") {
        process.kill(process.pid, "SIGKILL");
      }
    }, { ownershipRequired: true });
  }

  /**
   * Atomically replace the journal after causal-tail invalidation. A crash
   * during resume must never splice this generation with stale dependent calls.
   */
  rewriteJournal(entries: unknown[]): void {
    this.writeOwned(() => {
      const path = join(this.runDir, "journal.jsonl");
      replaceAtomicFile(path, entries.map((entry) => `${JSON.stringify(entry)}\n`).join(""), {
        mode: 0o600,
        fsync: true,
        syncParentDirectory: true,
      });
    }, { ownershipRequired: true });
  }

  recordLog(message: string): void {
    this.writeOwned(() => this.appendLifecycle("log", { message }));
  }

  recordPhase(title: string): void {
    this.writeOwned(() => {
      if (!this.record.phases?.some((phase) => phase.title === title)) {
        this.record.phases ??= [];
        this.record.phases.push({ title });
        this.writeJson("run.json", this.record);
      }
      // The phase list is static metadata; this event is the transition. Keep
      // it even for declared phases so narrator logs can be assigned correctly.
      this.appendLifecycle("phase", { title });
    });
  }

  /**
   * Publish the workflow's normalized return value before the run is marked
   * completed. Publishing undefined removes an artifact from an earlier
   * execution generation. Both operations require current run ownership so a
   * stale process cannot create or remove the winning generation's result.
   */
  writeWorkflowResult(result: unknown): void {
    this.writeOwned(() => {
      if (result === undefined) rmSync(join(this.runDir, "result.json"), { force: true });
      else this.writeJson("result.json", result, { durable: true });
    }, { ownershipRequired: true });
  }

  workflowFinished(status: "completed" | "failed" | "aborted", error?: string): void {
    this.writeOwned(() => {
      this.status.status = status;
      this.writeJson("status.json", this.status, { durable: true });
      this.appendLifecycle(`workflow_${status}`, error ? { error } : {});
    });
    this.releaseOwnershipHandle();
  }

  /**
   * Reinstate a prior terminal status after a resume was refused before it
   * mutated any replay state, so a declined replay authorization leaves the
   * run exactly as terminal as it was. The refusal itself stays auditable in
   * the event log.
   */
  restoreTerminalStatus(status: "completed" | "failed" | "aborted", reason: string): void {
    this.writeOwned(() => {
      this.status.status = status;
      this.writeJson("status.json", this.status, { durable: true });
      this.appendLifecycle("workflow_resume_refused", { error: reason });
    });
    this.releaseOwnershipHandle();
  }

  /** Give up ownership after resume setup fails, without changing prior run status. */
  releaseOwnership(): void {
    this.releaseOwnershipHandle();
  }

  resolveChild(id: string, resolved: ResolvedSpec, sessionFile?: string): void {
    this.writeOwned(() => {
      const child = this.record.children.find((item) => item.id === id);
      if (!child) return;
      child.resolved = resolved;
      child.sessionFile = sessionFile;
      this.writeJson("run.json", this.record);
    });
  }

  recordEvent(event: SubagentEvent): void {
    if (this.writesClosed) return;
    const runWasTerminal = this.record.kind === "subagent"
      && this.status.status !== "running"
      && this.status.status !== "pending";
    // Result already carries final usage. Ignore delayed activity/usage/status
    // events after completion. A terminal subagent run never reopens.
    if (runWasTerminal) return;
    const eventStatus = event.type === "result" ? event.result.status : event.type === "status" ? event.status : undefined;
    const projectedStatuses = Object.entries(this.status.children).map(([id, child]) => id === event.id && eventStatus ? eventStatus : child.status);
    let releaseAfterWrite = false;
    if (this.record.kind === "subagent" && eventStatus !== undefined && eventStatus !== "running" && eventStatus !== "pending") {
      const projectedStatus = deriveRunStatus(projectedStatuses);
      releaseAfterWrite = projectedStatus !== "running" && projectedStatus !== "pending";
    }
    this.writeOwned(() => {
      this.applyEventState(event);
      this.appendLine({ timestamp: this.now().toISOString(), ...event });
      this.writeJson("status.json", this.status, { durable: releaseAfterWrite });
    });
    // A failed terminal status write is degraded but must not leave ownership
    // claiming the run is live forever. Without an owner, readers reconcile the
    // last durable event/status as dead-owner state.
    if (releaseAfterWrite) {
      this.releaseOwnershipHandle();
    }
  }

  disposed(): void {
    this.writeOwned(() => this.appendLifecycle("disposed"));
  }

  /** Acquire ownership once and hold it until this store reaches a release point. */
  private ensureOwnership(): void {
    if (this.writesClosed) throw new RunStoreOwnershipError(`Run ${this.runId} is closed and can no longer be written`);
    if (this.runOwnership) return;
    if (this.ownershipAttempted) throw new RunStoreOwnershipError(`Run ${this.runId} is closed and can no longer be written`);
    this.ownershipAttempted = true;
    try {
      this.runOwnership = acquireRunOwnership(this.runDir);
    } catch (error) {
      if (error instanceof RunOwnershipConflictError) {
        const detail = error.owner ? ` (pid ${error.owner.pid} on ${error.owner.host})` : "";
        throw new RunStoreOwnershipError(`Run ${this.runId} is active in another process${detail}; wait for it to finish or stop it there first`);
      }
      throw error;
    }
  }

  private releaseOwnershipHandle(): void {
    const ownership = this.runOwnership;
    this.runOwnership = undefined;
    this.writesClosed = true;
    if (ownership) {
      try {
        ownership.release();
      } catch (error) {
        this.degrade(error);
      }
    }
  }

  private applyEventState(event: SubagentEvent): void {
    const child = this.status.children[event.id];
    if (child && event.type === "status") child.status = event.status;
    if (child && event.type === "usage") child.usage = event.usage;
    if (child && event.type === "result") {
      child.status = event.result.status;
      child.usage = event.result.usage;
    }
    if (this.record.kind === "workflow") {
      this.status.status = "running";
    } else {
      this.status.status = deriveRunStatus(Object.values(this.status.children).map((item) => item.status));
    }
  }

  /**
   * Persist only while this store owns the run. Critical workflow mutations
   * throw on failure; ordinary child event writes fail closed and degrade.
   */
  private writeOwned(
    write: () => void,
    options: { ownershipRequired?: boolean } = {},
  ): boolean {
    if (this.writesClosed) {
      if (options.ownershipRequired) throw new RunStoreOwnershipError(`Run ${this.runId} is closed and can no longer be written`);
      return false;
    }
    if (!this.runOwnership) {
      if (options.ownershipRequired) throw new RunStoreOwnershipError(`Run ${this.runId} is closed and can no longer be written`);
      return false;
    }
    try {
      write();
      return true;
    } catch (error) {
      this.degrade(error);
      if (options.ownershipRequired) throw error;
      return false;
    }
  }

  private degrade(error: unknown): void {
    const message = errorMessage(error);
    this.degradedReason ??= message;
    reportDiagnostic(`[subagent-workflow] run persistence failed: ${message}`);
  }

  private appendLifecycle(type: string, data: Record<string, unknown> = {}): { event: FrozenJson; line: string } {
    const event = Object.freeze({ timestamp: this.now().toISOString(), type, ...data }) as FrozenJson;
    return { event, line: this.appendLine(event) };
  }

  private appendLine(value: unknown): string {
    const line = `${JSON.stringify(value)}\n`;
    const path = join(this.runDir, "events.jsonl");
    const appended = `${jsonlFileSeparator(path)}${line}`;
    appendFileSync(path, appended, { mode: 0o600 });
    return appended;
  }

  private appendDurableJsonl(path: string, value: unknown): void {
    const created = !existsSync(path);
    const appended = `${jsonlFileSeparator(path)}${JSON.stringify(value)}\n`;
    const buffer = Buffer.from(appended, "utf8");
    let descriptor: number | undefined;
    let failure: { error: unknown } | undefined;
    try {
      descriptor = openSync(path, "a", 0o600);
      let offset = 0;
      while (offset < buffer.length) {
        const remaining = buffer.length - offset;
        const written = writeSync(descriptor, buffer, offset, remaining);
        if (!Number.isSafeInteger(written) || written <= 0 || written > remaining) {
          throw new Error(`Unable to complete durable JSONL append for ${path}`);
        }
        offset += written;
      }
      fsyncSync(descriptor);
    } catch (error) {
      failure = { error };
    }
    if (descriptor !== undefined) {
      try {
        closeSync(descriptor);
      } catch (error) {
        failure ??= { error };
      }
    }
    if (failure) throw failure.error;
    if (created) this.syncRunDirectory();
  }

  private writeJson(name: string, value: unknown, options: { durable?: boolean } = {}): void {
    const path = join(this.runDir, name);
    const content = this.jsonText(value);
    if (options.durable) {
      replaceAtomicFile(path, content, {
        mode: 0o600,
        fsync: true,
        syncParentDirectory: true,
      });
      return;
    }
    const temporary = this.stageFile(path, content);
    this.replaceStagedFile(temporary, path);
  }

  private jsonText(value: unknown): string {
    const json = JSON.stringify(value, null, 2);
    if (json === undefined) throw new TypeError("Run metadata must be JSON-serializable");
    return `${json}\n`;
  }

  private stageFile(path: string, content: string, options: { durable?: boolean } = {}): string {
    if (!options.durable) return stageFileExclusive(path, content);
    return stageAtomicFile(path, content, { mode: 0o600, fsync: true });
  }

  private replaceStagedFile(temporary: string, path: string): void {
    commitAtomicFile(temporary, path);
  }

  /** Directory barrier for durable marker creation, publication, and removal. */
  private syncRunDirectory(): void {
    syncDirectoryDurably(this.runDir);
  }

  private readOptionalText(path: string): string | undefined {
    try {
      return readFileSync(path, "utf8");
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined;
      throw error;
    }
  }

  private requiredSnapshotValue<T>(snapshot: RunSnapshot, name: string, value: FrozenJson | undefined): T {
    if (value !== undefined) return value as unknown as T;
    const reason = snapshot.diagnostics.find((diagnostic) => diagnostic.file === name && diagnostic.line === undefined)?.problem
      ?? `ENOENT: no such file or directory, open '${join(this.runDir, name)}'`;
    throw new Error(`Cannot resume run ${this.runId}: invalid ${name}: ${reason}`);
  }

  private validateWorkflowPolicy(record: RunRecord): void {
    const policy = record.workflowPolicy as unknown;
    if (policy === undefined) return;
    if (!isRecord(policy) || !("maxAgentsPerWorkflow" in policy)) {
      throw new Error(`Cannot resume run ${this.runId}: invalid run.json workflowPolicy; expected maxAgentsPerWorkflow`);
    }
    const value = policy.maxAgentsPerWorkflow;
    if (typeof value !== "number" || !Number.isInteger(value) || value < 1 || value > 1_000) {
      throw new Error(`Cannot resume run ${this.runId}: invalid run.json workflowPolicy.maxAgentsPerWorkflow; expected an integer from 1 to 1000`);
    }
  }

  private validateResumeRecord(record: RunRecord): void {
    if (!isRecord(record)
      || (record.v !== undefined && record.v !== 2 && record.v !== 3)
      || record.runId !== this.runId
      || !Array.isArray(record.children)
      || !record.children.every((child) => isRecord(child)
        && isSafeChildId(child.id) && isRecord(child.spec))) {
      throw new TypeError(`Cannot resume run ${this.runId}: invalid run.json: expected matching runId and children array of {id, spec}`);
    }
    if (record.v === 3 && !parseRunDeliveryIdentity(record)) {
      throw new TypeError(`Cannot resume run ${this.runId}: invalid run.json delivery protocol identity`);
    }
    if (new Set(record.children.map((child) => child.id)).size !== record.children.length) {
      throw new TypeError(`Cannot resume run ${this.runId}: invalid run.json: child IDs must be unique`);
    }
    this.validateWorkflowPolicy(record);
  }

  private validateResumeStatus(status: RunStatus): void {
    if (!isRecord(status)
      || !isSubagentStatus(status.status)
      || !isRecord(status.children)
      || !Object.values(status.children).every((child) => isRecord(child)
        && isSubagentStatus(child.status)
        && isUsageSummary(child.usage))) {
      throw new TypeError(`Cannot resume run ${this.runId}: invalid status.json: expected valid run and child status with cumulative usage`);
    }
  }

  private validateResumeSnapshot(snapshot: RunSnapshot): { record: RunRecord; status: RunStatus } {
    const record = this.requiredSnapshotValue<RunRecord>(snapshot, "run.json", snapshot.record);
    const status = this.requiredSnapshotValue<RunStatus>(snapshot, "status.json", snapshot.status);
    this.validateResumeRecord(record);
    this.validateResumeStatus(status);
    const recordIds = new Set(record.children.map((child) => child.id));
    const unrecordedStatusId = Object.keys(status.children).find((id) => !recordIds.has(id));
    if (unrecordedStatusId !== undefined) {
      throw new Error(`Cannot resume run ${this.runId}: status.json references child ${JSON.stringify(unrecordedStatusId)} missing from run.json`);
    }
    const diagnostic = snapshot.diagnostics.find(({ file }) => file === "run.json" || file === "status.json" || file === "events.jsonl");
    if (diagnostic) {
      const line = diagnostic.line === undefined ? "" : ` line ${diagnostic.line}`;
      throw new Error(`Cannot resume run ${this.runId}: invalid ${diagnostic.file}${line}: ${diagnostic.problem}`);
    }
    validatePersistedEvents(this.runId, snapshot.events);
    return { record, status };
  }

}

function jsonlSeparator(contents: string): string {
  return contents.length > 0 && !contents.endsWith("\n") ? "\n" : "";
}

function jsonlFileSeparator(path: string): string {
  if (!existsSync(path)) return "";
  const size = statSync(path).size;
  if (size === 0) return "";
  let descriptor: number | undefined;
  try {
    descriptor = openSync(path, "r");
    const last = Buffer.allocUnsafe(1);
    readSync(descriptor, last, 0, 1, size - 1);
    return last[0] === 0x0a ? "" : "\n";
  } catch {
    // A write-only JSONL file cannot be inspected. A conservative blank line
    // still guarantees the new record cannot fuse with existing bytes.
    return "\n";
  } finally {
    if (descriptor !== undefined) closeSync(descriptor);
  }
}

function withAppendedEvent(
  snapshot: RunSnapshot,
  appended: { event: FrozenJson; line: string },
): RunSnapshot {
  return Object.freeze({
    ...snapshot,
    events: snapshot.rawEvents === undefined ? snapshot.events : Object.freeze([...snapshot.events, appended.event]),
    rawEvents: snapshot.rawEvents === undefined ? undefined : `${snapshot.rawEvents}${appended.line}`,
  });
}

function cloneFrozenJson(value: FrozenJson): FrozenJson {
  if (value === null || typeof value !== "object") return value;
  const root: object = Array.isArray(value) ? [] : {};
  const pending: Array<{ source: readonly FrozenJson[] | { readonly [key: string]: FrozenJson }; target: object }> = [
    { source: value, target: root },
  ];
  while (pending.length > 0) {
    const { source, target } = pending.pop()!;
    for (const [key, item] of Object.entries(source)) {
      const cloned: FrozenJson | object = item !== null && typeof item === "object"
        ? (Array.isArray(item) ? [] : {})
        : item;
      Object.defineProperty(target, key, { value: cloned, enumerable: true, writable: true, configurable: true });
      if (item !== null && typeof item === "object") pending.push({ source: item, target: cloned as object });
    }
  }
  return root as FrozenJson;
}

function reconcilePersistedChildState(
  children: RunStatus["children"],
  events: readonly FrozenJson[],
): void {
  const promotable = new Set(Object.entries(children)
    .filter(([, child]) => child.status === "pending" || child.status === "running")
    .map(([id]) => id));
  for (const value of events) {
    if (!isRecord(value) || typeof value.id !== "string") continue;
    const child = children[value.id];
    if (!child) continue;
    const result = value.type === "result" && isRecord(value.result) ? value.result : undefined;
    const eventStatus = value.type === "status" ? value.status : result?.status;
    if (promotable.has(value.id) && isTerminalStatus(eventStatus)) child.status = eventStatus;
    const usage = value.type === "usage" ? value.usage : result?.usage;
    if (isUsageSummary(usage)) child.usage = maxUsage(child.usage, usage);
  }
}

function isTerminalStatus(value: unknown): value is Extract<SubagentStatus, "completed" | "failed" | "aborted"> {
  return value === "completed" || value === "failed" || value === "aborted";
}

function isSubagentStatus(value: unknown): value is SubagentStatus {
  return value === "pending" || value === "running" || isTerminalStatus(value);
}

function isSafeChildId(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && !Object.hasOwn(Object.prototype, value);
}

function isUsageSummary(value: unknown): value is UsageSummary {
  return isRecord(value)
    && [value.input, value.output, value.cacheRead, value.cacheWrite, value.cost, value.turns]
      .every((item) => typeof item === "number" && Number.isFinite(item) && item >= 0);
}

function maxUsage(left: UsageSummary, right: UsageSummary): UsageSummary {
  return {
    input: Math.max(left.input, right.input),
    output: Math.max(left.output, right.output),
    cacheRead: Math.max(left.cacheRead, right.cacheRead),
    cacheWrite: Math.max(left.cacheWrite, right.cacheWrite),
    cost: Math.max(left.cost, right.cost),
    turns: Math.max(left.turns, right.turns),
  };
}

function validatePersistedEvents(runId: string, events: readonly FrozenJson[]): void {
  const terminalByChild = new Map<string, Extract<SubagentStatus, "completed" | "failed" | "aborted">>();
  for (const [index, event] of events.entries()) {
    if (!isValidPersistedEvent(event)) {
      throw new TypeError(`Cannot resume run ${runId}: invalid events.jsonl event ${index + 1}`);
    }
    if (!isRecord(event) || typeof event.id !== "string") continue;
    const result = event.type === "result" && isRecord(event.result) ? event.result : undefined;
    const status = event.type === "status" ? event.status : result?.status;
    if (!isSubagentStatus(status)) continue;
    const terminal = terminalByChild.get(event.id);
    if (terminal !== undefined && status !== terminal) {
      throw new TypeError(`Cannot resume run ${runId}: contradictory lifecycle for child ${JSON.stringify(event.id)} in events.jsonl event ${index + 1}`);
    }
    if (isTerminalStatus(status)) terminalByChild.set(event.id, status);
  }
}

function isValidPersistedEvent(event: FrozenJson): boolean {
  if (!isRecord(event) || typeof event.type !== "string" || !isCanonicalTimestamp(event.timestamp)) return false;
  if (event.type === "status") return isSafeChildId(event.id) && isSubagentStatus(event.status);
  if (event.type === "activity") return isSafeChildId(event.id) && typeof event.description === "string";
  if (event.type === "usage") return isSafeChildId(event.id) && isUsageSummary(event.usage);
  if (event.type !== "result") return true;
  return isSafeChildId(event.id)
    && isRecord(event.result)
    && event.result.id === event.id
    && isTerminalStatus(event.result.status)
    && isUsageSummary(event.result.usage);
}

function isCanonicalTimestamp(value: unknown): value is string {
  if (typeof value !== "string") return false;
  const epoch = Date.parse(value);
  return Number.isFinite(epoch) && new Date(epoch).toISOString() === value;
}

export function deriveRunStatus(statuses: SubagentStatus[]): SubagentStatus {
  if (statuses.some((status) => status === "running")) return "running";
  if (statuses.some((status) => status === "pending")) return "pending";
  if (statuses.some((status) => status === "failed")) return "failed";
  if (statuses.some((status) => status === "aborted")) return "aborted";
  return "completed";
}

/**
 * An edited script owns the new phase skeleton. Historical phase metadata is
 * retained only where an already-persisted child still needs that grouping.
 */
function reconcileResumePhases(
  declared: readonly Readonly<WorkflowPhase>[] | undefined,
  previous: WorkflowPhase[] | undefined,
  children: ChildRecord[],
): WorkflowPhase[] | undefined {
  const phases = (declared ?? []).map((phase) => ({ ...phase }));
  const seen = new Set(phases.map((phase) => phase.title));
  const referenced = new Set(
    children
      .map((child) => child.phase ?? child.spec.phase)
      .filter((phase): phase is string => typeof phase === "string"),
  );

  for (const phase of previous ?? []) {
    if (!referenced.has(phase.title) || seen.has(phase.title)) continue;
    phases.push({ ...phase });
    seen.add(phase.title);
  }
  for (const title of referenced) {
    if (seen.has(title)) continue;
    phases.push({ title });
    seen.add(title);
  }

  return phases.length > 0 || declared !== undefined ? phases : undefined;
}

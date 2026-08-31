import { statSync } from "node:fs";
import { deriveRunStatus } from "./run-store.js";
import { activityFoldFromSnapshot, cloneActivityFold, foldActivity, type RunActivityFold } from "./activity-fold.js";
import type { FrozenJson, RunSnapshot } from "./run-snapshot.js";
import type { FollowUpReference, ResolvedSpec, SubagentEvent, SubagentSpec, SubagentStatus, UsageSummary } from "../types.js";
import { childLabel, countStatuses, firstLine, shortModel } from "../ui/format.js";
import { sanitizeTerminalText } from "../ui/sanitize.js";

export type RunKind = "subagent" | "workflow";

/** One row in the level-1 run list. */
export interface RunSummary {
  runId: string;
  runDir: string;
  kind: RunKind;
  /** Epoch ms; used for newest-first ordering. */
  createdAt: number;
  label: string;
  status: SubagentStatus;
  done: number;
  total: number;
  completed: number;
  failed: number;
  aborted: number;
  tokens: number;
  /** True when the run directory could not be parsed; renders as a dim placeholder. */
  corrupt: boolean;
  /** True when disk said live but no live owner exists; statuses were reconciled (dead-parent recovery). */
  reconciled: boolean;
}

/** One child agent inside a run detail. */
export interface ChildRow {
  id: string;
  label: string;
  model: string;
  /** Reasoning effort the child resolved to, shown beside the model. */
  thinking?: string;
  phase?: string;
  status: SubagentStatus;
  tokens: number;
  startedAt?: number;
  endedAt?: number;
  activity?: string;
  resultLine?: string;
  error?: string;
  sessionFile?: string;
  followUpOf?: FollowUpReference;
  spec: SubagentSpec;
}

/** A narrator log or phase-change line from the workflow run. */
export interface NarratorLine {
  timestamp: number;
  kind: "log" | "phase";
  text: string;
}

/** Full level-2 run detail. */
export interface RunDetail {
  runId: string;
  runDir: string;
  kind: RunKind;
  label: string;
  status: SubagentStatus;
  phases: { title: string }[];
  children: ChildRow[];
  narrator: NarratorLine[];
  hasScript: boolean;
  corrupt: boolean;
}

export interface RunRecordFile {
  runId?: string;
  kind?: string;
  createdAt?: string;
  children?: Array<{ id?: string; spec?: SubagentSpec; resolved?: ResolvedSpec; sessionFile?: string; followUpOf?: FollowUpReference; phase?: string }>;
  phases?: Array<{ title?: string }>;
}

export interface RunStatusFile {
  status?: SubagentStatus;
  children?: Record<string, { status?: SubagentStatus; usage?: UsageSummary }>;
}

/** Full mutable fold retained by the owning runner. Public views are cloned before rendering. */
export interface RunProjection {
  summary: RunSummary;
  detail: RunDetail;
  activity: RunActivityFold;
  priorGenerationNarration: NarratorLine[];
  terminalStatuses: Map<string, SubagentStatus>;
}

export interface SnapshotProjectionOptions {
  describeWorkflow?: (script: string) => string;
  workflowLabel?: string;
  ownerWasDead?: boolean;
}

export interface RunSnapshotSummaryProjection {
  liveSummary: RunSummary;
  reconciledSummary: RunSummary;
}

export type RunProjectionEvent =
  | { type: "child"; id: string; spec: SubagentSpec; followUpOf?: FollowUpReference; timestamp?: number }
  | { type: "resolved"; id: string; resolved: ResolvedSpec; sessionFile?: string; timestamp?: number }
  | { type: "workflow_started"; timestamp?: number }
  | { type: "log"; message: string; timestamp?: number }
  | { type: "phase"; title: string; timestamp?: number }
  | { type: "workflow_resume_refused"; error: string; timestamp?: number }
  | { type: "workflow_status"; status: SubagentStatus; timestamp?: number }
  | ({ timestamp?: number } & SubagentEvent);

const QUARANTINED_LABEL = "quarantined - crashed mid-resume";

export function corruptRunSummary(runDir: string, runId: string, label = "unreadable run"): RunSummary {
  return {
    runId,
    runDir,
    kind: "subagent",
    createdAt: mtimeMs(runDir),
    label,
    status: "failed",
    done: 0,
    total: 0,
    completed: 0,
    failed: 0,
    aborted: 0,
    tokens: 0,
    corrupt: true,
    reconciled: false,
  };
}

export function corruptRunDetail(runId: string, runDir: string, label = "unreadable run"): RunDetail {
  return { runId, runDir, kind: "subagent", label, status: "failed", phases: [], children: [], narrator: [], hasScript: false, corrupt: true };
}

export function snapshotSaysLive(snapshot: RunSnapshot): boolean {
  if (snapshot.generationPending) return false;
  const run = snapshot.record as RunRecordFile | undefined;
  if (!run || !Array.isArray(run.children)) return false;
  const ids = run.children.map((child) => child.id ?? "");
  return isLiveStatus(runStatusFor(snapshot.status as RunStatusFile | undefined, ids));
}

/** Build only the level-1 data needed by the run list. */
export function projectRunSnapshotSummary(
  snapshot: RunSnapshot,
  runId: string,
  options: SnapshotProjectionOptions = {},
): RunSnapshotSummaryProjection {
  const eventState = scanSummaryEvents(snapshot.events);
  const built = buildSnapshotSummary(snapshot, runId, options, eventState);
  return {
    liveSummary: built.summary,
    reconciledSummary: reconcileDeadOwnerSummary(built.summary, built.children),
  };
}

/** Tolerantly fold one immutable persisted snapshot into the navigator's full projection. */
export function projectRunSnapshot(snapshot: RunSnapshot, runId: string, options: SnapshotProjectionOptions = {}): RunProjection {
  const { runDir } = snapshot;
  if (snapshot.generationPending) return corruptProjection(runDir, runId, QUARANTINED_LABEL);
  const run = snapshot.record as RunRecordFile | undefined;
  if (!run || !Array.isArray(run.children)) return corruptProjection(runDir, runId);

  const eventState = scanEvents(snapshot.events);
  const built = buildSnapshotSummary(snapshot, runId, options, eventState.byChild);
  const children: ChildRow[] = run.children.map((child, index) => {
    const state = built.children[index]!;
    const events = eventState.byChild.get(state.id) ?? {};
    return {
      id: state.id,
      label: persistedChildLabel(child, state.id || "agent"),
      model: shortModel(optionalString(child.resolved?.modelId)),
      thinking: optionalString(child.resolved?.thinkingLevel),
      phase: optionalString(child.phase) ?? optionalString(child.spec?.phase),
      status: state.status,
      tokens: state.tokens,
      startedAt: events.startedAt,
      endedAt: events.endedAt,
      activity: events.activity,
      resultLine: events.resultLine,
      error: events.error,
      sessionFile: optionalString(child.sessionFile),
      followUpOf: child.followUpOf,
      spec: child.spec ?? { prompt: "" },
    };
  });

  const projection: RunProjection = {
    summary: built.summary,
    detail: {
      runId,
      runDir,
      kind: built.summary.kind,
      label: built.summary.label,
      status: built.summary.status,
      phases: Array.isArray(run.phases)
        ? run.phases.flatMap((phase) => {
          const title = optionalString(phase?.title);
          return title ? [{ title }] : [];
        })
        : [],
      children,
      narrator: eventState.narrator,
      hasScript: snapshot.scriptPresent,
      corrupt: false,
    },
    activity: activityFoldFromSnapshot(snapshot),
    priorGenerationNarration: eventState.priorGenerationNarration,
    terminalStatuses: new Map(built.children
      .filter((child): child is SummaryChildState & { terminalStatus: SubagentStatus } => child.terminalStatus !== undefined)
      .map((child) => [child.id, child.terminalStatus])),
  };
  return options.ownerWasDead ? reconcileDeadOwnerProjection(projection) : projection;
}

interface SummaryChildState {
  id: string;
  status: SubagentStatus;
  tokens: number;
  terminalStatus?: SubagentStatus;
}

interface SnapshotSummaryBuild {
  summary: RunSummary;
  children: SummaryChildState[];
}

function buildSnapshotSummary(
  snapshot: RunSnapshot,
  runId: string,
  options: SnapshotProjectionOptions,
  eventState: ReadonlyMap<string, SummaryChildEventState>,
): SnapshotSummaryBuild {
  const { runDir } = snapshot;
  if (snapshot.generationPending) {
    return { summary: corruptRunSummary(runDir, runId, QUARANTINED_LABEL), children: [] };
  }
  const run = snapshot.record as RunRecordFile | undefined;
  if (!run || !Array.isArray(run.children)) {
    return { summary: corruptRunSummary(runDir, runId), children: [] };
  }

  const status = snapshot.status as RunStatusFile | undefined;
  const kind: RunKind = run.kind === "workflow" ? "workflow" : "subagent";
  const ids = run.children.map((child) => child.id ?? "");
  const statuses = childStatuses(status, ids);
  const children = ids.map((id, index) => {
    const events = eventState.get(id);
    return {
      id,
      status: statuses[index] ?? "pending",
      tokens: tokensForChild(status?.children?.[id]?.usage, events),
      terminalStatus: events?.terminalStatus,
    };
  });
  const counts = countStatuses(statuses);
  return {
    summary: {
      runId,
      runDir,
      kind,
      createdAt: parseTimestamp(run.createdAt) || mtimeMs(runDir),
      label: labelForRun(kind, run.children, snapshot, options),
      status: runStatusFor(status, ids),
      done: counts.done,
      total: children.length,
      completed: counts.completed,
      failed: counts.failed,
      aborted: counts.aborted,
      tokens: children.reduce((total, child) => total + child.tokens, 0),
      corrupt: false,
      reconciled: false,
    },
    children,
  };
}

function reconcileDeadOwnerSummary(source: RunSummary, sourceChildren: readonly SummaryChildState[]): RunSummary {
  const summary = structuredClone(source);
  if (summary.corrupt || !isLiveStatus(summary.status)) return summary;
  const children = sourceChildren.map((child) => ({
    ...child,
    status: isLiveStatus(child.status) ? child.terminalStatus ?? "aborted" : child.status,
  }));
  refreshSummary(summary, children);
  if (summary.kind === "workflow" || children.length === 0) summary.status = "aborted";
  summary.reconciled = true;
  return summary;
}

function tokensForChild(usage: UsageSummary | undefined, events: SummaryChildEventState | undefined): number {
  if (events?.terminalTokens !== undefined) return events.terminalTokens;
  if (usage) return usage.input + usage.output;
  return events?.tokens ?? 0;
}

/** Reconcile a disk-live projection after proving that no process owns the run. */
export function reconcileDeadOwnerProjection(source: RunProjection): RunProjection {
  const projection = cloneRunProjection(source);
  if (projection.summary.corrupt || !isLiveStatus(projection.summary.status)) return projection;
  for (const child of projection.detail.children) {
    if (isLiveStatus(child.status)) child.status = projection.terminalStatuses.get(child.id) ?? "aborted";
  }
  refreshProjection(projection);
  if (projection.summary.kind === "workflow" || projection.detail.children.length === 0) {
    projection.summary.status = "aborted";
    projection.detail.status = "aborted";
  }
  projection.summary.reconciled = true;
  return projection;
}

/** Fold one live runner or workflow event synchronously into an owned projection. */
export function foldRunProjection(projection: RunProjection, event: RunProjectionEvent): void {
  if (projection.summary.corrupt) return;
  const timestamp = event.timestamp ?? Date.now();
  if (event.type === "child") {
    if (projection.detail.children.some((child) => child.id === event.id)) return;
    projection.detail.children.push({
      id: event.id,
      label: persistedChildLabel({ id: event.id, spec: event.spec, followUpOf: event.followUpOf }, event.id),
      model: "unknown",
      phase: event.spec.phase,
      status: "pending",
      tokens: 0,
      followUpOf: event.followUpOf,
      spec: event.spec,
    });
    foldActivity(projection.activity, { type: "child", id: event.id, label: event.spec.label ?? event.id });
    refreshProjection(projection);
    return;
  }
  if (event.type === "resolved") {
    const child = projection.detail.children.find((item) => item.id === event.id);
    if (!child) return;
    child.model = shortModel(event.resolved.modelId);
    child.thinking = event.resolved.thinkingLevel;
    child.sessionFile = event.sessionFile;
    foldActivity(projection.activity, { type: "child", id: event.id, label: event.resolved.label });
    return;
  }
  if (event.type === "workflow_started") {
    projection.priorGenerationNarration = projection.detail.narrator.splice(0);
    projection.summary.status = "running";
    projection.detail.status = "running";
    return;
  }
  if (event.type === "log") {
    projection.detail.narrator.push({ timestamp, kind: "log", text: event.message });
    return;
  }
  if (event.type === "phase") {
    if (!projection.detail.phases.some((phase) => phase.title === event.title)) projection.detail.phases.push({ title: event.title });
    projection.detail.narrator.push({ timestamp, kind: "phase", text: event.title });
    return;
  }
  if (event.type === "workflow_resume_refused") {
    projection.detail.narrator.splice(0, projection.detail.narrator.length, ...projection.priorGenerationNarration);
    projection.detail.narrator.push({
      timestamp,
      kind: "log",
      text: sanitizeTerminalText(`resume refused: ${firstLine(event.error)}`),
    });
    return;
  }
  if (event.type === "workflow_status") {
    projection.summary.status = event.status;
    projection.detail.status = event.status;
    return;
  }

  const child = projection.detail.children.find((item) => item.id === event.id);
  if (!child) return;
  if (event.type === "status") {
    child.status = event.status;
    if (event.status === "running" && child.startedAt === undefined) child.startedAt = timestamp;
    if (isTerminalStatus(event.status)) {
      child.endedAt = timestamp;
      projection.terminalStatuses.set(event.id, event.status);
    }
  } else if (event.type === "activity") {
    child.activity = event.description;
    foldActivity(projection.activity, event);
  } else if (event.type === "usage") {
    child.tokens = event.usage.input + event.usage.output;
  } else if (event.type === "result") {
    child.status = event.result.status;
    projection.terminalStatuses.set(event.id, event.result.status);
    child.tokens = event.result.usage.input + event.result.usage.output;
    child.endedAt = timestamp;
    if (event.result.text) child.resultLine = firstLine(event.result.text);
    child.error = event.result.error;
    child.sessionFile = event.result.sessionFile ?? child.sessionFile;
    child.model = shortModel(event.result.resolved.modelId);
    child.thinking = event.result.resolved.thinkingLevel;
  }
  refreshProjection(projection);
}

export function cloneRunProjection(projection: RunProjection): RunProjection {
  return {
    summary: structuredClone(projection.summary),
    detail: structuredClone(projection.detail),
    activity: cloneActivityFold(projection.activity),
    priorGenerationNarration: structuredClone(projection.priorGenerationNarration),
    terminalStatuses: new Map(projection.terminalStatuses),
  };
}

function corruptProjection(runDir: string, runId: string, label?: string): RunProjection {
  return {
    summary: corruptRunSummary(runDir, runId, label),
    detail: corruptRunDetail(runId, runDir, label),
    activity: { children: new Map(), complete: false },
    priorGenerationNarration: [],
    terminalStatuses: new Map(),
  };
}

function refreshProjection(projection: RunProjection): void {
  const { summary, detail } = projection;
  refreshSummary(summary, detail.children);
  if (summary.kind === "subagent") {
    detail.status = summary.status;
    summary.label = detail.children[0]?.label ?? "subagent";
    detail.label = summary.label;
  }
}

function refreshSummary(summary: RunSummary, children: readonly Pick<ChildRow, "status" | "tokens">[]): void {
  const statuses = children.map((child) => child.status);
  if (summary.kind === "subagent") summary.status = statuses.length === 0 ? "pending" : deriveRunStatus(statuses);
  const counts = countStatuses(statuses);
  summary.total = statuses.length;
  summary.done = counts.done;
  summary.completed = counts.completed;
  summary.failed = counts.failed;
  summary.aborted = counts.aborted;
  summary.tokens = children.reduce((total, child) => total + child.tokens, 0);
}

function labelForRun(
  kind: RunKind,
  children: NonNullable<RunRecordFile["children"]>,
  snapshot: RunSnapshot,
  options: SnapshotProjectionOptions,
): string {
  if (kind === "workflow") return options.workflowLabel ?? workflowName(snapshot, options.describeWorkflow) ?? "workflow";
  const child = children[0];
  return child ? persistedChildLabel(child, "subagent") : "subagent";
}

function persistedChildLabel(child: NonNullable<RunRecordFile["children"]>[number], fallback: string): string {
  const spec = child.spec as Record<string, unknown> | undefined;
  const explicit = optionalString(spec?.label)?.trim();
  const prompt = optionalString(spec?.prompt);
  const label = explicit || (prompt === undefined ? fallback : childLabel({ prompt }));
  return child.followUpOf ? `${label} (follow-up)` : label;
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function workflowName(snapshot: RunSnapshot, describeWorkflow: SnapshotProjectionOptions["describeWorkflow"]): string | undefined {
  if (!describeWorkflow) return undefined;
  try {
    if (!snapshot.scriptPresent || snapshot.script === undefined) return undefined;
    return describeWorkflow(snapshot.script);
  } catch {
    return undefined;
  }
}

interface SummaryChildEventState {
  tokens?: number;
  terminalTokens?: number;
  terminalStatus?: SubagentStatus;
}

interface ChildEventState extends SummaryChildEventState {
  startedAt?: number;
  endedAt?: number;
  activity?: string;
  resultLine?: string;
  error?: string;
}

interface EventProjection {
  byChild: Map<string, ChildEventState>;
  narrator: NarratorLine[];
  priorGenerationNarration: NarratorLine[];
}

function scanSummaryEvents(events: readonly FrozenJson[]): Map<string, SummaryChildEventState> {
  const byChild = new Map<string, SummaryChildEventState>();
  for (const value of events) {
    const event = value as Record<string, unknown>;
    const id = typeof event.id === "string" ? event.id : undefined;
    if (!id) continue;
    const state = byChild.get(id) ?? {};
    foldSummaryChildEvent(state, event);
    byChild.set(id, state);
  }
  return byChild;
}

function scanEvents(events: readonly FrozenJson[]): EventProjection {
  const byChild = new Map<string, ChildEventState>();
  const narrator: NarratorLine[] = [];
  let priorGenerationNarration: NarratorLine[] = [];
  for (const value of events) {
    // Deliberately shape-tolerant like the historical JSONL fold. Valid JSON
    // with an unusable shape is converted by the public reader into corruption.
    const event = value as Record<string, unknown>;
    const timestamp = parseTimestamp(typeof event.timestamp === "string" ? event.timestamp : undefined);
    const type = event.type;
    if (type === "workflow_started") priorGenerationNarration = narrator.splice(0);
    if (type === "workflow_resume_refused") {
      narrator.splice(0, narrator.length, ...priorGenerationNarration);
      narrator.push({
        timestamp,
        kind: "log",
        text: sanitizeTerminalText(`resume refused: ${typeof event.error === "string" ? firstLine(event.error) : "environment changed"}`),
      });
      continue;
    }
    if (type === "log" && typeof event.message === "string") {
      narrator.push({ timestamp, kind: "log", text: event.message });
      continue;
    }
    if (type === "phase" && typeof event.title === "string") {
      narrator.push({ timestamp, kind: "phase", text: event.title });
      continue;
    }
    const id = typeof event.id === "string" ? event.id : undefined;
    if (!id) continue;
    const state = byChild.get(id) ?? {};
    foldSummaryChildEvent(state, event);
    if (type === "status" && event.status === "running" && state.startedAt === undefined) state.startedAt = timestamp;
    if (type === "status" && isTerminalStatus(event.status)) state.endedAt = timestamp;
    if (type === "activity" && typeof event.description === "string") state.activity = event.description;
    if (type === "result") {
      state.endedAt = timestamp;
      const result = event.result as Record<string, unknown> | undefined;
      const text = optionalString(result?.text);
      const error = optionalString(result?.error);
      if (text) state.resultLine = firstLine(text);
      if (error) state.error = error;
    }
    byChild.set(id, state);
  }
  return { byChild, narrator, priorGenerationNarration };
}

function foldSummaryChildEvent(state: SummaryChildEventState, event: Record<string, unknown>): void {
  if (event.type === "status" && isTerminalStatus(event.status)) state.terminalStatus = event.status;
  if (event.type === "usage") {
    const usage = event.usage as UsageSummary | undefined;
    if (usage) state.tokens = usage.input + usage.output;
  }
  if (event.type === "result") {
    const result = event.result as { status?: string; usage?: UsageSummary } | undefined;
    if (result?.usage) {
      state.tokens = result.usage.input + result.usage.output;
      state.terminalTokens = state.tokens;
    }
    if (isTerminalStatus(result?.status)) state.terminalStatus = result.status;
  }
}

function childStatuses(status: RunStatusFile | undefined, ids: string[]): SubagentStatus[] {
  return ids.map((id) => status?.children?.[id]?.status ?? "pending");
}

function runStatusFor(status: RunStatusFile | undefined, ids: string[]): SubagentStatus {
  const statuses = childStatuses(status, ids);
  return status?.status ?? (countStatuses(statuses).active ? "running" : ids.length === 0 ? "pending" : "completed");
}

function parseTimestamp(value: string | undefined): number {
  const parsed = value ? Date.parse(value) : NaN;
  return Number.isFinite(parsed) ? parsed : 0;
}

function mtimeMs(path: string): number {
  try {
    return statSync(path).mtimeMs;
  } catch {
    return 0;
  }
}

export function isLiveStatus(status: SubagentStatus): boolean {
  return status === "running" || status === "pending";
}

function isTerminalStatus(status: unknown): status is Extract<SubagentStatus, "completed" | "failed" | "aborted"> {
  return status === "completed" || status === "failed" || status === "aborted";
}

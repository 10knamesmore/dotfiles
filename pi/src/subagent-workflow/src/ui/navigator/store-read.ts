/** Three-state data source dispatcher for the /agents navigator. */

import { readdirSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { subagentRunner } from "../../runner/runner.js";
import { runOwnerIsLive } from "../../store/lease.js";
import {
  cloneRunProjection,
  corruptRunDetail,
  corruptRunSummary,
  projectRunSnapshot,
  projectRunSnapshotSummary,
  reconcileDeadOwnerProjection,
  snapshotSaysLive,
  type RunProjection,
} from "../../store/run-projection.js";
import { encodeCwd } from "../../store/run-store.js";
import { readRunSnapshot, type RunSnapshot } from "../../store/run-snapshot.js";

export type {
  ChildRow,
  RunDetail,
  RunRecordFile,
  RunStatusFile,
  RunSummary,
} from "../../store/run-projection.js";
import type { RunDetail, RunSummary } from "../../store/run-projection.js";

export interface ReadOptions {
  /** Root of the runs tree; defaults to the store's location. */
  root?: string;
  /** Extract a workflow's display name from its script.js contents. */
  describeWorkflow?: (script: string) => string;
  /** Read seams used to make snapshot, listing, and ownership races deterministic. */
  readSnapshot?: (runDir: string) => RunSnapshot;
  listRunIds?: (runsDir: string) => string[];
  ownerIsLive?: (runDir: string) => boolean;
  /**
   * Cache-key stat seam. The invalidation contract is "any change to mtime,
   * ctime, or size is a change", and a test cannot vary those one at a time on a
   * real filesystem: tmpfs derives ctime from the kernel's coarse clock, whose
   * granularity is one jiffy (4ms at CONFIG_HZ=250), so two writes inside a tick
   * are indistinguishable no matter how the test sleeps.
   */
  statFile?: (path: string) => { mtimeNs: bigint; ctimeNs: bigint; size: bigint } | undefined;
  /** In-memory projection seam; defaults to the process-wide runner. */
  ownedProjection?: (runId: string) => RunProjection | undefined;
  /** Selected non-owned runs bypass the mtime cache to preserve fresh detail. */
  bypassCache?: boolean;
}

interface RelevantFileSignatures {
  run: string;
  status: string;
  events: string;
  script: string;
  generationPending: string;
}

interface DiskRunEntry {
  runId: string;
  describeWorkflow: ReadOptions["describeWorkflow"];
  signatures: RelevantFileSignatures;
  diskLive: boolean;
  ownerLive: boolean;
  ownerProbedAt: number;
  liveSummary: RunSummary;
  reconciledSummary: RunSummary;
  liveProjection?: RunProjection;
  reconciledProjection?: RunProjection;
}

const OWNERSHIP_REPROBE_MS = 1_000;
const DISK_RUN_CAP = 500;
const diskRuns = new Map<string, DiskRunEntry>();

export function defaultRunsRoot(): string {
  return join(getAgentDir(), "subagent-workflow", "runs");
}

export function runsDirFor(cwd: string, root: string = defaultRunsRoot()): string {
  return join(root, encodeCwd(cwd));
}

/** Parse one run directory into a summary. Never throws. */
export function readRunSummary(runDir: string, runId: string, opts: ReadOptions = {}): RunSummary {
  const owned = readOwnedProjection(runId, runDir, opts);
  if (owned) {
    diskRuns.delete(resolve(runDir));
    return owned.summary;
  }
  try {
    return readDiskSummary(runDir, runId, opts);
  } catch {
    diskRuns.delete(resolve(runDir));
    return corruptRunSummary(runDir, runId);
  }
}

/** All runs for a cwd, newest first. Corrupt directories are included as dim rows. */
export function listRunSummaries(cwd: string, opts: ReadOptions = {}): RunSummary[] {
  const dir = runsDirFor(cwd, opts.root ?? defaultRunsRoot());
  let ids: string[];
  try {
    ids = (opts.listRunIds ?? readdirSync)(dir);
  } catch {
    pruneDiskRuns(dir, new Set());
    return [];
  }

  // Summaries keep their enumeration slot so the stable sort's tie-breaking
  // is independent of the owned-first read order below.
  const summaries: RunSummary[] = [];
  const diskRunsToRead: Array<{ id: string; runDir: string; slot: number }> = [];
  const present = new Set<string>();
  for (const id of ids) {
    const runDir = join(dir, id);
    try {
      if (!statSync(runDir).isDirectory()) continue;
    } catch {
      continue;
    }
    present.add(resolve(runDir));
    // Serve owned runs first so their stale disk entries free cache capacity
    // before any non-owned run in this scan tries to occupy a slot.
    const slot = summaries.length;
    summaries.length += 1;
    const owned = readOwnedProjection(id, runDir, opts);
    if (owned) {
      diskRuns.delete(resolve(runDir));
      summaries[slot] = owned.summary;
    } else {
      diskRunsToRead.push({ id, runDir, slot });
    }
  }

  pruneDiskRuns(dir, present);
  for (const { id, runDir, slot } of diskRunsToRead) {
    try {
      summaries[slot] = readDiskSummary(runDir, id, opts, present);
    } catch {
      diskRuns.delete(resolve(runDir));
      summaries[slot] = corruptRunSummary(runDir, id);
    }
  }
  return summaries.sort((left, right) => right.createdAt - left.createdAt);
}

/** Parse one run directory into a detail view. Never throws. */
export function readRunDetail(runDir: string, runId: string, opts: ReadOptions = {}): RunDetail {
  const owned = readOwnedProjection(runId, runDir, opts);
  if (owned) {
    diskRuns.delete(resolve(runDir));
    return owned.detail;
  }
  try {
    return readDiskProjection(runDir, runId, opts).detail;
  } catch {
    diskRuns.delete(resolve(runDir));
    return corruptRunDetail(runId, runDir);
  }
}

function readOwnedProjection(runId: string, runDir: string, opts: ReadOptions): RunProjection | undefined {
  const supplied = opts.ownedProjection?.(runId);
  const projection = opts.ownedProjection
    ? supplied ? cloneRunProjection(supplied) : undefined
    : subagentRunner.runProjection(runId);
  if (!projection) return undefined;
  projection.summary.runDir = runDir;
  projection.detail.runDir = runDir;
  return projection;
}

function readDiskSummary(
  runDir: string,
  runId: string,
  opts: ReadOptions,
  protectedCacheKeys?: ReadonlySet<string>,
): RunSummary {
  const cacheKey = resolve(runDir);
  const signatures = relevantFileSignatures(cacheKey, opts);
  const entry = matchingDiskRun(cacheKey, runId, opts);
  if (!opts.bypassCache && entry && sameFileSignatures(entry.signatures, signatures)) {
    reprobeOwner(entry, runDir, opts);
    touchDiskRun(cacheKey, entry);
    return summaryForCaller(summaryForEntry(entry), runDir);
  }

  const snapshot = (opts.readSnapshot ?? readRunSnapshot)(runDir);
  const ownership = diskOwnership(snapshot, runDir, entry, opts);
  const summaries = projectRunSnapshotSummary(snapshot, runId, { describeWorkflow: opts.describeWorkflow });
  const next: DiskRunEntry = {
    runId,
    describeWorkflow: opts.describeWorkflow,
    signatures,
    ...ownership,
    ...summaries,
  };
  const summary = summaryForEntry(next);
  if (summary.corrupt) diskRuns.delete(cacheKey);
  else cacheDiskRun(cacheKey, next, protectedCacheKeys);
  return summaryForCaller(summary, runDir);
}

function readDiskProjection(runDir: string, runId: string, opts: ReadOptions): RunProjection {
  const cacheKey = resolve(runDir);
  const signatures = relevantFileSignatures(cacheKey, opts);
  const entry = matchingDiskRun(cacheKey, runId, opts);
  if (!opts.bypassCache && entry && sameFileSignatures(entry.signatures, signatures)) {
    reprobeOwner(entry, runDir, opts);
    const projection = projectionForEntry(entry);
    if (projection) {
      touchDiskRun(cacheKey, entry);
      return projectionForCaller(projection, runDir);
    }
  }

  const snapshot = (opts.readSnapshot ?? readRunSnapshot)(runDir);
  const ownership = diskOwnership(snapshot, runDir, entry, opts);
  const liveProjection = projectRunSnapshot(snapshot, runId, { describeWorkflow: opts.describeWorkflow });
  const reconciledProjection = ownership.diskLive ? reconcileDeadOwnerProjection(liveProjection) : undefined;
  const next: DiskRunEntry = {
    runId,
    describeWorkflow: opts.describeWorkflow,
    signatures,
    ...ownership,
    liveSummary: liveProjection.summary,
    reconciledSummary: reconciledProjection?.summary ?? liveProjection.summary,
    liveProjection,
    ...(reconciledProjection ? { reconciledProjection } : {}),
  };
  const projection = projectionForEntry(next)!;
  if (projection.summary.corrupt) diskRuns.delete(cacheKey);
  else cacheDiskRun(cacheKey, next);
  return projectionForCaller(projection, runDir);
}

function matchingDiskRun(cacheKey: string, runId: string, opts: ReadOptions): DiskRunEntry | undefined {
  const entry = diskRuns.get(cacheKey);
  if (!entry || (entry.runId === runId && entry.describeWorkflow === opts.describeWorkflow)) return entry;
  diskRuns.delete(cacheKey);
  return undefined;
}

function diskOwnership(
  snapshot: RunSnapshot,
  runDir: string,
  entry: DiskRunEntry | undefined,
  opts: ReadOptions,
): Pick<DiskRunEntry, "diskLive" | "ownerLive" | "ownerProbedAt"> {
  const diskLive = snapshotSaysLive(snapshot);
  if (!diskLive) return { diskLive, ownerLive: false, ownerProbedAt: 0 };
  const now = Date.now();
  if (entry && now - entry.ownerProbedAt < OWNERSHIP_REPROBE_MS) {
    return { diskLive, ownerLive: entry.ownerLive, ownerProbedAt: entry.ownerProbedAt };
  }
  return {
    diskLive,
    ownerLive: (opts.ownerIsLive ?? runOwnerIsLive)(runDir),
    ownerProbedAt: now,
  };
}

function reprobeOwner(entry: DiskRunEntry, runDir: string, opts: ReadOptions): void {
  const now = Date.now();
  if (!entry.diskLive || now - entry.ownerProbedAt < OWNERSHIP_REPROBE_MS) return;
  entry.ownerLive = (opts.ownerIsLive ?? runOwnerIsLive)(runDir);
  entry.ownerProbedAt = now;
}

function summaryForEntry(entry: DiskRunEntry): RunSummary {
  return entry.diskLive && !entry.ownerLive ? entry.reconciledSummary : entry.liveSummary;
}

function projectionForEntry(entry: DiskRunEntry): RunProjection | undefined {
  return entry.diskLive && !entry.ownerLive ? entry.reconciledProjection : entry.liveProjection;
}

function touchDiskRun(cacheKey: string, entry: DiskRunEntry): void {
  diskRuns.delete(cacheKey);
  diskRuns.set(cacheKey, entry);
}

function cacheDiskRun(cacheKey: string, entry: DiskRunEntry, protectedCacheKeys?: ReadonlySet<string>): void {
  if (!diskRuns.has(cacheKey) && diskRuns.size >= DISK_RUN_CAP) {
    for (const key of diskRuns.keys()) {
      if (protectedCacheKeys?.has(key)) continue;
      diskRuns.delete(key);
      break;
    }
    if (diskRuns.size >= DISK_RUN_CAP) return;
  }
  touchDiskRun(cacheKey, entry);
}

function pruneDiskRuns(runRoot: string, present: ReadonlySet<string>): void {
  const root = resolve(runRoot);
  for (const cacheKey of diskRuns.keys()) {
    if (dirname(cacheKey) === root && !present.has(cacheKey)) diskRuns.delete(cacheKey);
  }
}

function summaryForCaller(summary: RunSummary, runDir: string): RunSummary {
  const cloned = structuredClone(summary);
  cloned.runDir = runDir;
  return cloned;
}

function projectionForCaller(projection: RunProjection, runDir: string): RunProjection {
  const cloned = cloneRunProjection(projection);
  cloned.summary.runDir = runDir;
  cloned.detail.runDir = runDir;
  return cloned;
}

function relevantFileSignatures(runDir: string, opts: ReadOptions): RelevantFileSignatures {
  const stat = opts.statFile ?? defaultStatFile;
  return {
    run: fileSignature(join(runDir, "run.json"), stat),
    status: fileSignature(join(runDir, "status.json"), stat),
    events: fileSignature(join(runDir, "events.jsonl"), stat),
    script: fileSignature(join(runDir, "script.js"), stat),
    generationPending: fileSignature(join(runDir, "generation.pending"), stat),
  };
}

type StatFile = NonNullable<ReadOptions["statFile"]>;

function defaultStatFile(path: string): ReturnType<StatFile> {
  try {
    return statSync(path, { bigint: true });
  } catch {
    return undefined;
  }
}

function fileSignature(path: string, stat: StatFile): string {
  const value = stat(path);
  return value === undefined ? "missing" : `${value.mtimeNs}:${value.ctimeNs}:${value.size}`;
}

function sameFileSignatures(left: RelevantFileSignatures, right: RelevantFileSignatures): boolean {
  return left.run === right.run
    && left.status === right.status
    && left.events === right.events
    && left.script === right.script
    && left.generationPending === right.generationPending;
}

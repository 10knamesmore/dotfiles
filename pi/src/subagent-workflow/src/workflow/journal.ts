import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { isRecord } from "../util.js";
import type { WorkflowCallIdentity, WorkflowCallScopeSegment } from "./vm.js";

/**
 * Manual compatibility epoch for CallFingerprint, distinct from the journal
 * entry format version: bump it when resolution semantics or child framing
 * change materially enough that results produced under the old semantics
 * should no longer replay, even though the entry format still parses.
 *
 * v2: the subprocess-child cutover. Children became separate pi processes,
 * extensionTools moved from a filtered in-process scan to the unfiltered
 * discovery a child process actually performs, and childExtensionExclusions
 * ceased to exist as policy. v1 entries were produced under semantics no
 * child can reproduce, so they must rerun rather than replay.
 */
export const CALL_FINGERPRINT_VERSION = 2;

/**
 * The resolved execution environment a completed call ran under. Replay
 * requires it to match the environment the call would resolve to now.
 *
 * The fingerprint verifies STATICALLY DECLARED capability shape, not
 * implementation identity or runtime behavior. Deliberately OUTSIDE it:
 * repository contents (a workflow's own children mutate them, so hashing
 * them would invalidate the run's completed work and re-run side effects;
 * authored state that matters must be interpolated into the prompt), extension
 * implementation digests and Pi/builtin-tool versions (any upgrade would void
 * all cached work, disabling resume exactly when recovery is most valuable;
 * the version field above is the deliberate escape hatch), tools an extension
 * registers dynamically during session_start (observing them requires
 * constructing a live session, which executes third-party side effects; they
 * change with extension behavior, so relevant authored state likewise belongs
 * in the prompt), and external web state (unknowable).
 */
export interface CallFingerprint {
  version: number;
  provider: string;
  modelId: string;
  thinkingLevel: string;
  cwd: string;
  /** Sorted, deduped tool names provided by extensions that reach the child. */
  extensionTools: string[];
}

export interface JournalEntry {
  v: 4;
  call: WorkflowCallIdentity;
  hash: string;
  fingerprint: CallFingerprint;
  result: unknown;
  childId: string;
}

export interface WorkflowJournal {
  /** Latest durable result for each stable async-lineage call identity. */
  entries: Map<string, JournalEntry>;
  tornTail: boolean;
}

export class JournalUnreadableError extends Error {
  constructor(readonly path: string, readonly lineNumber: number, reason: string) {
    super(`Cannot resume workflow: journal ${path} line ${lineNumber} ${reason}. Re-run the workflow fresh.`);
    this.name = "JournalUnreadableError";
  }
}

export function hashAgentPayload(payload: unknown): string {
  return createHash("sha256").update(stableJson(payload)).digest("hex");
}

export function hashScript(script: string): string {
  return createHash("sha256").update(script).digest("hex");
}

export function journalCallKey(call: WorkflowCallIdentity): string {
  return stableJson(call);
}

export function readJournal(path: string): WorkflowJournal {
  const journal: WorkflowJournal = { entries: new Map(), tornTail: false };
  if (!existsSync(path)) return journal;
  const contents = readFileSync(path, "utf8");
  const lines = contents.split("\n");
  for (const [index, line] of lines.entries()) {
    if (!line.trim()) continue;
    let value: unknown;
    try {
      value = JSON.parse(line);
    } catch {
      if (index === lines.length - 1 && !contents.endsWith("\n")) {
        journal.tornTail = true;
        continue;
      }
      throw new JournalUnreadableError(path, index + 1, "is unreadable because it contains invalid JSON");
    }
    if (isJournalEntry(value)) {
      journal.entries.set(journalCallKey(value.call), value);
      continue;
    }
    if (isPriorJournalEntry(value) || isPreLineageJournalEntry(value)) {
      throw new JournalUnreadableError(path, index + 1, "predates the current format, so this run cannot be resumed");
    }
    throw new JournalUnreadableError(path, index + 1, "is unreadable because the entry does not match the current format");
  }
  return journal;
}

/**
 * Whether `candidate` depends causally on a miss at `origin`.
 *
 * A miss invalidates later calls and branch groups in its own logical scope,
 * but never a sibling branch. This preserves successful parallel work while
 * retaining the old guarantee that a changed sequential call cannot splice in
 * a stale downstream result after a crash.
 */
export function isInCausalTail(candidate: WorkflowCallIdentity, origin: WorkflowCallIdentity): boolean {
  if (scopeStartsWith(candidate.scope, origin.scope)) {
    if (candidate.scope.length === origin.scope.length) return candidate.operation >= origin.operation;
    if (candidate.scope[origin.scope.length]!.operation >= origin.operation) return true;
  }

  // A changed branch also invalidates work after each ancestor join. Siblings
  // in the same group remain reusable, while a root call after `await
  // parallel(...)` cannot splice in state from the pre-change generation.
  for (let depth = 0; depth < origin.scope.length; depth += 1) {
    const ancestor = origin.scope.slice(0, depth);
    if (!scopeStartsWith(candidate.scope, ancestor)) continue;
    const groupOperation = origin.scope[depth]!.operation;
    if (candidate.scope.length === depth) {
      if (candidate.operation >= groupOperation) return true;
    } else if (candidate.scope[depth]!.operation > groupOperation) {
      return true;
    }
  }
  return false;
}

function scopeStartsWith(candidate: WorkflowCallScopeSegment[], prefix: WorkflowCallScopeSegment[]): boolean {
  if (candidate.length < prefix.length) return false;
  return prefix.every((segment, index) => {
    const other = candidate[index];
    return other?.operation === segment.operation && other.branch === segment.branch && other.kind === segment.kind;
  });
}

function isWorkflowCallIdentity(value: unknown): value is WorkflowCallIdentity {
  if (!isRecord(value)) return false;
  const call = value as Partial<WorkflowCallIdentity>;
  if (!Number.isSafeInteger(call.operation) || call.operation! < 0 || !Array.isArray(call.scope)) return false;
  return call.scope.every((segment) => {
    if (!isRecord(segment)) return false;
    const item = segment as Partial<WorkflowCallScopeSegment>;
    return Number.isSafeInteger(item.operation) && item.operation! >= 0
      && Number.isSafeInteger(item.branch) && item.branch! >= 0
      && (item.kind === "parallel" || item.kind === "pipeline");
  });
}

function isJournalEntry(value: unknown): value is JournalEntry {
  if (!isRecord(value)) return false;
  return value.v === 4
    && isWorkflowCallIdentity(value.call)
    && typeof value.hash === "string"
    && isCallFingerprint(value.fingerprint)
    && "result" in value
    && typeof value.childId === "string";
}

export function isCallFingerprint(value: unknown): value is CallFingerprint {
  if (!isRecord(value)) return false;
  const fingerprint = value as Partial<CallFingerprint>;
  return Number.isSafeInteger(fingerprint.version)
    && typeof fingerprint.provider === "string"
    && typeof fingerprint.modelId === "string"
    && typeof fingerprint.thinkingLevel === "string"
    && typeof fingerprint.cwd === "string"
    && isStringArray(fingerprint.extensionTools);
}

/**
 * Human-readable per-field drift between the environment a call ran under and
 * the environment it would resolve to now. Empty when replay is sound.
 */
export function describeFingerprintDrift(persisted: CallFingerprint, current: CallFingerprint): string[] {
  const drift: string[] = [];
  const scalars = ["version", "provider", "modelId", "thinkingLevel", "cwd"] as const;
  for (const field of scalars) {
    if (persisted[field] !== current[field]) {
      drift.push(`${field} was ${JSON.stringify(persisted[field])} and is now ${JSON.stringify(current[field])}`);
    }
  }
  const lists = ["extensionTools"] as const;
  for (const field of lists) {
    if (persisted[field].length !== current[field].length || persisted[field].some((item, index) => item !== current[field][index])) {
      drift.push(`${field} was ${JSON.stringify(persisted[field])} and is now ${JSON.stringify(current[field])}`);
    }
  }
  return drift;
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function isPriorJournalEntry(value: unknown): boolean {
  return isRecord(value)
    && (value.v === undefined || value.v === 2 || value.v === 3)
    && isWorkflowCallIdentity(value.call)
    && typeof value.hash === "string";
}

function isPreLineageJournalEntry(value: unknown): boolean {
  return isRecord(value) && Number.isSafeInteger(value.index) && (value.index as number) >= 0;
}

function stableJson(value: unknown): string {
  if (value === undefined) return "undefined";
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  const object = value as Record<string, unknown>;
  return `{${Object.keys(object).sort().map((key) => `${JSON.stringify(key)}:${stableJson(object[key])}`).join(",")}}`;
}

/** Policy-neutral, immutable view of one persisted run directory. */

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { errorMessage, isRecord } from "../util.js";

export type FrozenJson = null | boolean | number | string | readonly FrozenJson[] | { readonly [key: string]: FrozenJson };

interface RunSnapshotDiagnostic {
  readonly file: string;
  readonly line?: number;
  readonly problem: string;
}

export interface RunSnapshot {
  readonly runDir: string;
  readonly record: FrozenJson | undefined;
  readonly status: FrozenJson | undefined;
  readonly events: readonly FrozenJson[];
  readonly generationPending: boolean;
  readonly ownerMetadata: FrozenJson | undefined;
  /** Exact source bytes for consumers that must preserve persisted formatting. */
  readonly rawRecord: string | undefined;
  readonly rawStatus: string | undefined;
  readonly rawEvents: string | undefined;
  /** Display and save inputs are kept raw so policy stays with callers. */
  readonly script: string | undefined;
  readonly scriptPresent: boolean;
  readonly argsText: string | undefined;
  readonly diagnostics: readonly RunSnapshotDiagnostic[];
}

interface ParsedFile {
  readonly value: FrozenJson | undefined;
  readonly text: string | undefined;
}

/**
 * Read and parse the persisted files for one run. Corruption is data here:
 * consumers decide which diagnostics, shapes, and marker states are fatal. The
 * read callback is a narrow seam for deterministic read-race tests.
 */
export function readRunSnapshot(
  runDir: string,
  readText: (path: string) => string = (path) => readFileSync(path, "utf8"),
): RunSnapshot {
  let recordDiagnostics: RunSnapshotDiagnostic[] = [];
  const diagnostics: RunSnapshotDiagnostic[] = [];
  // Record marker presence at both edges of the read. Consumers still decide
  // whether an observed in-progress generation is fatal.
  const pendingBefore = existsSync(join(runDir, "generation.pending"));
  let record = readJsonFile(runDir, "run.json", recordDiagnostics, readText);
  // Match saved-run ordering: the generation commit publishes status before
  // script and args, so reading status last cannot pair an old completed status
  // with a newly committed script or args value.
  const scriptPath = join(runDir, "script.js");
  const scriptPresent = existsSync(scriptPath);
  const script = readOptionalText(scriptPath, "script.js", diagnostics, readText, scriptPresent);
  const argsText = readOptionalText(join(runDir, "args.json"), "args.json", diagnostics, readText);
  const status = readJsonFile(runDir, "status.json", diagnostics, readText);
  const eventFile = readRequiredText(runDir, "events.jsonl", diagnostics, readText);
  const events = parseEventLines(eventFile, diagnostics);
  const ownerText = readOptionalText(join(runDir, "owner.json"), "owner.json", diagnostics, readText);
  const ownerMetadata = parseJsonFile(ownerText, "owner.json", diagnostics);

  // addChild publishes run.json before status.json and its event. If the first
  // record read raced that publish, either later file can reveal the new child.
  // Re-read only the early file once rather than repeating the full snapshot.
  if (referencesUnrecordedChild(record.value, status.value, events)) {
    recordDiagnostics = [];
    record = readJsonFile(runDir, "run.json", recordDiagnostics, readText);
    if (recordDiagnostics.length === 0 && referencesUnrecordedChild(record.value, status.value, events)) {
      recordDiagnostics.push({
        file: "run.json",
        problem: "run.json still omits child IDs referenced by status.json or events.jsonl after retry",
      });
    }
    // A persistently malformed record keeps its ordinary read/parse diagnostic;
    // a valid but persistently incoherent record gets the single diagnostic above.
  }
  const pendingAfter = existsSync(join(runDir, "generation.pending"));

  const frozenDiagnostics = Object.freeze([...recordDiagnostics, ...diagnostics].map((diagnostic) => Object.freeze(diagnostic)));
  return Object.freeze({
    runDir,
    record: record.value,
    status: status.value,
    events: Object.freeze(events),
    generationPending: pendingBefore || pendingAfter,
    ownerMetadata: ownerMetadata.value,
    rawRecord: record.text,
    rawStatus: status.text,
    rawEvents: eventFile,
    script,
    scriptPresent,
    argsText,
    diagnostics: frozenDiagnostics,
  });
}

function readJsonFile(
  runDir: string,
  file: string,
  diagnostics: RunSnapshotDiagnostic[],
  readText: (path: string) => string,
): ParsedFile {
  const text = readRequiredText(runDir, file, diagnostics, readText);
  return parseJsonFile(text, file, diagnostics);
}

function parseJsonFile(
  text: string | undefined,
  file: string,
  diagnostics: RunSnapshotDiagnostic[],
): ParsedFile {
  if (text === undefined) return { value: undefined, text };
  try {
    return { value: freezeJson(JSON.parse(text)), text };
  } catch (error) {
    diagnostics.push({ file, problem: errorMessage(error) });
    return { value: undefined, text };
  }
}

function readRequiredText(
  runDir: string,
  file: string,
  diagnostics: RunSnapshotDiagnostic[],
  readText: (path: string) => string,
): string | undefined {
  try {
    return readText(join(runDir, file));
  } catch (error) {
    diagnostics.push({ file, problem: errorMessage(error) });
    return undefined;
  }
}

function readOptionalText(
  path: string,
  file: string,
  diagnostics: RunSnapshotDiagnostic[],
  readText: (path: string) => string,
  diagnoseMissing = false,
): string | undefined {
  try {
    return readText(path);
  } catch (error) {
    if ((error as NodeJS.ErrnoException | undefined)?.code === "ENOENT" && !diagnoseMissing) return undefined;
    diagnostics.push({ file, problem: errorMessage(error) });
    return undefined;
  }
}

function parseEventLines(text: string | undefined, diagnostics: RunSnapshotDiagnostic[]): FrozenJson[] {
  if (text === undefined) return [];
  const events: FrozenJson[] = [];
  for (const [index, line] of text.split("\n").entries()) {
    if (!line.trim()) continue;
    try {
      events.push(freezeJson(JSON.parse(line)));
    } catch (error) {
      diagnostics.push({ file: "events.jsonl", line: index + 1, problem: errorMessage(error) });
    }
  }
  return events;
}

function referencesUnrecordedChild(
  record: FrozenJson | undefined,
  status: FrozenJson | undefined,
  events: readonly FrozenJson[],
): boolean {
  const recordedIds = new Set<string>();
  const recordObject = jsonObject(record);
  const children = recordObject?.children;
  if (Array.isArray(children)) {
    for (const child of children) {
      const id = jsonObject(child)?.id;
      if (typeof id === "string") recordedIds.add(id);
    }
  }

  const statusChildren = jsonObject(jsonObject(status)?.children);
  if (statusChildren) {
    for (const id of Object.keys(statusChildren)) {
      if (!recordedIds.has(id)) return true;
    }
  }
  for (const event of events) {
    const id = jsonObject(event)?.id;
    if (typeof id === "string" && !recordedIds.has(id)) return true;
  }
  return false;
}

export function jsonObject(value: unknown): Record<string, unknown> | undefined {
  return isRecord(value) ? value : undefined;
}

function freezeJson(value: unknown): FrozenJson {
  if (value !== null && typeof value === "object") {
    const pending: object[] = [value];
    while (pending.length > 0) {
      const current = pending.pop()!;
      for (const item of Object.values(current)) {
        if (item !== null && typeof item === "object") pending.push(item);
      }
      Object.freeze(current);
    }
  }
  return value as FrozenJson;
}

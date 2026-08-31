/**
 * Saved-workflow discovery and persistence (save-a-run).
 *
 * A proven run's script can be saved to a project scope (`.pi/workflows/<name>.js`)
 * or a user scope (`~/.pi/agent/subagent-workflow/workflows/<name>.js`) and later
 * re-invoked with fresh args, either as `/wf-<name>` or via the workflow tool's
 * `script: "@<name>"` reference. User scope is always available. Project scope
 * requires the current Pi context to trust the project; when it is available,
 * it wins name conflicts.
 */

import { existsSync, mkdirSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { replaceAtomicFile } from "../store/atomic-file.js";
import { encodeCwd } from "../store/run-store.js";
import { readRunSnapshot, type RunSnapshot } from "../store/run-snapshot.js";
import { isRecord } from "../util.js";

export type SavedScope = "project" | "user";

export interface SavedWorkflow {
  name: string;
  scope: SavedScope;
  path: string;
}

/** kebab-case, matching workflow meta.name. Also guards against path traversal. */
const NAME = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
const RUN_ID = /^(run|workflow)-[0-9a-z]+-[0-9a-z]+$/;

export function isValidWorkflowName(name: string): boolean {
  return NAME.test(name);
}

/** Resolves a run id only when it names a direct child of this cwd's runs directory. */
export function resolveRunDir(cwd: string, runId: string, runsRoot: string = workflowRunsRoot()): string {
  if (!RUN_ID.test(runId)) throw new Error(`Invalid run id: "${runId}"`);
  const cwdRunsDir = resolve(runsRoot, encodeCwd(cwd));
  const runDir = resolve(cwdRunsDir, runId);
  if (dirname(runDir) !== cwdRunsDir) throw new Error(`Invalid run id path: "${runId}"`);
  return runDir;
}

function projectWorkflowsDir(cwd: string): string {
  return join(resolve(cwd), ".pi", "workflows");
}

export function userWorkflowsDir(): string {
  return join(getAgentDir(), "subagent-workflow", "workflows");
}

interface SavedWorkflowAccess {
  /** Current Pi project trust. Production callers must pass this explicitly. */
  projectTrusted?: boolean;
  /** An injected project directory explicitly authorizes that storage seam. */
  projectDir?: string;
  userDir?: string;
}

/** Saved workflows visible from `cwd`, keyed by name. Project scope is fail-closed. */
export function discoverSavedWorkflows(cwd: string, access: SavedWorkflowAccess = {}): Map<string, SavedWorkflow> {
  const result = new Map<string, SavedWorkflow>();
  for (const workflow of listScope(access.userDir ?? userWorkflowsDir(), "user")) result.set(workflow.name, workflow);
  if (projectScopeAllowed(access)) {
    for (const workflow of listScope(access.projectDir ?? projectWorkflowsDir(cwd), "project")) result.set(workflow.name, workflow);
  }
  return result;
}

export function resolveSavedWorkflow(name: string, cwd: string, access: SavedWorkflowAccess = {}): SavedWorkflow | undefined {
  return discoverSavedWorkflows(cwd, access).get(name);
}

export function readSavedScript(workflow: SavedWorkflow): string {
  return readFileSync(workflow.path, "utf8");
}

function listScope(dir: string, scope: SavedScope): SavedWorkflow[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((file) => file.endsWith(".js"))
    .map((file) => ({ name: file.slice(0, -3), scope, path: join(dir, file) }))
    .filter((workflow) => isValidWorkflowName(workflow.name));
}

interface SaveProvenance {
  runId: string;
  date: string;
  args: unknown;
}

interface SaveWorkflowInput {
  name: string;
  scope: SavedScope;
  cwd: string;
  script: string;
  provenance: SaveProvenance;
}

/** Writes the script verbatim under a provenance header comment. Returns the path. */
export function saveWorkflow(input: SaveWorkflowInput, access: SavedWorkflowAccess = {}): string {
  if (!isValidWorkflowName(input.name)) {
    throw new Error(`Cannot save workflow: "${input.name}" is not a kebab-case name`);
  }
  if (input.scope === "project" && !projectScopeAllowed(access)) {
    throw new Error("Cannot save a project workflow because the current project is not trusted");
  }
  const dir = input.scope === "project"
    ? (access.projectDir ?? projectWorkflowsDir(input.cwd))
    : (access.userDir ?? userWorkflowsDir());
  mkdirSync(dir, { recursive: true });
  const path = join(dir, `${input.name}.js`);
  const existingMode = fileMode(path);
  replaceAtomicFile(path, `${provenanceHeader(input.provenance)}${ensureTrailingNewline(input.script)}`, {
    mode: existingMode ?? 0o666,
    exactMode: existingMode !== undefined,
  });
  return path;
}

function projectScopeAllowed(access: SavedWorkflowAccess): boolean {
  return access.projectTrusted === true || access.projectDir !== undefined;
}

function fileMode(path: string): number | undefined {
  try {
    return statSync(path).mode & 0o777;
  } catch {
    return undefined;
  }
}

function provenanceHeader(provenance: SaveProvenance): string {
  const args = safeJson(provenance.args);
  return [
    "// Saved workflow (pi-subagent-workflow)",
    `// runId: ${provenance.runId}`,
    `// saved: ${provenance.date}`,
    `// args: ${args}`,
    "",
    "",
  ].join("\n");
}

function ensureTrailingNewline(text: string): string {
  return text.endsWith("\n") ? text : `${text}\n`;
}

function safeJson(value: unknown): string {
  try {
    // JSON permits literal Unicode line and paragraph separators, but
    // JavaScript treats them as line terminators. Escape both so provenance
    // always remains inside its // comment when the saved file is executed.
    return (JSON.stringify(value ?? null) ?? "null")
      .replaceAll("\u2028", "\\u2028")
      .replaceAll("\u2029", "\\u2029");
  } catch {
    return "null";
  }
}

export interface WorkflowRunInfo {
  runId: string;
  runDir: string;
  script: string;
  args: unknown;
  createdAt: string;
}

function workflowRunsRoot(): string {
  return join(getAgentDir(), "subagent-workflow", "runs");
}

/** Newest completed workflow run for `cwd`, for the argument-less `/workflow-save`. */
export function findLatestCompletedWorkflowRun(cwd: string, runsRoot: string = workflowRunsRoot()): WorkflowRunInfo | undefined {
  const dir = join(runsRoot, encodeCwd(cwd));
  if (!existsSync(dir)) return undefined;
  let best: WorkflowRunInfo | undefined;
  for (const runId of readdirSync(dir)) {
    let runDir: string;
    try {
      runDir = resolveRunDir(cwd, runId, runsRoot);
    } catch {
      continue;
    }
    const info = readCompletedWorkflowRun(runDir, runId);
    if (info && (!best || info.createdAt > best.createdAt)) best = info;
  }
  return best;
}

export function findWorkflowRunById(cwd: string, runId: string, runsRoot: string = workflowRunsRoot()): WorkflowRunInfo | undefined {
  try {
    return readCompletedWorkflowRun(resolveRunDir(cwd, runId, runsRoot), runId);
  } catch {
    return undefined;
  }
}

function readCompletedWorkflowRun(runDir: string, runId: string): WorkflowRunInfo | undefined {
  try {
    if (!statSync(runDir).isDirectory()) return undefined;
  } catch {
    return undefined;
  }
  const snapshot = readRunSnapshot(runDir);
  if (snapshot.generationPending) return undefined;
  const info = readWorkflowRunAt(snapshot, runId);
  if (!info) return undefined;
  const status = isRecord(snapshot.status) ? snapshot.status : undefined;
  return status?.status === "completed" ? info : undefined;
}

function readWorkflowRunAt(snapshot: RunSnapshot, runId: string): WorkflowRunInfo | undefined {
  const record = isRecord(snapshot.record)
    ? snapshot.record as { kind?: string; createdAt?: string }
    : undefined;
  if (!record || record.kind !== "workflow" || !snapshot.scriptPresent) return undefined;
  if (snapshot.script === undefined) {
    const problem = snapshot.diagnostics.find((diagnostic) => diagnostic.file === "script.js")?.problem;
    throw new Error(problem ?? `Unable to read workflow script in ${snapshot.runDir}`);
  }
  return {
    runId,
    runDir: snapshot.runDir,
    script: snapshot.script,
    args: parseSavedArgs(snapshot.argsText),
    createdAt: record.createdAt ?? "",
  };
}

function parseSavedArgs(text: string | undefined): unknown {
  if (text === undefined) return null;
  try {
    return JSON.parse(text) ?? null;
  } catch {
    return null;
  }
}

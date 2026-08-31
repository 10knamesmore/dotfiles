/**
 * Subprocess child builder: resolve the requested configuration, start one
 * isolated `pi --mode rpc` process, verify its active tools, and return the
 * RPC-backed session consumed by the runner.
 */

import { randomUUID } from "node:crypto";
import { accessSync, constants, mkdirSync, realpathSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { childLabel } from "../../util.js";
import { missingExplicitTools, resolveModel, SUBAGENT_FRAMING, type ConstructedChild, type ParentContext } from "../child.js";
import type { SchemaCapture } from "../schema-tool.js";
import type { SubagentSpec } from "../../types.js";
import { buildChildArgs } from "./child-args.js";
import { RpcChildSession } from "./rpc-child-session.js";
import { spawnChildRpc, type ChildRpc } from "./rpc-transport.js";
import { readToolReport, SHIM_SPEC_ENV, type ShimToolReport } from "./shim-contract.js";

/** How long a child pi process gets to boot, load extensions, and report its toolset. */
const TOOL_REPORT_TIMEOUT_MS = 60_000;
const TOOL_REPORT_POLL_MS = 100;

/** Extension UI methods that block the child until someone answers. */
const ANSWER_REQUIRED_UI_METHODS = new Set(["select", "confirm", "input", "editor"]);

/**
 * The pi CLI entry to run children with. Prefer the exact entry the parent
 * process is running, so children always match the parent's pi version; fall
 * back to the pi-coding-agent copy this extension resolves. Subpath exports
 * are resolved by hand (createRequire + sibling path) because the host's
 * extension loader may shim import.meta.resolve without exports-map support.
 */
export function resolveChildPiEntry(): string {
  const invoked = process.argv[1];
  if (invoked) {
    try {
      const real = realpathSync(invoked);
      if (/[\\/]cli\.js$/.test(real)) return real;
    } catch {
      // The invoked script may not exist as a file (embedded runtimes); fall through.
    }
  }
  // Bare id only: some loaders mis-resolve exports-map subpaths, and the
  // return value may be a file URL (node, bun) or a plain path (jiti shims).
  const resolved = import.meta.resolve("@earendil-works/pi-coding-agent");
  const index = resolved.startsWith("file:") ? fileURLToPath(resolved) : resolved;
  return join(dirname(index), "cli.js");
}

/** The shim ships beside this extension. Match the host's file extension so a
 * compiled `.js` install resolves child-shim.js, not the unshipped .ts. */
export function resolveShimPath(selfPath: string): string {
  return join(dirname(selfPath), `child-shim${extname(selfPath) || ".ts"}`);
}

type SpawnRpc = (command: readonly string[], options: { cwd: string; env?: NodeJS.ProcessEnv }) => ChildRpc;

export interface LaunchArtifactOverrides {
  childPiEntry?: string;
  shimPath?: string;
  forkSessionFile?: string;
}

function validateLaunchArtifact(path: string, label: string): void {
  let isFile: boolean;
  try {
    isFile = statSync(path).isFile();
    accessSync(path, constants.R_OK);
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(`${label} ${JSON.stringify(path)} is not a readable regular file: ${detail}`, { cause: error });
  }
  if (!isFile) throw new Error(`${label} ${JSON.stringify(path)} is not a readable regular file: path is not a regular file`);
}

/** Side-effect-free checks that must pass before any launch artifact is written. */
export function preflightSubprocessChild(spec: SubagentSpec, parent: ParentContext, overrides: LaunchArtifactOverrides = {}) {
  const cwd = spec.cwd ?? parent.ctx.cwd;
  let isDirectory: boolean;
  try {
    isDirectory = statSync(cwd).isDirectory();
    accessSync(cwd, constants.X_OK);
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    throw new Error(`Child cwd ${JSON.stringify(cwd)} is not usable as a spawn cwd: ${detail}`, { cause: error });
  }
  if (!isDirectory) throw new Error(`Child cwd ${JSON.stringify(cwd)} is not usable as a spawn cwd: path is not a directory`);

  const { model, thinking } = resolveModel(spec, parent.ctx, parent.thinkingLevel);
  const tools = spec.schema && spec.tools ? [...new Set([...spec.tools, "report_result"])] : spec.tools;
  const excludeTools = spec.schema ? spec.excludeTools?.filter((name) => name !== "report_result") : spec.excludeTools;
  for (const name of [...(tools ?? []), ...(excludeTools ?? [])]) {
    if (name.includes(",")) throw new Error(`Tool name ${JSON.stringify(name)} contains a comma and cannot cross the pi CLI boundary`);
  }
  const shimPath = overrides.shimPath ?? resolveShimPath(parent.selfPath);
  const childPiEntry = overrides.childPiEntry ?? resolveChildPiEntry();
  validateLaunchArtifact(childPiEntry, "Child pi CLI entry");
  validateLaunchArtifact(shimPath, "Child shim");
  if (overrides.forkSessionFile !== undefined) validateLaunchArtifact(overrides.forkSessionFile, "Fork session file");
  return { cwd, model, thinking, tools, excludeTools, shimPath, childPiEntry };
}

/** The child construction backend; the runner's ChildBuilder seam. */
export async function spawnSubprocessChild(
  spec: SubagentSpec,
  parent: ParentContext,
  persistence: { sessionsDir: string; forkSessionFile?: string },
  spawnRpc: SpawnRpc = spawnChildRpc,
): Promise<ConstructedChild> {
  // Everything that can fail synchronously (cwd, model, arg validation, entry
  // resolution) runs before any file is written. buildChildArgs rechecks its
  // own pure boundary rules as construction-time defense in depth.
  const { cwd, model, thinking, tools, excludeTools, shimPath, childPiEntry } = preflightSubprocessChild(spec, parent, {
    forkSessionFile: persistence.forkSessionFile,
  });
  const capture: SchemaCapture | undefined = spec.schema ? { called: false } : undefined;
  const args = buildChildArgs({
    provider: model.provider,
    modelId: model.id,
    thinkingLevel: thinking,
    tools,
    excludeTools,
    sessionDir: persistence.sessionsDir,
    forkSessionFile: persistence.forkSessionFile,
    appendSystemPrompt: SUBAGENT_FRAMING,
    shimPath,
  });

  const shimDir = join(dirname(persistence.sessionsDir), "shim");
  mkdirSync(shimDir, { recursive: true, mode: 0o700 });
  const stem = join(shimDir, randomUUID());
  const toolReportPath = `${stem}.tools.json`;
  const specPath = `${stem}.spec.json`;
  writeFileSync(specPath, JSON.stringify({ schema: spec.schema, toolReportPath }), { mode: 0o600 });

  let rpc: ChildRpc;
  try {
    rpc = spawnRpc([process.execPath, childPiEntry, ...args], {
      cwd,
      env: { ...process.env, [SHIM_SPEC_ENV]: specPath },
    });
  } catch (error) {
    rmSync(specPath, { force: true });
    rmSync(toolReportPath, { force: true });
    throw error;
  }
  // The spec file must outlive the CHILD, not just construction: pi re-runs
  // extension factories on session reload, and the shim re-reads its spec
  // then. Tie cleanup to process exit; rm never throws into the child's fate.
  void rpc.exited.then(() => {
    rmSync(specPath, { force: true });
    rmSync(toolReportPath, { force: true });
  }).catch(() => undefined);

  try {
    // A headless child has no person to answer extension UI; cancel instead
    // of letting a third-party extension block the child forever.
    rpc.onEvent((event) => {
      if (event.type !== "extension_ui_request" || typeof event.id !== "string") return;
      if (ANSWER_REQUIRED_UI_METHODS.has(event.method as string)) rpc.send({ type: "extension_ui_response", id: event.id, cancelled: true });
    });

    const report = await awaitToolReport(rpc, toolReportPath);
    // Recursion guard, fail-closed: --exclude-tools should have removed these,
    // but if a report still lists them (extension installed under a name the
    // exclusion missed) refuse the spawn rather than allow child trees to
    // recurse. Mirrors the in-process activeTools assertion.
    const guardBreach = report.activeTools.filter((name) => name === "subagent" || name === "workflow");
    if (guardBreach.length > 0) {
      throw new Error(`Recursion guard: child still exposes ${guardBreach.join(", ")} despite the exclusion.`);
    }
    const missing = missingExplicitTools(spec.tools, report.activeTools);
    if (missing.length > 0) {
      throw new Error(
        `Missing explicitly requested tools: ${missing.join(", ")}. No active tool by that name resolved for the child at ${cwd}: `
        + `the providing extension may not be installed there, may be detached from children, or the name may be misspelled or in excludeTools. `
        + `Active tools: ${report.activeTools.join(", ") || "none"}.`,
      );
    }
    const session = await RpcChildSession.start(rpc);
    if (capture) wireSchemaCapture(session, capture);
    return {
      session,
      resolved: {
        provider: model.provider,
        modelId: model.id,
        thinkingLevel: thinking,
        tools: report.activeTools,
        cwd,
        label: childLabel(spec),
      },
      schemaCapture: capture,
    };
  } catch (error) {
    // Await real exit before rethrowing: the runner's failure path has no
    // session to dispose, so this is the only place that guarantees the
    // process is gone before worktree cleanup or the next admission runs.
    rpc.kill("SIGKILL");
    await rpc.exited;
    throw error;
  }
}

/**
 * The report_result call happens inside the child; the parent observes it
 * through tool events, captures the validated arguments, and ends the turn,
 * mirroring the in-process capture-and-terminate contract.
 */
function wireSchemaCapture(session: RpcChildSession, capture: SchemaCapture): void {
  capture.terminate = () => {
    void session.abort().catch(() => undefined);
  };
  type CallState =
    | { state: "active"; toolName: string; args: unknown }
    | { state: "terminal"; toolName: string };
  interface Candidate { id: string; value: unknown }
  const calls = new Map<string, CallState>();
  let candidate: Candidate | undefined;
  let published = false;
  let attemptPoisoned = false;
  let activeAttempt: number | undefined;
  const clearAttempt = (): void => {
    calls.clear();
    candidate = undefined;
  };
  const poisonAttempt = (): void => {
    attemptPoisoned = true;
  };

  session.onPromptAttempt((attemptEvent) => {
    if (published) return;
    if (attemptEvent.type === "started") {
      activeAttempt = attemptEvent.attempt;
      clearAttempt();
      attemptPoisoned = false;
      return;
    }
    if (attemptEvent.attempt !== activeAttempt) return;
    if (attemptEvent.type === "discarded") {
      clearAttempt();
      attemptPoisoned = false;
      activeAttempt = undefined;
      return;
    }
    if (attemptEvent.type === "settled") {
      const state = candidate && calls.get(candidate.id);
      const hasActiveCall = [...calls.values()].some((call) => call.state === "active");
      if (
        !attemptPoisoned
        && !hasActiveCall
        && candidate
        && state?.state === "terminal"
        && state.toolName === "report_result"
      ) {
        capture.called = true;
        capture.value = candidate.value;
        published = true;
      }
      clearAttempt();
      attemptPoisoned = false;
      activeAttempt = undefined;
      return;
    }
    const event = attemptEvent.event;
    if (event.type !== "tool_execution_start" && event.type !== "tool_execution_end") return;
    const existing = calls.get(event.toolCallId);
    if (event.type === "tool_execution_start") {
      if (existing || (event.toolName === "report_result" && candidate)) poisonAttempt();
      if (!existing) calls.set(event.toolCallId, { state: "active", toolName: event.toolName, args: event.args });
      return;
    }
    if (!existing || existing.state !== "active" || existing.toolName !== event.toolName) {
      poisonAttempt();
      return;
    }
    calls.set(event.toolCallId, { state: "terminal", toolName: event.toolName });
    if (event.toolName !== "report_result" || event.isError) return;
    if (candidate) {
      poisonAttempt();
      return;
    }
    candidate = { id: event.toolCallId, value: existing.args };
    capture.terminate?.();
  });
}

async function awaitToolReport(rpc: ChildRpc, path: string): Promise<ShimToolReport> {
  const deadline = Date.now() + TOOL_REPORT_TIMEOUT_MS;
  let exited = false;
  rpc.onExit(() => { exited = true; });
  for (;;) {
    const report = await readToolReport(path);
    if (report) return report;
    if (exited) throw new Error(`Child pi process exited before reporting its toolset. Stderr: ${rpc.stderrTail() || "(empty)"}`);
    if (Date.now() > deadline) throw new Error(`Child pi process did not report its toolset within ${TOOL_REPORT_TIMEOUT_MS / 1_000}s. Stderr: ${rpc.stderrTail() || "(empty)"}`);
    await new Promise((resolve) => setTimeout(resolve, TOOL_REPORT_POLL_MS));
  }
}

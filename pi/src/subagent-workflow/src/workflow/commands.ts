/**
 * Saved-workflow commands.
 *
 * - `/workflow-save <runId?>` saves a completed workflow run (default: the most
 *   recent completed run for this cwd) to project or user scope as `<name>.js`.
 * - `/wf-<name>` runs a saved workflow with optional args text passed through.
 *
 * Discovery happens on `session_start` using the session's effective cwd. User
 * scope is always visible; project scope is discovered only while that context
 * trusts the project. A freshly saved workflow is runnable immediately via the
 * tool's `script: "@<name>"` reference; its `/wf-<name>` command is registered on
 * save when the host allows runtime registration, otherwise after the next restart.
 */

import type { ExtensionAPI, ExtensionCommandContext } from "@earendil-works/pi-coding-agent";
import type { ParentContext } from "../runner/child.js";
import type { ThinkingLevel } from "../types.js";
import { sanitizeTerminalText } from "../ui/sanitize.js";
import type { ApproveLaunch, LaunchPlan, WorkflowApprovalPolicy } from "./approval.js";
import type { ConsentStore } from "./consent.js";
import { deliverWorkflowInBackground, launchWorkflow, type StartedWorkflow } from "./launch.js";
import { parseWorkflowScript } from "./parser.js";
import {
  discoverSavedWorkflows,
  findLatestCompletedWorkflowRun,
  findWorkflowRunById,
  readSavedScript,
  resolveSavedWorkflow,
  saveWorkflow,
  type SavedScope,
  type SavedWorkflow,
  type WorkflowRunInfo,
} from "./saved.js";
import { normalizeArgs } from "./workflow-runner.js";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage } from "../util.js";

export interface SavedCommandServices {
  consent: ConsentStore;
  approve: ApproveLaunch;
  approvalPolicy: () => WorkflowApprovalPolicy;
  observeRun?: (run: StartedWorkflow, ctx: ExtensionCommandContext) => void;
  selfPath: string;
}

/** Register saved-workflow commands and return the shared save flow for the navigator. */
export function registerSavedWorkflowCommands(pi: ExtensionAPI, services: SavedCommandServices) {
  const registered = new Set<string>();
  const registerWf = (workflow: SavedWorkflow): void => {
    const command = `wf-${workflow.name}`;
    if (registered.has(command)) return;
    registered.add(command);
    pi.registerCommand(command, {
      description: `Run saved workflow ${workflow.name} (${workflow.scope} scope)`,
      handler: (argsText, ctx) => runSavedWorkflow(pi, services, workflow.name, argsText, ctx),
    });
  };

  // Discover from the session's effective cwd, not process.cwd(). Extensions are
  // reloaded on every session switch (resume/new/fork), each time firing
  // session_start with the destination session's cwd against a fresh command
  // registry - so a session resumed in another project sees that project's
  // saved workflows, and the previous project's project-scope commands (which
  // cannot be unregistered) never carry over.
  pi.on("session_start", (_event, ctx) => {
    for (const workflow of discoverSavedWorkflows(ctx.cwd, { projectTrusted: ctx.isProjectTrusted() }).values()) registerWf(workflow);
  });

  pi.registerCommand("workflow-save", {
    description: "Save a completed workflow run for reuse by @<name> or /wf-<name>",
    handler: (argsText, ctx) => saveWorkflowCommand(argsText.trim(), ctx, registerWf),
  });

  return async (runId: string, ctx: ExtensionCommandContext): Promise<void> => {
    const run = findWorkflowRunById(ctx.cwd, runId);
    if (!run) {
      ctx.ui.notify(`No saved workflow script for run ${runId} (only workflow runs can be saved)`, "warning");
      return;
    }
    return performSave(run, ctx, registerWf);
  };
}

async function runSavedWorkflow(pi: ExtensionAPI, services: SavedCommandServices, name: string, argsText: string, ctx: ExtensionCommandContext): Promise<void> {
  const workflow = resolveSavedWorkflow(name, ctx.cwd, { projectTrusted: ctx.isProjectTrusted() });
  if (!workflow) {
    ctx.ui.notify(`Saved workflow "${name}" was not found`, "error");
    return;
  }
  let plan: LaunchPlan;
  try {
    const script = readSavedScript(workflow);
    const parsed = parseWorkflowScript(script);
    const trimmed = argsText.trim();
    plan = { workflow: parsed, args: normalizeArgs(trimmed === "" ? undefined : trimmed), origin: "saved" };
  } catch (error) {
    ctx.ui.notify(`Cannot run ${name}: ${errorMessage(error)}`, "error");
    return;
  }
  const parent: ParentContext = {
    ctx,
    thinkingLevel: pi.getThinkingLevel() as ThinkingLevel,
    selfPath: services.selfPath,
  };
  try {
    const { started, execution } = await launchWorkflow(
      pi,
      parent,
      { plan },
      { approve: services.approve, ctx, deps: { consent: services.consent, policy: services.approvalPolicy() } },
    );
    try {
      services.observeRun?.(started, ctx);
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] workflow observer failed: ${sanitizeTerminalText(errorMessage(error))}`);
    }
    deliverWorkflowInBackground(pi, execution, ctx.sessionManager.getSessionId());
    ctx.ui.notify(`Workflow ${name} started (${started.runId})`, "info");
  } catch (error) {
    ctx.ui.notify(`Workflow ${name} was not launched: ${errorMessage(error)}`, "warning");
  }
}

async function saveWorkflowCommand(runIdArg: string, ctx: ExtensionCommandContext, registerWf: (workflow: SavedWorkflow) => void): Promise<void> {
  const run = runIdArg ? findWorkflowRunById(ctx.cwd, runIdArg) : findLatestCompletedWorkflowRun(ctx.cwd);
  if (!run) {
    ctx.ui.notify(runIdArg ? `Workflow run "${runIdArg}" was not found for this project` : "No completed workflow run found for this project", "error");
    return;
  }
  return performSave(run, ctx, registerWf);
}

async function performSave(run: WorkflowRunInfo, ctx: ExtensionCommandContext, registerWf: (workflow: SavedWorkflow) => void): Promise<void> {
  let name: string;
  try {
    name = parseWorkflowScript(run.script).meta.name;
  } catch (error) {
    ctx.ui.notify(`Cannot save run ${run.runId}: ${errorMessage(error)}`, "error");
    return;
  }
  const projectChoice = "Project (.pi/workflows)";
  const userChoice = "User (~/.pi/agent/subagent-workflow/workflows)";
  const scopeChoice = await ctx.ui.select(
    `Save workflow "${name}" (from run ${run.runId})`,
    ctx.isProjectTrusted() ? [projectChoice, userChoice] : [userChoice],
  );
  if (!scopeChoice) return;
  const scope: SavedScope = scopeChoice === projectChoice ? "project" : "user";
  const projectTrusted = ctx.isProjectTrusted();
  if (scope === "project" && !projectTrusted) {
    ctx.ui.notify("Project workflows can only be saved after the current project is trusted", "warning");
    return;
  }
  let path: string;
  try {
    path = saveWorkflow(
      { name, scope, cwd: ctx.cwd, script: run.script, provenance: { runId: run.runId, date: new Date().toISOString(), args: run.args } },
      { projectTrusted },
    );
  } catch (error) {
    ctx.ui.notify(`Save failed: ${errorMessage(error)}`, "error");
    return;
  }
  let commandRegistered = true;
  try {
    registerWf({ name, scope, path });
  } catch {
    commandRegistered = false;
    // Some hosts only accept command registration at load; the workflow is usable now
    // via script: "@<name>" and as /wf-<name> after the next restart.
  }
  const runHint = commandRegistered
    ? `Run it with /wf-${name}`
    : `Run it now with the workflow tool using script: "@${name}"; /wf-${name} will be available next session`;
  ctx.ui.notify(`Saved workflow ${name} to ${path}. ${runHint}`, "info");
}

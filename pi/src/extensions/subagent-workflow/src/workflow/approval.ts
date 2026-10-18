/**
 * Workflow launch approval in Pi's TUI.
 *
 * Non-TUI modes (json / print / rpc) auto-approve: a headless caller already made
 * an explicit tool call. In the TUI a dialog previews the workflow (name,
 * description and phases) and offers: Open in editor / Accept / Reject /
 * Reject with reason. An inline script is a new script every time, so it never
 * skips the dialog. Rejection throws a clear error the model can relay.
 */

import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { SettingsManager, type ExtensionUIContext } from "@earendil-works/pi-coding-agent";
import type { WorkflowSettings } from "@dotfiles/pi-config";
import { reportDiagnostic } from "../diagnostics.js";
import { sanitizeTerminalText } from "../ui/sanitize.js";
import { errorMessage } from "../util.js";
import { parseWorkflowScript, type ParsedWorkflow } from "./parser.js";

/** Run mode; mirrors pi's ExtensionMode, which the package does not re-export. */
export type ExtensionMode = "tui" | "rpc" | "json" | "print";

export interface LaunchPlan {
  readonly workflow: ParsedWorkflow;
  /** Undefined preserves inherited args on resume. */
  readonly args: unknown;
}

/** The narrow context the approver needs, satisfied by ExtensionContext. */
export interface ApprovalContext {
  mode: ExtensionMode;
  cwd: string;
  isProjectTrusted(): boolean;
  ui: Pick<ExtensionUIContext, "select" | "input" | "custom" | "notify">;
}

export type WorkflowApprovalPolicy = WorkflowSettings["workflowApproval"];

const OPEN = "Open in editor";
const ACCEPT = "Accept";
const REJECT = "Reject";
const REJECT_WITH_REASON = "Reject with reason";

interface WorkflowDraft {
  script: string;
  argsJson: string;
}

/** Edit both inputs in temporary files, releasing the TUI until the external editor exits. */
async function editWorkflowDraft(draft: WorkflowDraft, ctx: ApprovalContext): Promise<WorkflowDraft> {
  const command = SettingsManager.create(ctx.cwd, undefined, {
    projectTrusted: ctx.isProjectTrusted(),
  }).getExternalEditorCommand();
  const directory = await mkdtemp(join(tmpdir(), "pi-workflow-editor-"));
  const scriptPath = join(directory, "workflow.js");
  const argsPath = join(directory, "args.json");
  try {
    await writeFile(scriptPath, draft.script, "utf8");
    await writeFile(argsPath, draft.argsJson, "utf8");
    reportDiagnostic(`[subagent-workflow] opening workflow editor: ${command}`);
    await ctx.ui.custom<void>(async (tui, _theme, _keybindings, done) => {
      tui.stop();
      try {
        const [editor, ...editorArgs] = command.split(" ");
        if (basename(editor!) === "nvim") editorArgs.push("-O");
        await new Promise<void>((resolve, reject) => {
          const child = spawn(editor!, [...editorArgs, scriptPath, argsPath], {
            cwd: ctx.cwd,
            stdio: "inherit",
          });
          child.once("error", reject);
          child.once("close", (code, signal) => {
            if (code === 0) resolve();
            else reject(new Error(`External editor exited with ${signal ? `signal ${signal}` : `code ${code}`}`));
          });
        });
      } finally {
        tui.start();
        tui.requestRender(true);
      }
      done();
      return { render: () => [], invalidate: () => {} };
    });
    const script = await readFile(scriptPath, "utf8");
    const argsJson = await readFile(argsPath, "utf8");
    reportDiagnostic("[subagent-workflow] workflow editor completed");
    return { script, argsJson };
  } finally {
    await rm(directory, { recursive: true, force: true }).catch((error: unknown) => {
      reportDiagnostic(`[subagent-workflow] workflow editor cleanup failed: ${errorMessage(error)}`);
    });
  }
}

function parseWorkflowDraft(draft: WorkflowDraft, originalPlan: LaunchPlan, originalArgs: unknown): LaunchPlan {
  const workflow =
    draft.script === originalPlan.workflow.script ? originalPlan.workflow : parseWorkflowScript(draft.script);
  let args: unknown;
  try {
    args = JSON.parse(draft.argsJson);
  } catch (error) {
    throw new Error(`Invalid args.json: ${errorMessage(error)}`);
  }
  // Formatting and object key order changes must not turn inherited args into an explicit override.
  return { workflow, args: isDeepStrictEqual(args, originalArgs) ? originalPlan.args : args };
}

export function buildApprovalSummary(plan: LaunchPlan): string {
  const { meta } = plan.workflow;
  const lines = [`Launch workflow: ${meta.name}`];
  if (meta.description) lines.push(sanitizeTerminalText(meta.description));
  const phases = (meta.phases ?? []).map((phase) => sanitizeTerminalText(phase.title));
  lines.push(phases.length > 0 ? `Phases: ${phases.join(" -> ")}` : "Single phase");
  return lines.join("\n");
}

export async function approveLaunch(
  plan: LaunchPlan,
  ctx: ApprovalContext,
  policy: WorkflowApprovalPolicy,
  readEffectiveArgs: () => unknown,
): Promise<LaunchPlan> {
  // Headless (json/print) and rpc auto-approve; the dialog is TUI-only.
  if (ctx.mode !== "tui") return plan;
  if (policy === "auto") return plan;
  if (policy !== "always-prompt") {
    throw new TypeError(`Invalid workflow approval policy: ${String(policy)}`);
  }
  const originalArgs = readEffectiveArgs();
  let draft: WorkflowDraft = {
    script: plan.workflow.script,
    argsJson: `${JSON.stringify(originalArgs, null, 2)}\n`,
  };
  let summary = buildApprovalSummary(plan);
  const options = [OPEN, ACCEPT, REJECT, REJECT_WITH_REASON];

  for (;;) {
    const choice = await ctx.ui.select(summary, options);
    if (choice === OPEN) {
      try {
        draft = await editWorkflowDraft(draft, ctx);
      } catch (error) {
        reportDiagnostic(`[subagent-workflow] workflow editor failed: ${errorMessage(error)}`);
        ctx.ui.notify("External editor failed. Your workflow draft is unchanged.", "error");
        continue;
      }
      try {
        summary = buildApprovalSummary(parseWorkflowDraft(draft, plan, originalArgs));
      } catch (error) {
        summary = "Launch workflow: workflow.js or args.json needs correction";
        reportDiagnostic(`[subagent-workflow] workflow approval draft parse failed: ${errorMessage(error)}`);
        ctx.ui.notify(`${errorMessage(error)}\nEdit workflow.js or args.json before accepting.`, "error");
      }
      continue;
    }
    if (choice === ACCEPT) {
      try {
        return parseWorkflowDraft(draft, plan, originalArgs);
      } catch (error) {
        reportDiagnostic(`[subagent-workflow] workflow approval draft parse failed: ${errorMessage(error)}`);
        ctx.ui.notify(`${errorMessage(error)}\nEdit workflow.js or args.json before accepting.`, "error");
        continue;
      }
    }
    if (choice === REJECT_WITH_REASON) {
      const reason = await ctx.ui.input(
        "Reason for rejecting workflow",
        "Explain the blocker or what should change before retrying",
      );
      if (reason === undefined) continue;
      const normalizedReason = sanitizeTerminalText(reason.trim());
      if (!normalizedReason) {
        ctx.ui.notify("A rejection reason is required.", "warning");
        continue;
      }
      reportDiagnostic(`[subagent-workflow] workflow launch rejected with reason (${normalizedReason.length} chars)`);
      throw new Error(
        `Workflow "${plan.workflow.meta.name}" launch was rejected by the user.\nReason: ${normalizedReason}\nDo not retry unless the user explicitly asks to run it.`,
      );
    }
    throw new Error(
      `Workflow "${plan.workflow.meta.name}" launch was rejected by the user. Do not retry unless the user explicitly asks to run it.`,
    );
  }
}

/**
 * Workflow launch approval in Pi's TUI.
 *
 * Non-TUI modes (json / print / rpc) auto-approve: a headless caller already made
 * an explicit tool call. In the TUI a dialog previews the workflow (name,
 * description and phases) and offers: Open in editor / Accept / Reject.
 * An inline script is a new script every time, so it never skips the dialog.
 * Reject throws a clear error the model can relay.
 */

import { spawn } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { SettingsManager, type ExtensionUIContext } from "@earendil-works/pi-coding-agent";
import type { WorkflowSettings } from "../../../../config/index.js";
import { reportDiagnostic } from "../diagnostics.js";
import { sanitizeTerminalText } from "../ui/sanitize.js";
import { errorMessage } from "../util.js";
import { parseWorkflowScript, type ParsedWorkflow } from "./parser.js";

/** Run mode; mirrors pi's ExtensionMode, which the package does not re-export. */
export type ExtensionMode = "tui" | "rpc" | "json" | "print";

export interface LaunchPlan {
  readonly workflow: ParsedWorkflow;
  readonly args: unknown;
}

/** The narrow context the approver needs, satisfied by ExtensionContext. */
export interface ApprovalContext {
  mode: ExtensionMode;
  cwd: string;
  isProjectTrusted(): boolean;
  ui: Pick<ExtensionUIContext, "select" | "custom" | "notify">;
}

export type WorkflowApprovalPolicy = WorkflowSettings["workflowApproval"];

const OPEN = "Open in editor";
const ACCEPT = "Accept";
const REJECT = "Reject";

/** Edit a temporary JavaScript file with Pi's configured external editor, releasing the TUI until it exits. */
async function editWorkflowScript(draft: string, ctx: ApprovalContext): Promise<string> {
  const command = SettingsManager.create(ctx.cwd, undefined, {
    projectTrusted: ctx.isProjectTrusted(),
  }).getExternalEditorCommand();
  const directory = await mkdtemp(join(tmpdir(), "pi-workflow-editor-"));
  const scriptPath = join(directory, "workflow.js");
  try {
    await writeFile(scriptPath, draft, "utf8");
    reportDiagnostic(`[subagent-workflow] opening workflow editor: ${command}`);
    await ctx.ui.custom<void>(async (tui, _theme, _keybindings, done) => {
      tui.stop();
      try {
        const [editor, ...editorArgs] = command.split(" ");
        // Match Pi's editor command handling; async spawn also releases console input on Windows.
        await new Promise<void>((resolve, reject) => {
          const child = spawn(editor!, [...editorArgs, scriptPath], {
            cwd: ctx.cwd,
            stdio: "inherit",
            shell: process.platform === "win32",
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
    const saved = await readFile(scriptPath, "utf8");
    reportDiagnostic("[subagent-workflow] workflow editor completed");
    return saved;
  } finally {
    await rm(directory, { recursive: true, force: true }).catch((error: unknown) => {
      reportDiagnostic(`[subagent-workflow] workflow editor cleanup failed: ${errorMessage(error)}`);
    });
  }
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
): Promise<LaunchPlan> {
  // Headless (json/print) and rpc auto-approve; the dialog is TUI-only.
  if (ctx.mode !== "tui") return plan;
  if (policy === "auto") return plan;
  if (policy !== "always-prompt") {
    throw new TypeError(`Invalid workflow approval policy: ${String(policy)}`);
  }
  let draft = plan.workflow.script;
  let summary = buildApprovalSummary(plan);
  const options = [OPEN, ACCEPT, REJECT];

  for (;;) {
    const choice = await ctx.ui.select(summary, options);
    if (choice === OPEN) {
      let saved: string;
      try {
        saved = await editWorkflowScript(draft, ctx);
      } catch (error) {
        reportDiagnostic(`[subagent-workflow] workflow editor failed: ${errorMessage(error)}`);
        ctx.ui.notify("External editor failed. Your workflow draft is unchanged.", "error");
        continue;
      }
      draft = saved;
      try {
        summary = buildApprovalSummary({ ...plan, workflow: parseWorkflowScript(draft) });
      } catch (error) {
        summary = "Launch workflow: edited script needs correction";
        reportDiagnostic(`[subagent-workflow] workflow approval script parse failed: ${errorMessage(error)}`);
        ctx.ui.notify("Workflow script is invalid. Edit it before accepting.", "error");
      }
      continue;
    }
    if (choice === ACCEPT) {
      try {
        const workflow = draft === plan.workflow.script ? plan.workflow : parseWorkflowScript(draft);
        return { ...plan, workflow };
      } catch (error) {
        reportDiagnostic(`[subagent-workflow] workflow approval script parse failed: ${errorMessage(error)}`);
        ctx.ui.notify("Workflow script is invalid. Edit it before accepting.", "error");
        continue;
      }
    }
    throw new Error(
      `Workflow "${plan.workflow.meta.name}" launch was rejected by the user. Do not retry unless the user explicitly asks to run it.`,
    );
  }
}

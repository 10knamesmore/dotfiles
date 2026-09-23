/**
 * Workflow launch approval in Pi's TUI.
 *
 * Non-TUI modes (json / print / rpc) auto-approve: a headless caller already made
 * an explicit tool call. In the TUI a dialog previews the workflow (name,
 * description and phases) and offers: Open in editor / Accept / Reject.
 * An inline script is a new script every time, so it never skips the dialog.
 * Reject throws a clear error the model can relay.
 */

import type { ExtensionUIContext } from "@earendil-works/pi-coding-agent";
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
  ui: Pick<ExtensionUIContext, "select" | "editor" | "notify">;
}

export type WorkflowApprovalPolicy = WorkflowSettings["workflowApproval"];

const OPEN = "Open in editor";
const ACCEPT = "Accept";
const REJECT = "Reject";

export function buildApprovalSummary(plan: LaunchPlan): string {
  const { meta } = plan.workflow;
  const lines = [`Launch workflow: ${meta.name}`];
  if (meta.description) lines.push(sanitizeTerminalText(meta.description));
  const phases = (meta.phases ?? []).map((phase) => sanitizeTerminalText(phase.title));
  lines.push(phases.length > 0 ? `Phases: ${phases.join(" -> ")}` : "Single phase");
  return lines.join("\n");
}

export async function approveLaunch(plan: LaunchPlan, ctx: ApprovalContext, policy: WorkflowApprovalPolicy): Promise<LaunchPlan> {
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
      // pi exposes no direct "spawn $EDITOR" API; its multi-line editor is the sanctioned
      // extension editor surface and offers Ctrl+G to the external $EDITOR.
      const saved = await ctx.ui.editor("Workflow script", draft);
      if (saved === undefined) continue;
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
    throw new Error(`Workflow "${plan.workflow.meta.name}" launch was rejected by the user. Do not retry unless the user explicitly asks to run it.`);
  }
}

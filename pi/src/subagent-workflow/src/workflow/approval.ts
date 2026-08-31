/**
 * Workflow launch approval - the injectable seam behind the workflow tool.
 *
 * Non-TUI modes (json / print / rpc) auto-approve: a headless caller already made
 * an explicit tool call. In the TUI a dialog previews the workflow (name,
 * description and phases) and offers:
 * Run once / Always for this workflow in this project / View script / Open in
 * editor / Deny. "Always" is offered only for saved workflows (an inline script
 * is a new script every time and never skips the dialog). Deny throws a clear
 * error the model can relay.
 */

import type { ExtensionUIContext } from "@earendil-works/pi-coding-agent";
import { scriptOverlayFactory } from "../ui/script-overlay.js";
import { sanitizeTerminalText } from "../ui/sanitize.js";
import type { ConsentStore } from "./consent.js";
import type { ParsedWorkflow } from "./parser.js";
import { hashScript } from "./journal.js";

/** Run mode; mirrors pi's ExtensionMode, which the package does not re-export. */
export type ExtensionMode = "tui" | "rpc" | "json" | "print";

export type LaunchOrigin = "inline" | "saved";

export interface LaunchPlan {
  readonly workflow: ParsedWorkflow;
  readonly args: unknown;
  readonly origin: LaunchOrigin;
}

/** The narrow context the approver needs, satisfied by ExtensionContext. */
export interface ApprovalContext {
  mode: ExtensionMode;
  cwd: string;
  ui: Pick<ExtensionUIContext, "select" | "editor" | "custom" | "notify">;
}

export interface ApprovalDeps {
  consent: ConsentStore;
  policy?: WorkflowApprovalPolicy;
}

export type WorkflowApprovalPolicy = "always-prompt" | "remember" | "auto";

export type ApproveLaunch = (plan: LaunchPlan, ctx: ApprovalContext, deps: ApprovalDeps) => Promise<void>;

const RUN_ONCE = "Run once";
const ALWAYS = "Always for this workflow in this project";
const VIEW = "View script";
const OPEN = "Open in editor";
const DENY = "Deny";

export function buildApprovalSummary(plan: LaunchPlan): string {
  const { meta, script } = plan.workflow;
  const lines = [`Launch workflow: ${meta.name}`];
  if (meta.description) lines.push(sanitizeTerminalText(meta.description));
  const phases = (meta.phases ?? []).map((phase) => sanitizeTerminalText(phase.title));
  lines.push(phases.length > 0 ? `Phases: ${phases.join(" -> ")}` : "Single phase");
  if (plan.origin === "saved") lines.push(`Saved workflow · ${meta.name}`);
  return lines.join("\n");
}

export const approveLaunch: ApproveLaunch = async (plan, ctx, deps) => {
  // Headless (json/print) and rpc auto-approve; the dialog is TUI-only.
  if (ctx.mode !== "tui") return;
  const policy = deps.policy ?? "remember";
  if (policy === "auto") return;
  if (policy !== "remember" && policy !== "always-prompt") {
    throw new TypeError(`Invalid workflow approval policy: ${String(policy)}`);
  }
  const { meta, script } = plan.workflow;
  const scriptHash = hashScript(script);
  if (policy === "remember" && plan.origin === "saved" && deps.consent.isApproved(meta.name, ctx.cwd, scriptHash)) return;

  const summary = buildApprovalSummary(plan);
  const options = policy === "remember" && plan.origin === "saved"
    ? [RUN_ONCE, ALWAYS, VIEW, OPEN, DENY]
    : [RUN_ONCE, VIEW, OPEN, DENY];

  for (;;) {
    const choice = await ctx.ui.select(summary, options);
    if (choice === RUN_ONCE) return;
    if (choice === ALWAYS && policy === "remember" && plan.origin === "saved") {
      deps.consent.record(meta.name, ctx.cwd, scriptHash);
      return;
    }
    if (choice === VIEW) {
      await ctx.ui.custom(scriptOverlayFactory(script), { overlay: true, overlayOptions: { anchor: "center", width: "80%" } });
      continue;
    }
    if (choice === OPEN) {
      // pi exposes no direct "spawn $EDITOR" API; its multi-line editor is the sanctioned
      // extension editor surface and offers Ctrl+G to the external $EDITOR. Edits are discarded.
      await ctx.ui.editor(`Workflow script: ${meta.name}`, script);
      continue;
    }
    throw new Error(`Workflow "${meta.name}" launch was denied by the user. Do not retry unless the user explicitly asks to run it.`);
  }
};

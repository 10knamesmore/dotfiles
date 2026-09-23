import { cpus } from "node:os";
import type { WorkflowSettings } from "../../../../config/index.js";
import { subagentRunner } from "../runner/runner.js";
import type { SubagentStatusWidget } from "../ui/status-widget.js";

/** Apply the configuration loaded with this extension to the runner and widget. */
export function applyWorkflowSettings(settings: Readonly<WorkflowSettings>, widget: SubagentStatusWidget): void {
  const limit = settings.maxConcurrentAgents;
  subagentRunner.setMaxConcurrentAgents(limit === "auto" ? Math.max(1, Math.min(16, cpus().length - 2)) : limit);
  subagentRunner.setAgentTimeoutMinutes(settings.agentTimeoutMinutes);
  widget.configure(settings.showStatusWidget);
}

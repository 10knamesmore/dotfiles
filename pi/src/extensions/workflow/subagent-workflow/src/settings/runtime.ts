import { cpus } from "node:os";
import type { WorkflowSettings } from "@dotfiles/pi-config";
import type { SubagentRunner } from "../runner/runner.js";
import type { SubagentStatusWidget } from "../ui/status-widget.js";

/** Apply the configuration loaded with this extension to the runner and widget. */
export function applyWorkflowSettings(
  settings: Readonly<WorkflowSettings>,
  runner: SubagentRunner,
  widget: SubagentStatusWidget,
): void {
  const limit = settings.maxConcurrentAgents;
  runner.setMaxConcurrentAgents(limit === "auto" ? Math.max(1, Math.min(16, cpus().length - 2)) : limit);
  runner.setAgentTimeoutMinutes(settings.agentTimeoutMinutes);
  widget.configure(settings.showStatusWidget);
}

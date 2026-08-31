import { cpus } from "node:os";
import type { SubagentRunner } from "../runner/runner.js";
import type { SubagentStatusWidget } from "../ui/status-widget.js";
import { WorkflowSettingsStore, type ConcurrentAgentLimit, type WorkflowSettings } from "./workflow-settings.js";

const SETTINGS_STATE_VERSION = "v4";
const SETTINGS_KEY = `__piSubagentWorkflowSettings_${SETTINGS_STATE_VERSION}__`;
const scope = globalThis as unknown as Record<string, WorkflowSettingsStore | undefined>;

/** One process-wide authority, shared by pi's per-cwd extension instances. */
export function globalWorkflowSettings(): WorkflowSettingsStore {
  return scope[SETTINGS_KEY] ??= new WorkflowSettingsStore();
}

export function resolveConcurrentAgentLimit(limit: ConcurrentAgentLimit, cpuCount = cpus().length): number {
  return limit === "auto" ? Math.max(1, Math.min(16, cpuCount - 2)) : limit;
}

/** Apply every live-safe setting to the process runner and this session's widget. */
export function applyLiveWorkflowSettings(
  settings: Readonly<WorkflowSettings>,
  runner: Pick<SubagentRunner, "setMaxConcurrentAgents" | "setAgentTimeoutMinutes">,
  widget: Pick<SubagentStatusWidget, "configure">,
): void {
  runner.setMaxConcurrentAgents(resolveConcurrentAgentLimit(settings.maxConcurrentAgents));
  runner.setAgentTimeoutMinutes(settings.agentTimeoutMinutes);
  widget.configure(settings.showStatusWidget);
}

/** Slash commands and native TUI editor for user-global workflow settings. */

import { getSelectListTheme, getSettingsListTheme, type ExtensionAPI, type ExtensionCommandContext } from "@earendil-works/pi-coding-agent";
import { Container, type SelectItem, SelectList, type SettingItem, SettingsList, truncateToWidth, wrapTextWithAnsi } from "@earendil-works/pi-tui";
import { sanitizeTerminalText } from "../ui/sanitize.js";
import { errorMessage } from "../util.js";
import {
  type NumericSettingId,
  type NumericSettingRule,
  NUMERIC_SETTING_RULES,
  type WorkflowApprovalMode,
  type WorkflowSettings,
  WorkflowSettingsStore,
} from "./workflow-settings.js";

type SettingId = keyof WorkflowSettings;
type Choice = { type: "set"; id: SettingId; value: string } | { type: "action"; id: ActionId };
type ActionId = "clear-approvals" | "reload" | "reset";

interface WorkflowSettingsCommandServices {
  settings: WorkflowSettingsStore;
  clearApprovals: () => void;
}

/** Register both public names. They intentionally share one global settings store. */
export function registerWorkflowSettingsCommands(pi: ExtensionAPI, services: WorkflowSettingsCommandServices): void {
  const handler = (argsText: string, ctx: ExtensionCommandContext) => handleSettingsCommand(argsText, ctx, services);
  pi.registerCommand("workflow-settings", {
    description: "Configure subagent and workflow runtime settings",
    handler,
  });
  pi.registerCommand("subagent-settings", {
    description: "Configure subagent and workflow runtime settings (alias)",
    handler,
  });
}

export async function handleSettingsCommand(
  argsText: string,
  ctx: ExtensionCommandContext,
  services: WorkflowSettingsCommandServices,
): Promise<void> {
  const command = argsText.trim().toLowerCase();
  if (command === "show") {
    ctx.ui.notify(formatSettings(services.settings), "info");
    return;
  }
  if (command === "reload") {
    reloadSettings(ctx, services.settings);
    return;
  }
  if (command === "reset") {
    return resetSettings(ctx, services.settings);
  }
  if (command === "clear-approvals") {
    return clearApprovals(ctx, services);
  }
  if (command !== "") {
    ctx.ui.notify("Usage: /workflow-settings [show|reload|reset|clear-approvals]", "warning");
    return;
  }
  if (ctx.mode !== "tui") {
    ctx.ui.notify("/workflow-settings requires TUI mode; use /workflow-settings show in headless modes", "warning");
    return;
  }
  return openSettingsEditor(ctx, services);
}

export function settingsItems(settings: Readonly<WorkflowSettings>): SettingItem[] {
  return [
    numericItem("maxConcurrentAgents", "Max concurrent agents", String(settings.maxConcurrentAgents), "Process-wide admission limit. 'auto' uses max(1, min(16, CPU count - 2))."),
    selectionItem("workflowApproval", "Workflow approval", settings.workflowApproval, [
      { value: "always-prompt", label: "Always prompt", description: "Ask before every workflow launch." },
      { value: "remember", label: "Remember approvals", description: "Remember approval for an unchanged saved workflow in this project." },
      { value: "auto", label: "Auto-approve", description: "Trust every workflow launch without an approval dialog." },
    ], "Choose how workflow launches are approved."),
    numericItem("agentTimeoutMinutes", "Agent timeout", String(settings.agentTimeoutMinutes), "Wall timeout for each newly admitted model prompt. 0 disables it."),
    selectionItem("showStatusWidget", "Show status widget", String(settings.showStatusWidget), [
      { value: "true", label: "Enabled", description: "Show active workflow and subagent runs below the editor." },
      { value: "false", label: "Disabled", description: "Hide the workflow status widget." },
    ], "Choose whether the workflow status widget is visible."),
    actionItem("reload", "Reload edited file", "Re-read settings.json and apply valid values."),
    actionItem("clear-approvals", "Clear approvals", "Forget all exact-script workflow approvals."),
    actionItem("reset", "Reset all settings", "Restore and save the built-in defaults."),
  ];
}

export function formatSettings(store: WorkflowSettingsStore): string {
  const settings = store.get();
  const warning = store.getWarning();
  return [
    `Workflow settings: ${sanitizeTerminalText(store.path)}`,
    `maxConcurrentAgents: ${settings.maxConcurrentAgents}`,
    `workflowApproval: ${settings.workflowApproval}`,
    `agentTimeoutMinutes: ${settings.agentTimeoutMinutes}`,
    `showStatusWidget: ${settings.showStatusWidget}`,
    ...(warning ? [`Warning: ${sanitizeTerminalText(warning)}`] : []),
  ].join("\n");
}

export function parseNumericSetting(id: NumericSettingId, text: string): number | "auto" {
  const range = NUMERIC_SETTING_RULES[id];
  const normalized = text.trim().toLowerCase();
  if (range.sentinel === "auto" && normalized === "auto") return "auto";
  if (!/^-?\d+$/.test(normalized)) throw new Error(expectedRange(range));
  const value = Number(normalized);
  if (!Number.isSafeInteger(value)) throw new Error(expectedRange(range));
  if (range.sentinel === 0 && value === 0) return value;
  if (value < range.min || value > range.max) throw new Error(expectedRange(range));
  return value;
}

export function confirmAutoApproval(ctx: ExtensionCommandContext): Promise<boolean> {
  return ctx.ui.confirm(
    "Auto-approve all workflows?",
    "Trust every workflow script without showing a launch approval dialog?",
  );
}

async function openSettingsEditor(ctx: ExtensionCommandContext, services: WorkflowSettingsCommandServices): Promise<void> {
  for (;;) {
    const choice = await chooseSetting(ctx, services.settings);
    if (!choice) return;
    if (choice.type === "action") {
      if (choice.id === "reload") reloadSettings(ctx, services.settings);
      if (choice.id === "reset") await resetSettings(ctx, services.settings);
      if (choice.id === "clear-approvals") await clearApprovals(ctx, services);
      continue;
    }
    let value: WorkflowSettings[SettingId];
    if (choice.id === "maxConcurrentAgents" || choice.id === "agentTimeoutMinutes") {
      const range = NUMERIC_SETTING_RULES[choice.id];
      const input = choice.value === "edit"
        ? await ctx.ui.input(`Set ${choice.id}`, expectedRange(range))
        : choice.value;
      if (input === undefined) continue;
      try {
        value = parseNumericSetting(choice.id, input);
      } catch (error) {
        ctx.ui.notify(message(error), "warning");
        continue;
      }
    } else if (choice.id === "showStatusWidget") {
      value = choice.value === "true";
    } else {
      value = choice.value as WorkflowApprovalMode;
      if (value === "auto" && services.settings.get().workflowApproval !== "auto" && !(await confirmAutoApproval(ctx))) continue;
    }
    try {
      services.settings.set(choice.id, value as never);
    } catch (error) {
      ctx.ui.notify(message(error), "error");
    }
  }
}

function chooseSetting(ctx: ExtensionCommandContext, store: WorkflowSettingsStore): Promise<Choice | undefined> {
  return ctx.ui.custom<Choice | undefined>((tui, theme, _keybindings, done) => {
    const container = new Container();
    const warning = store.getWarning();
    container.addChild({
      render: (width: number) => {
        const cap = Math.max(1, width);
        // The warning (e.g. a schema-version reset notice) wraps rather than
        // truncates: its version numbers and "file left unchanged" detail are
        // operationally important.
        const warningLines = warning
          ? wrapTextWithAnsi(sanitizeTerminalText(warning), cap).map((line) => theme.fg("warning", line))
          : [];
        return [
          truncateToWidth(theme.fg("accent", theme.bold("Subagent workflow settings")), cap),
          truncateToWidth(theme.fg("dim", sanitizeTerminalText(store.path)), cap),
          ...warningLines,
          "",
        ];
      },
      invalidate: () => {},
    });
    const list = new SettingsList(
      settingsItems(store.get()),
      10,
      getSettingsListTheme(),
      (id, value) => {
        if (id.startsWith("action:")) done({ type: "action", id: id.slice("action:".length) as ActionId });
        else done({ type: "set", id: id as SettingId, value });
      },
      () => done(undefined),
    );
    container.addChild(list);
    return {
      // The final cap makes the whole composed dialog immune to any child
      // line escaping width discipline; pi-tui kills the process otherwise.
      render: (width) => container.render(width).map((line) => truncateToWidth(line, Math.max(1, width))),
      invalidate: () => container.invalidate(),
      handleInput: (data) => {
        list.handleInput(data);
        tui.requestRender();
      },
    };
  });
}

function numericItem(id: NumericSettingId, label: string, value: string, description: string): SettingItem {
  const range = NUMERIC_SETTING_RULES[id];
  const choices: SelectItem[] = [
    { value: "edit", label: "Enter a value", description: expectedRange(range) },
  ];
  if (range.sentinel === "auto") choices.push({ value: "auto", label: "Automatic", description: "Use the CPU-aware automatic limit." });
  else choices.push({ value: "0", label: "Disabled", description: "Disable this timeout." });
  return selectionItem(id, label, value, choices, description, false);
}

function selectionItem(
  id: SettingId,
  label: string,
  value: string,
  choices: SelectItem[],
  description: string,
  preselectCurrent = true,
): SettingItem {
  return {
    id,
    label,
    currentValue: value,
    description,
    submenu: (currentValue, done) => {
      const list = new SelectList(choices, Math.min(choices.length, 10), getSelectListTheme());
      if (preselectCurrent) {
        const current = choices.findIndex((choice) => choice.value === currentValue);
        if (current >= 0) list.setSelectedIndex(current);
      }
      list.onSelect = (choice) => done(choice.value);
      list.onCancel = () => done();
      return list;
    },
  };
}

function actionItem(id: ActionId, label: string, description: string): SettingItem {
  return { id: `action:${id}`, label, currentValue: "select", values: ["select"], description };
}

function expectedRange(range: NumericSettingRule): string {
  const sentinel = range.sentinel === 0 ? "0 to disable" : "auto";
  return `Enter an integer from ${range.min} to ${range.max}, or ${sentinel}`;
}

function reloadSettings(ctx: ExtensionCommandContext, store: WorkflowSettingsStore): void {
  store.reload();
  const warning = store.getWarning();
  ctx.ui.notify(warning ? sanitizeTerminalText(warning) : `Reloaded workflow settings from ${sanitizeTerminalText(store.path)}`, warning ? "warning" : "info");
}

async function resetSettings(ctx: ExtensionCommandContext, store: WorkflowSettingsStore): Promise<void> {
  const confirmed = await ctx.ui.confirm("Reset workflow settings?", "Restore every workflow setting to its built-in default.");
  if (!confirmed) return;
  try {
    store.reset();
    ctx.ui.notify(`Reset workflow settings at ${sanitizeTerminalText(store.path)}`, "info");
  } catch (error) {
    ctx.ui.notify(message(error), "error");
  }
}

async function clearApprovals(ctx: ExtensionCommandContext, services: WorkflowSettingsCommandServices): Promise<void> {
  const confirmed = await ctx.ui.confirm("Clear remembered approvals?", "Every saved workflow will require approval again unless auto-approval is enabled.");
  if (!confirmed) return;
  try {
    services.clearApprovals();
    ctx.ui.notify("Cleared remembered workflow approvals", "info");
  } catch (error) {
    ctx.ui.notify(`Could not clear approvals: ${message(error)}`, "error");
  }
}

function message(error: unknown): string {
  return sanitizeTerminalText(errorMessage(error));
}

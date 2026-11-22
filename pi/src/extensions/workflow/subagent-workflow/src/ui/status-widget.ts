import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { TUI } from "@earendil-works/pi-tui";
import { reportDiagnostic } from "../diagnostics.js";
import type { SubagentRunner } from "../runner/runner.js";
import { errorMessage } from "../util.js";
import { formatContextUsage, formatFullModel, tokenUsageColumns } from "./format.js";
import { sanitizeTerminalText } from "./sanitize.js";
import { renderStatusRows, type StatusRow } from "./status-layout.js";

const WIDGET_KEY = "subagent-workflow";
const CHILD_PREVIEW = 3;

type LiveRun = ReturnType<SubagentRunner["liveStatuses"]>[number];
type WidgetRow = StatusRow & { runId: string; childId?: string };

function activityCounts(running: number, queued: number): string[] {
  return [running && `${running} running`, queued && `${queued} queued`].filter((part): part is string => !!part);
}

function singleLine(value: string): string {
  return sanitizeTerminalText(value, undefined, true).replace(/[\t\n]/g, " ");
}

function agentRow(runId: string, child: LiveRun["children"][number], prefix: string): WidgetRow {
  return {
    key: child.id,
    kind: prefix ? "child" : "agent",
    runId,
    childId: child.id,
    prefix,
    label: singleLine(child.label),
    status: child.status,
    model: formatFullModel(child.model && singleLine(child.model), child.thinking),
    context: child.context ? formatContextUsage(child.context) : child.status === "pending" ? "queued" : "—",
    ...tokenUsageColumns(child.usage),
  };
}

function workflowRows(run: LiveRun, collapsed: boolean): WidgetRow[] {
  const phaseIndex = run.phases.findIndex(({ title }) => title === run.phase);
  const position = phaseIndex >= 0 && run.phases.length > 1 ? ` (${phaseIndex + 1}/${run.phases.length})` : "";
  const phase = run.phase && singleLine(run.phase) + position;
  const progress = run.total ? `${run.done}/${run.total} done` : "starting";
  const rows: WidgetRow[] = [
    {
      key: run.runId,
      kind: "workflow",
      runId: run.runId,
      prefix: collapsed ? "▸ " : "▾ ",
      label: singleLine(run.label),
      phaseProgress: [phase, progress].filter(Boolean).join(" · "),
      ...tokenUsageColumns(run.usage),
    },
  ];
  if (collapsed) return rows;

  const ordered = [...run.children].sort((a, b) => Number(b.status === "running") - Number(a.status === "running"));
  for (const [i, child] of ordered.slice(0, CHILD_PREVIEW).entries()) {
    rows.push(agentRow(run.runId, child, i === ordered.length - 1 ? "└─ " : "├─ "));
  }
  const hidden = ordered.slice(CHILD_PREVIEW);
  if (hidden.length) {
    const running = hidden.filter((child) => child.status === "running").length;
    rows.push({
      key: `${run.runId}:more`,
      kind: "more",
      runId: run.runId,
      prefix: "└─ ",
      label: `+${activityCounts(running, hidden.length - running).join(" · ")}`,
    });
  }
  return rows;
}

export class SubagentStatusWidget {
  private ctx?: ExtensionContext;
  private tui?: TUI;
  private unsubscribe?: () => void;
  private enabled = true;
  private collapsed = new Set<string>();
  private rows: WidgetRow[] = [];
  private selected?: WidgetRow;
  private summary = "";
  private signature = "";

  constructor(private readonly runner: SubagentRunner) {}

  attach(ctx: ExtensionContext): void {
    if (!ctx.hasUI) return;
    this.ctx = ctx;
    this.unsubscribe = this.runner.subscribeLiveStatus(() => this.refresh());
    this.refresh();
  }

  configure(enabled: boolean): void {
    this.enabled = enabled;
    this.refresh();
  }

  dispose(): void {
    this.unsubscribe?.();
    this.unsubscribe = undefined;
    this.hide();
    this.ctx = undefined;
    this.collapsed.clear();
  }

  /** A downward step past the last row lets the editor move focus to its footer. */
  selectRun(delta: -1 | 1): boolean {
    const index = this.selected ? this.rows.indexOf(this.selected) + delta : delta > 0 ? 0 : this.rows.length - 1;
    if (index >= this.rows.length) return false;
    this.selected = this.rows[index];
    this.tui?.requestRender();
    return !!this.selected;
  }

  hasSelectedRun(): boolean {
    return !!this.selected;
  }

  takeSelectedRun(): { runId: string; childId?: string } | undefined {
    if (!this.selected) return undefined;
    const { runId, childId } = this.selected;
    this.clearSelectedRun();
    return { runId, childId };
  }

  clearSelectedRun(): void {
    if (!this.selected) return;
    this.selected = undefined;
    this.tui?.requestRender();
  }

  private selectedWorkflow(): WidgetRow | undefined {
    return this.rows.find((row) => row.kind === "workflow" && row.key === this.selected?.runId);
  }

  setExpanded(expanded: boolean): boolean {
    const workflow = this.selectedWorkflow();
    if (!workflow) return false;
    if (expanded) this.collapsed.delete(workflow.runId);
    else this.collapsed.add(workflow.runId);
    this.refresh();
    return true;
  }

  private hide(): void {
    this.rows = [];
    this.selected = undefined;
    this.tui = undefined;
    if (!this.signature) return;
    this.signature = "";
    this.summary = "";
    this.ctx?.ui.setWidget(WIDGET_KEY, undefined);
    this.ctx?.ui.setStatus(WIDGET_KEY, undefined);
  }

  private refresh(): void {
    try {
      this.update();
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] status widget failed: ${errorMessage(error)}`);
    }
  }

  private update(): void {
    if (!this.ctx) return;
    const runs = this.enabled ? this.runner.liveStatuses() : [];
    for (const id of this.collapsed) if (!runs.some((run) => run.runId === id)) this.collapsed.delete(id);
    if (!runs.length) {
      this.hide();
      return;
    }

    const workflows = runs.filter((run) => run.kind === "workflow");
    this.rows = [
      ...runs
        .filter((run) => run.kind === "subagent")
        .flatMap((run) => run.children.map((child) => agentRow(run.runId, child, ""))),
      ...workflows.flatMap((run) => workflowRows(run, this.collapsed.has(run.runId))),
    ];
    const previous = this.selected;
    if (previous) {
      this.selected =
        this.rows.find((row) => row.key === previous.key) ?? this.rows.find((row) => row.key === previous.runId);
    }
    const children = runs.flatMap((run) => run.children);
    const running = children.filter((child) => child.status === "running").length;
    const parts = activityCounts(running, children.length - running);
    if (workflows.length) parts.unshift(`${workflows.length} workflow${workflows.length > 1 ? "s" : ""}`);
    const summary = parts.join(" · ");
    const signature = JSON.stringify([summary, this.rows]);
    if (this.signature === signature) return;
    const register = !this.signature;
    if (this.summary !== summary) this.ctx.ui.setStatus(WIDGET_KEY, summary);
    this.summary = summary;
    if (register) {
      this.ctx.ui.setWidget(
        WIDGET_KEY,
        (tui, theme) => {
          this.tui = tui;
          return {
            render: (width) =>
              renderStatusRows(
                this.rows,
                this.selected?.key,
                theme,
                width,
                Math.max(1, Math.floor(tui.terminal.rows / 4)),
              ),
            invalidate() {},
          };
        },
        { placement: "belowEditor" },
      );
    } else {
      this.tui?.requestRender();
    }
    this.signature = signature;
  }
}

import { homedir, hostname, userInfo } from "node:os";
import type {
  ExtensionContext,
  ReadonlyFooterDataProvider,
} from "@earendil-works/pi-coding-agent";
import type { Component } from "@earendil-works/pi-tui";
import {
  fitByDropping,
  formatDuration,
  formatFooterCwd,
  formatTokens,
  sanitizeFooterText,
} from "./format.js";
import type {
  GitDiffStat,
  GitFileStatus,
  GitStatusSnapshot,
} from "./git-status.js";
import { GitStatusCache } from "./git-status.js";
import {
  type RecentHitRateTracker,
  type PromptRunSnapshot,
  type PromptRunTracker,
  type SessionUsageTotals,
  type ToolUsageSnapshot,
  ToolUsageTracker,
  TurnTracker,
  type UsageCounter,
} from "./metrics.js";
import { palette, separator } from "./palette.js";

interface ClaudeFooterComponentOptions {
  /** Returns the latest event context instead of a session-start snapshot. */
  getContext: () => ExtensionContext;

  /** Pi-owned extension status provider. */
  footerData: ReadonlyFooterDataProvider;

  /** Event-driven git snapshot cache owned by this component. */
  git: GitStatusCache;

  /** Tool execution counts observed during the active extension session. */
  tools: ToolUsageTracker;

  /** Turn and agent-run counts observed during the active extension session. */
  turns: TurnTracker;

  /** Parent-model usage and wall time for the current or last accepted prompt. */
  promptRun: PromptRunTracker;

  /** Session-total usage grown by events, pre-filled from persisted entries. */
  usage: UsageCounter;

  /** Cache hit rate over the most recent completed turns. */
  hitRate: RecentHitRateTracker;

  /** Requests an event-driven TUI repaint. */
  requestRender: () => void;
}

interface FirstLineRenderOptions {
  /** Terminal width available to the first footer line. */
  width: number;

  /** Sanitized current username. */
  username: string;

  /** Sanitized short hostname. */
  host: string;

  /** Current Pi extension context. */
  ctx: ExtensionContext;

  /** Current cached Git snapshot. */
  git: GitStatusSnapshot;
}

interface SecondLineRenderOptions {
  /** Terminal width available to the second footer line. */
  width: number;

  /** Current Pi extension context. */
  ctx: ExtensionContext;

  /** Accumulated session usage. */
  usage: SessionUsageTotals;

  /** Recent cache hit rate, already formatted for display. */
  hitRate: string;
}

interface ThirdLineRenderOptions {
  /** Terminal width available to the third footer line. */
  width: number;

  /** Pi-owned extension status provider. */
  footerData: ReadonlyFooterDataProvider;

  /** Tool execution tracker. */
  tools: ToolUsageTracker;

  /** Turn and agent-run tracker. */
  turns: TurnTracker;

  /** Current or last prompt, absent before the first run in this runtime. */
  promptRun: PromptRunSnapshot | undefined;
}

function currentUsername(): string {
  try {
    return sanitizeFooterText(userInfo().username);
  } catch {
    return sanitizeFooterText(process.env.USER ?? "user");
  }
}

function shortHostname(): string {
  const fullHostname = hostname();
  const shortName = fullHostname.split(".")[0] ?? fullHostname;

  return sanitizeFooterText(shortName);
}

function formatGitOperation(snapshot: GitStatusSnapshot): string {
  if (snapshot.kind === "unavailable") {
    return "";
  }

  if (snapshot.operation === undefined) {
    return "";
  }

  const operation = snapshot.operation;
  let progress = "";

  if (operation.step !== undefined && operation.total !== undefined) {
    progress = ` ${operation.step}/${operation.total}`;
  }

  return palette.yellow(`(${operation.label}${progress})`);
}

/** Prefix a git segment with the pipe that separates it from the previous one. */
function gitSegment(segment: string): string {
  return segment === "" ? "" : `${palette.overlay2("|")} ${segment}`;
}

/** Upstream tracking state shown after the branch name. */
function formatGitTracking(snapshot: GitStatusSnapshot): string {
  if (snapshot.kind === "unavailable" || snapshot.detached) {
    return "";
  }

  if (snapshot.upstream === undefined) {
    return palette.overlay2("no upstream");
  }

  const parts = [palette.overlay2("→"), palette.sky(snapshot.upstream)];

  if (snapshot.ahead === undefined || snapshot.behind === undefined) {
    parts.push(palette.yellow("↑? ↓?"));
  } else if (snapshot.ahead === 0 && snapshot.behind === 0) {
    parts.push(palette.green("synced"));
  } else {
    if (snapshot.ahead > 0) parts.push(palette.peach(`↑${snapshot.ahead}`));
    if (snapshot.behind > 0) parts.push(palette.peach(`↓${snapshot.behind}`));
  }

  return parts.join(" ");
}

/** One staged/unstaged group: file count plus text line totals. */
function formatGitDiffStat(
  label: string,
  stat: GitDiffStat,
  labelColor: (text: string) => string,
): string {
  if (stat.files === 0) {
    return "";
  }

  const parts = [labelColor(`${label} ${stat.files}`)];

  if (stat.added > 0) parts.push(palette.green(`+${stat.added}`));
  if (stat.deleted > 0) parts.push(palette.red(`−${stat.deleted}`));

  return parts.join(" ");
}

/** Conflicts stay separate from the staged/unstaged diff totals. */
function formatGitConflicts(files: GitFileStatus): string {
  return files.conflicted > 0
    ? palette.red(`conflicts ${files.conflicted}`)
    : "";
}

function formatGitUntracked(files: GitFileStatus): string {
  return files.untracked > 0
    ? palette.overlay2(`untracked ${files.untracked}`)
    : "";
}

function formatGitStash(files: GitFileStatus): string {
  return files.stashed > 0 ? palette.mauve(`stash ${files.stashed}`) : "";
}

/** True when no tracked, untracked, or unmerged change is present. */
function gitIsClean(files: GitFileStatus): boolean {
  return (
    files.staged.files === 0 &&
    files.unstaged.files === 0 &&
    files.untracked === 0 &&
    files.conflicted === 0
  );
}

function renderFirstLine(options: FirstLineRenderOptions): string {
  const { width, username, host, ctx, git } = options;
  const identity = [
    palette.overlay2("["),
    palette.peach(username),
    palette.overlay2("@"),
    palette.red(host),
    palette.overlay2("]"),
  ].join("");
  const cwd = palette.peach(
    formatFooterCwd(ctx.sessionManager.getCwd(), homedir()),
  );

  const parts = [identity, cwd];

  if (git.kind === "repository") {
    parts.push(
      gitSegment(palette.yellow(git.branch)),
      formatGitTracking(git),
      formatGitOperation(git),
      formatGitConflicts(git.files),
      gitSegment(formatGitDiffStat("staged", git.files.staged, palette.green)),
      gitSegment(
        formatGitDiffStat("unstaged", git.files.unstaged, palette.sky),
      ),
      gitSegment(formatGitUntracked(git.files)),
      gitSegment(formatGitStash(git.files)),
      gitSegment(gitIsClean(git.files) ? palette.green("clean") : ""),
    );
  }

  // Drop order: stats, then the static identity, then upstream state; cwd,
  // branch, and operation/conflict state always remain.
  const dropOrder = [10, 9, 8, 7, 6, 0, 3];

  return fitByDropping(parts, dropOrder, width);
}

function formatContext(ctx: ExtensionContext): string {
  const usage = ctx.getContextUsage();
  const contextWindow = usage?.contextWindow ?? ctx.model?.contextWindow ?? 0;

  if (usage === undefined || usage.tokens === null || usage.percent === null) {
    let contextWindowText = "?";

    if (contextWindow > 0) {
      contextWindowText = formatTokens(contextWindow);
    }

    return palette.overlay2(`ctx ?/${contextWindowText}`);
  }

  const percent = Math.round(usage.percent);
  const usedTokens = formatTokens(usage.tokens);
  const contextWindowTokens = formatTokens(contextWindow);
  const body = `ctx ${usedTokens}/${contextWindowTokens} ${percent}%`;

  if (percent >= 70) {
    return `🥵 ${palette.red(body)}`;
  }

  if (percent >= 50 || usage.tokens >= 250_000) {
    return `😢 ${palette.yellow(body)}`;
  }

  return `😎 ${palette.green(body)}`;
}

function formatToolUsage(snapshot: ToolUsageSnapshot): string {
  if (snapshot.total === 0) {
    return "";
  }

  const top = snapshot.top
    .map(({ name, count }) => {
      const cleanName = sanitizeFooterText(name);
      return `${cleanName}:${count}`;
    })
    .join(" ");
  const parts = [
    palette.overlay2("tools"),
    palette.peach(String(snapshot.total)),
  ];

  if (top) {
    parts.push(palette.sky(top));
  }

  if (snapshot.errors > 0) {
    parts.push(palette.red(`✘${snapshot.errors}`));
  }

  return parts.join(" ");
}

function formatTurns(snapshot: { turns: number; agents: number; recentAgentMilliseconds?: number }): string {
  const turnSummary = [
    palette.overlay2("turns"),
    palette.lavender(String(snapshot.turns)),
  ].join(" ");

  if (snapshot.agents === 0) {
    return turnSummary;
  }

  const agentSummary = [
    palette.overlay2("agents"),
    palette.lavender(String(snapshot.agents)),
    ...(snapshot.recentAgentMilliseconds === undefined
      ? []
      : [palette.overlay2(`(last ${formatDuration(snapshot.recentAgentMilliseconds)})`)]),
  ].join(" ");

  return `${turnSummary}${separator}${agentSummary}`;
}

function formatRecentHitRate(hitRate: number | undefined): string {
  if (hitRate === undefined) {
    return "";
  }

  const percent = Math.round(hitRate * 10) / 10;
  const label = `${percent.toFixed(1)}%`;

  if (percent >= 90) {
    return palette.green(label);
  }

  if (percent >= 80) {
    return palette.yellow(label);
  }

  return palette.red(label);
}

function renderSecondLine(options: SecondLineRenderOptions): string {
  const { width, ctx, usage, hitRate } = options;
  const totalsParts = [
    palette.peach(`in:${formatTokens(usage.input + usage.cacheRead, 2)}`),
    palette.sky(`out:${formatTokens(usage.output, 2)}`),
  ];

  if (usage.cacheRead > 0) {
    totalsParts.push(
      palette.green(`cached:${formatTokens(usage.cacheRead, 2)}`),
    );
  }

  if (hitRate) {
    totalsParts.push(hitRate);
  }

  const totals = totalsParts.join(" ");
  const cost = usage.parentCostUsd > 0
    ? palette.peach(`$${usage.parentCostUsd.toFixed(3)}`)
    : "";

  return fitByDropping([formatContext(ctx), totals, cost], [1], width, separator);
}

function statusPriority(key: string): number {
  if (key === "subagent-workflow") {
    return 1;
  }

  return 2;
}

function formatExtensionStatuses(
  footerData: ReadonlyFooterDataProvider,
): string {
  const statuses = [...footerData.getExtensionStatuses().entries()]
    .filter(([key]) => key !== "todo" && key !== "subagent-workflow:usage")
    .sort(([leftKey], [rightKey]) => {
      const priorityDifference =
        statusPriority(leftKey) - statusPriority(rightKey);

      if (priorityDifference !== 0) {
        return priorityDifference;
      }

      return leftKey.localeCompare(rightKey);
    })
    .map(([key, value]) => {
      const clean = sanitizeFooterText(value);

      if (!clean) {
        return "";
      }

      if (key === "subagent-workflow") {
        return `agents ${clean}`;
      }

      return clean;
    })
    .filter(Boolean);

  if (statuses.length > 0) {
    return palette.lavender(statuses.join(" · "));
  }

  return "";
}

function formatPromptRun(run: PromptRunSnapshot | undefined): string {
  if (!run) return "";
  const label = run.state === "running" ? "run" : "last";
  const duration = palette.lavender(formatDuration(run.elapsedMilliseconds));
  const input = palette.peach(`in:${run.usage ? formatTokens(run.usage.input, 2) : "—"}`);
  const output = palette.sky(`out:${run.usage ? formatTokens(run.usage.output, 2) : "—"}`);
  const cost = run.usage ? palette.peach(`~$${run.usage.costUsd.toFixed(3)}`) : palette.overlay2("—");
  return `${palette.overlay2(label)} ${duration} · ${input} ${output} · ${cost}`;
}

function renderThirdLine(options: ThirdLineRenderOptions): string {
  const {
    width,
    footerData,
    tools,
    turns,
    promptRun,
  } = options;
  const toolUsage = formatToolUsage(tools.snapshot(4));
  const turnCount = formatTurns(turns.snapshot());
  const statuses = formatExtensionStatuses(footerData);
  const thirdParts = [formatPromptRun(promptRun), toolUsage, turnCount, statuses];
  const dropOrder = [1, 2, 3];

  return fitByDropping(thirdParts, dropOrder, width, separator);
}

/**
 * Three-row Catppuccin footer backed only by in-memory Pi and git snapshots
 * during render.
 */
export class ClaudeFooterComponent implements Component {
  private readonly username = currentUsername();
  private readonly host = shortHostname();
  private disposed = false;

  public constructor(private readonly options: ClaudeFooterComponentOptions) {}

  /**
   * Rebind background git observation when Pi replaces the active session
   * context.
   */
  public updateContext(ctx: ExtensionContext): void {
    this.options.git.setCwd(ctx.sessionManager.getCwd());
  }

  /**
   * Refresh repository state after Pi reports a lifecycle or tool-completion
   * event.
   */
  public refreshGit(): void {
    this.options.git.refreshForEvent();
  }

  /**
   * Request a repaint for provider/model lifecycle events without running an
   * idle timer.
   */
  public requestRender(): void {
    if (!this.disposed) {
      this.options.requestRender();
    }
  }

  public render(width: number): string[] {
    if (width <= 0) {
      return ["", "", ""];
    }

    const ctx = this.options.getContext();
    const git = this.options.git.snapshot();
    const usage = this.options.usage.snapshot();

    return [
      renderFirstLine({
        width,
        username: this.username,
        host: this.host,
        ctx,
        git,
      }),
      renderSecondLine({
        width,
        ctx,
        usage,
        hitRate: formatRecentHitRate(this.options.hitRate.hitRatePercent()),
      }),
      renderThirdLine({
        width,
        footerData: this.options.footerData,
        tools: this.options.tools,
        turns: this.options.turns,
        promptRun: this.options.promptRun.snapshot(),
      }),
    ];
  }

  public invalidate(): void {
    // The footer reads current state from its sources during every render.
  }

  /** Stop git watchers, debounce timers, and any running git process. */
  public dispose(): void {
    if (this.disposed) {
      return;
    }

    this.disposed = true;
    this.options.git.dispose();
  }
}

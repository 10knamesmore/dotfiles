import { homedir, hostname, userInfo } from "node:os";
import type {
  ExtensionContext,
  ReadonlyFooterDataProvider,
} from "@earendil-works/pi-coding-agent";
import type { Component } from "@earendil-works/pi-tui";
import {
  fitByDropping,
  formatFooterCwd,
  formatTokens,
  sanitizeFooterText,
} from "./format.js";
import type { GitFileStatus, GitStatusSnapshot } from "./git-status.js";
import { GitStatusCache } from "./git-status.js";
import {
  type RecentHitRateTracker,
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

function formatGitFiles(files: GitFileStatus): string {
  const parts: string[] = [];

  if (files.conflicted > 0) {
    parts.push(palette.red(`👎${files.conflicted}`));
  }

  if (files.stashed > 0) {
    parts.push(palette.mauve(`&${files.stashed}`));
  }

  if (files.deleted > 0) {
    parts.push(palette.red(`✘${files.deleted}`));
  }

  if (files.renamed > 0) {
    parts.push(palette.overlay2(`»${files.renamed}`));
  }

  if (files.modified > 0) {
    parts.push(palette.sky(`!${files.modified}`));
  }

  if (files.staged > 0) {
    parts.push(palette.green(`${files.staged}`));
  }

  if (files.untracked > 0) {
    parts.push(palette.overlay2(`?${files.untracked}`));
  }

  if (files.ahead > 0 && files.behind > 0) {
    parts.push(palette.peach(`⇕⇡${files.ahead}⇣${files.behind}`));
  } else if (files.ahead > 0) {
    parts.push(palette.peach(`⇡${files.ahead}`));
  } else if (files.behind > 0) {
    parts.push(palette.peach(`⇣${files.behind}`));
  }

  return parts.join(" ");
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

  let branch = "";
  let files = "";

  if (git.kind === "repository") {
    branch = palette.yellow(git.branch);
    files = formatGitFiles(git.files);
  }

  const operation = formatGitOperation(git);

  return fitByDropping(
    [identity, cwd, branch, operation, files],
    [4, 3, 2, 0],
    width,
  );
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

function formatTurns(snapshot: { turns: number; agents: number }): string {
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
  ].join(" ");

  return `${turnSummary}${separator}${agentSummary}`;
}

function formatRecentHitRate(hitRate: number | undefined): string {
  if (hitRate === undefined) {
    return "";
  }

  if (hitRate >= 70) {
    return palette.green(`${hitRate}%`);
  }

  if (hitRate >= 40) {
    return palette.yellow(`${hitRate}%`);
  }

  return palette.red(`${hitRate}%`);
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

  return fitByDropping([formatContext(ctx), totals], [1], width, separator);
}

function statusPriority(key: string): number {
  if (key === "todo") {
    return 0;
  }

  if (key === "subagent-workflow") {
    return 1;
  }

  if (key === "subagent-workflow:usage") {
    return 3;
  }

  return 2;
}

function formatExtensionStatuses(
  footerData: ReadonlyFooterDataProvider,
): string {
  const statuses = [...footerData.getExtensionStatuses().entries()]
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

function renderThirdLine(options: ThirdLineRenderOptions): string {
  const {
    width,
    footerData,
    tools,
    turns,
  } = options;
  const toolUsage = formatToolUsage(tools.snapshot(3));
  const turnCount = formatTurns(turns.snapshot());
  const statuses = formatExtensionStatuses(footerData);
  const thirdParts = [toolUsage, turnCount, statuses];
  const dropOrder = [0, 1];

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

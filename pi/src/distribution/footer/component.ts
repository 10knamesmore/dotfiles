import { homedir, hostname, userInfo } from "node:os";
import type {
  ExtensionContext,
  ReadonlyFooterDataProvider,
} from "@earendil-works/pi-coding-agent";
import {
  truncateToWidth,
  visibleWidth,
  type Component,
} from "@earendil-works/pi-tui";
import {
  fitByDropping,
  fitRequiredPair,
  formatDuration,
  formatFooterCwd,
  formatTokens,
  formatTokensPerSecond,
  sanitizeFooterText,
} from "./format.js";
import type { GitFileStatus, GitStatusSnapshot } from "./git-status.js";
import { GitStatusCache } from "./git-status.js";
import {
  type ActiveSessionDurationTracker,
  type ModelDurationTracker,
  type RecentHitRateTracker,
  RecentTokensPerSecondTracker,
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

  /**
   * Wall duration of model calls observed during the active extension session.
   */
  modelDuration: ModelDurationTracker;

  /** Tool execution counts observed during the active extension session. */
  tools: ToolUsageTracker;

  /** Turn and agent-run counts observed during the active extension session. */
  turns: TurnTracker;

  /** Session-total usage grown by events, pre-filled from persisted entries. */
  usage: UsageCounter;

  /** Cache hit rate over the most recent completed turns. */
  hitRate: RecentHitRateTracker;

  /** Generated-token throughput over the most recent completed turns. */
  throughput: RecentTokensPerSecondTracker;

  /**
   * Monotonic elapsed time observed since the active session's latest
   * `session_start`.
   */
  sessionDuration: ActiveSessionDurationTracker;

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

  /** Model wall-duration tracker. */
  modelDuration: ModelDurationTracker;

  /** Tool execution tracker. */
  tools: ToolUsageTracker;

  /** Turn and agent-run tracker. */
  turns: TurnTracker;

  /** Recent generated-token throughput tracker. */
  throughput: RecentTokensPerSecondTracker;

  /** Active session wall-duration tracker. */
  sessionDuration: ActiveSessionDurationTracker;
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

function joined(parts: readonly string[]): string {
  const nonEmptyParts = parts.filter(Boolean);
  return nonEmptyParts.join(separator);
}

function modelVariants(ctx: ExtensionContext): {
  full: string;
  withoutThinking: string;
  modelOnly: string;
} {
  if (!ctx.model) {
    const noModel = palette.sky("no-model");
    return {
      full: noModel,
      withoutThinking: noModel,
      modelOnly: noModel,
    };
  }

  const modelName = sanitizeFooterText(ctx.model.name || ctx.model.id);
  const provider = sanitizeFooterText(ctx.model.provider);
  const modelOnly = palette.sky(modelName);
  const withoutThinking = `${palette.overlay2(`${provider}/`)}${modelOnly}`;
  let thinking = "";

  if (ctx.model.reasoning) {
    thinking = ` ${palette.mauve(`(${ctx.thinkingLevel ?? "off"})`)}`;
  }

  return {
    full: `${withoutThinking}${thinking}`,
    withoutThinking,
    modelOnly,
  };
}

function fitModelContextLine(
  ctx: ExtensionContext,
  totals: string,
  width: number,
): string {
  const model = modelVariants(ctx);
  const context = formatContext(ctx);
  let line = joined([model.full, context, totals]);

  if (visibleWidth(line) <= width) {
    return line;
  }

  line = joined([model.full, context]);

  if (visibleWidth(line) <= width) {
    return line;
  }

  line = joined([model.withoutThinking, context]);

  if (visibleWidth(line) <= width) {
    return line;
  }

  line = joined([model.modelOnly, context]);

  if (visibleWidth(line) <= width) {
    return line;
  }

  return fitRequiredPair(model.modelOnly, context, width);
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

  return truncateToWidth(
    fitModelContextLine(ctx, totals, width),
    width,
    palette.overlay2("…"),
  );
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
    modelDuration,
    tools,
    turns,
    throughput,
    sessionDuration,
  } = options;
  const sessionTime = formatDuration(sessionDuration.elapsedMilliseconds());
  const modelTime = formatDuration(modelDuration.elapsedMilliseconds());
  const tokensPerSecond = throughput.tokensPerSecond();

  // `session 1m1s(api 2m2s) 13.3 toks/s` as one cohesive timing block:
  // API wall duration in parentheses right after the session duration (no
  // space), then throughput; the whole block drops together when narrow.
  const apiDurationText =
    `${palette.overlay2("(api")} ${palette.sky(modelTime)}${palette.overlay2(")")}`;
  const throughputText =
    tokensPerSecond === undefined
      ? ""
      : ` ${palette.sky(formatTokensPerSecond(tokensPerSecond))} ${palette.overlay2("toks/s")}`;
  const timingText =
    `${palette.overlay2("session")} ${palette.lavender(sessionTime)}` +
    `${apiDurationText}${throughputText}`;

  const toolSnapshot = tools.snapshot(3);
  const toolUsage = formatToolUsage(toolSnapshot);
  const turnSnapshot = turns.snapshot();
  const turnCount = formatTurns(turnSnapshot);
  const statuses = formatExtensionStatuses(footerData);
  const thirdParts = [timingText, toolUsage, turnCount, statuses];
  const dropOrder = [1, 2, 0];

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
        modelDuration: this.options.modelDuration,
        tools: this.options.tools,
        turns: this.options.turns,
        throughput: this.options.throughput,
        sessionDuration: this.options.sessionDuration,
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

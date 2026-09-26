import { homedir, hostname, userInfo } from "node:os";
import type { ExtensionContext, ReadonlyFooterDataProvider } from "@earendil-works/pi-coding-agent";
import { visibleWidth, type Component } from "@earendil-works/pi-tui";
import { fitByDropping, formatDuration, formatFooterCwd, formatTokens, sanitizeFooterText } from "./format.js";
import type { GitDiffStat, GitFileStatus, GitStatusSnapshot } from "./git-status.js";
import { GitStatusCache } from "./git-status.js";
import type { RecentHitRateTracker } from "./metrics.js";
import type {
  PromptRunSnapshot,
  SessionMetrics,
  SessionMetricsSnapshot,
  ToolUsageSnapshot,
} from "./session-metrics.js";
import { palette, separator } from "./palette.js";

interface ClaudeFooterComponentOptions {
  /** Returns the latest event context instead of a session-start snapshot. */
  getContext: () => ExtensionContext;

  /** Pi-owned extension status provider. */
  footerData: ReadonlyFooterDataProvider;

  /** Event-driven git snapshot cache owned by this component. */
  git: GitStatusCache;

  /** Transcript-derived session totals and the current or last run. */
  metrics: SessionMetrics;

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
  width: number;
  metrics: SessionMetricsSnapshot;
}

interface ThirdLineRenderOptions {
  width: number;
  footerData: ReadonlyFooterDataProvider;
  promptRun: PromptRunSnapshot | undefined;
  hitRate: string;
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
function formatGitDiffStat(label: string, stat: GitDiffStat, labelColor: (text: string) => string): string {
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
  return files.conflicted > 0 ? palette.red(`conflicts ${files.conflicted}`) : "";
}

function formatGitUntracked(files: GitFileStatus): string {
  return files.untracked > 0 ? palette.overlay2(`untracked ${files.untracked}`) : "";
}

function formatGitStash(files: GitFileStatus): string {
  return files.stashed > 0 ? palette.mauve(`stash ${files.stashed}`) : "";
}

/** True when no tracked, untracked, or unmerged change is present. */
function gitIsClean(files: GitFileStatus): boolean {
  return files.staged.files === 0 && files.unstaged.files === 0 && files.untracked === 0 && files.conflicted === 0;
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
  const cwd = palette.peach(formatFooterCwd(ctx.sessionManager.getCwd(), homedir()));

  const parts = [identity, cwd];

  if (git.kind === "repository") {
    parts.push(
      gitSegment(palette.yellow(git.branch)),
      formatGitTracking(git),
      formatGitOperation(git),
      formatGitConflicts(git.files),
      gitSegment(formatGitDiffStat("staged", git.files.staged, palette.green)),
      gitSegment(formatGitDiffStat("unstaged", git.files.unstaged, palette.sky)),
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

function formatToolUsage(snapshot: ToolUsageSnapshot, includeDetails: boolean): string {
  if (snapshot.total === 0) {
    return "";
  }

  const top = snapshot.top
    .map(({ name, count }) => {
      const cleanName = sanitizeFooterText(name);
      return `${cleanName}:${count}`;
    })
    .join(" ");
  const parts = [palette.overlay2("tools"), palette.peach(String(snapshot.total))];

  if (includeDetails && top) {
    parts.push(palette.sky(top));
  }

  if (snapshot.errors > 0) {
    parts.push(palette.red(`✘${snapshot.errors}`));
  }

  return parts.join(" ");
}

function formatRecentHitRate(hitRate: number | undefined): string {
  if (hitRate === undefined) {
    return "";
  }

  const percent = Math.round(hitRate * 10) / 10;
  const label = `${percent.toFixed(1)}%`;

  const colored = percent >= 90 ? palette.green(label) : percent >= 80 ? palette.yellow(label) : palette.red(label);
  return `${palette.overlay2("hit:")}${colored}`;
}

function renderSecondLine(options: SecondLineRenderOptions): string {
  const { width, metrics } = options;
  const { usage, tools } = metrics;
  const counts = [
    `${palette.overlay2("runs")} ${palette.lavender(String(metrics.runs))}`,
    `${palette.overlay2("turns")} ${palette.lavender(String(metrics.turns))}`,
  ].join(" · ");
  const totals = [
    palette.peach(`in:${formatTokens(usage.input + usage.cacheRead, 2)}`),
    palette.sky(`out:${formatTokens(usage.output, 2)}`),
    palette.peach(`$${usage.parentCostUsd.toFixed(3)}`),
  ].join(" ");
  const cached = usage.cacheRead > 0 ? palette.green(`cached:${formatTokens(usage.cacheRead, 2)}`) : "";
  const parts = [counts, totals, cached, formatToolUsage(tools, true)];
  if (visibleWidth(parts.filter(Boolean).join(separator)) > width) {
    parts[3] = formatToolUsage(tools, false);
  }
  return fitByDropping(parts, [2, 3], width, separator);
}

function formatExtensionStatuses(footerData: ReadonlyFooterDataProvider): string {
  const statuses = [...footerData.getExtensionStatuses().entries()]
    .filter(([key]) => key !== "todo" && key !== "subagent-workflow" && key !== "subagent-workflow:usage")
    .sort(([leftKey], [rightKey]) => leftKey.localeCompare(rightKey))
    .map(([, value]) => sanitizeFooterText(value))
    .filter(Boolean);

  if (statuses.length > 0) {
    return palette.lavender(statuses.join(" · "));
  }

  return "";
}

function formatPromptRun(run: PromptRunSnapshot | undefined): string[] {
  const label = run?.state === "running" ? "run" : "last";
  const duration = palette.lavender(formatDuration(run?.elapsedMilliseconds ?? 0));
  const turns = `${palette.overlay2("turns")} ${palette.lavender(String(run?.turns ?? 0))}`;
  const totals = [
    palette.peach(`in:${formatTokens(run?.usage?.input ?? 0, 2)}`),
    palette.sky(`out:${formatTokens(run?.usage?.output ?? 0, 2)}`),
    palette.peach(`$${(run?.usage?.costUsd ?? 0).toFixed(3)}`),
  ].join(" ");
  return [`${palette.overlay2(label)} ${duration}`, turns, totals];
}

function renderThirdLine(options: ThirdLineRenderOptions): string {
  const { width, footerData, promptRun, hitRate } = options;
  const statuses = formatExtensionStatuses(footerData);
  return fitByDropping([...formatPromptRun(promptRun), hitRate, statuses], [4, 3], width, separator);
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
    const metrics = this.options.metrics.snapshot(ctx);

    return [
      renderFirstLine({
        width,
        username: this.username,
        host: this.host,
        ctx,
        git,
      }),
      renderSecondLine({ width, metrics }),
      renderThirdLine({
        width,
        footerData: this.options.footerData,
        promptRun: metrics.promptRun,
        hitRate: formatRecentHitRate(this.options.hitRate.hitRatePercent()),
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

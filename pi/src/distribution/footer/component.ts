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
  sanitizeFooterText,
} from "./format.js";
import type { GitFileStatus, GitStatusSnapshot } from "./git-status.js";
import { GitStatusCache } from "./git-status.js";
import {
  collectSessionUsage,
  type ActiveSessionDurationTracker,
  type ModelDurationTracker,
  type RateLimitSnapshot,
  type RateLimitTracker,
} from "./metrics.js";
import { palette, separator } from "./palette.js";

interface ClaudeFooterComponentOptions {
  /** Returns the latest event context instead of a session-start snapshot. */
  getContext: () => ExtensionContext;

  /** Pi-owned extension status provider. */
  footerData: ReadonlyFooterDataProvider;

  /** Event-driven git snapshot cache owned by this component. */
  git: GitStatusCache;

  /** Latest provider-backed rate-limit snapshot. */
  rateLimits: RateLimitTracker;

  /** Wall duration of model calls observed during the active extension session. */
  modelDuration: ModelDurationTracker;

  /** Monotonic elapsed time observed since the active session's latest `session_start`. */
  sessionDuration: ActiveSessionDurationTracker;

  /** Requests an event-driven TUI repaint. */
  requestRender: () => void;
}

function currentUsername(): string {
  try {
    return sanitizeFooterText(userInfo().username);
  } catch {
    return sanitizeFooterText(process.env.USER ?? "user");
  }
}

function shortHostname(): string {
  return sanitizeFooterText(hostname().split(".")[0] ?? hostname());
}

function formatGitOperation(snapshot: GitStatusSnapshot): string {
  if (snapshot.kind === "unavailable" || snapshot.operation === undefined)
    return "";
  const progress =
    snapshot.operation.step !== undefined &&
    snapshot.operation.total !== undefined
      ? ` ${snapshot.operation.step}/${snapshot.operation.total}`
      : "";
  return palette.yellow(`(${snapshot.operation.label}${progress})`);
}

function formatGitFiles(files: GitFileStatus): string {
  const parts: string[] = [];
  if (files.conflicted > 0) parts.push(palette.red(`👎${files.conflicted}`));
  if (files.stashed > 0) parts.push(palette.mauve(`&${files.stashed}`));
  if (files.deleted > 0) parts.push(palette.red(`✘${files.deleted}`));
  if (files.renamed > 0) parts.push(palette.overlay2(`»${files.renamed}`));
  if (files.modified > 0) parts.push(palette.sky(`!${files.modified}`));
  if (files.staged > 0) parts.push(palette.green(`${files.staged}`));
  if (files.untracked > 0) parts.push(palette.overlay2(`?${files.untracked}`));
  if (files.ahead > 0 && files.behind > 0)
    parts.push(palette.peach(`⇕⇡${files.ahead}⇣${files.behind}`));
  else if (files.ahead > 0) parts.push(palette.peach(`⇡${files.ahead}`));
  else if (files.behind > 0) parts.push(palette.peach(`⇣${files.behind}`));
  return parts.join(" ");
}

function formatContext(ctx: ExtensionContext): string {
  const usage = ctx.getContextUsage();
  const contextWindow = usage?.contextWindow ?? ctx.model?.contextWindow ?? 0;
  if (usage === undefined || usage.tokens === null || usage.percent === null) {
    return palette.overlay2(
      `ctx ?/${contextWindow > 0 ? formatTokens(contextWindow) : "?"}`,
    );
  }
  const percent = Math.round(usage.percent);
  const body = `ctx ${formatTokens(usage.tokens)}/${formatTokens(contextWindow)} ${percent}%`;
  if (percent >= 70) return `🥵 ${palette.red(body)}`;
  if (percent >= 50 || usage.tokens >= 250_000)
    return `😢 ${palette.yellow(body)}`;
  return `😎 ${palette.green(body)}`;
}

function rateColor(usedPercent: number, text: string): string {
  if (usedPercent >= 80) return palette.red(text);
  if (usedPercent >= 50) return palette.yellow(text);
  return palette.green(text);
}

function formatRateLimits(snapshot: RateLimitSnapshot): {
  fiveHour: string;
  weekly: string;
} {
  return {
    fiveHour: snapshot.fiveHour
      ? rateColor(
          snapshot.fiveHour.usedPercent,
          `5h:${Math.round(snapshot.fiveHour.usedPercent)}%`,
        )
      : "",
    weekly: snapshot.weekly
      ? rateColor(
          snapshot.weekly.usedPercent,
          `168h:${Math.round(snapshot.weekly.usedPercent)}%`,
        )
      : "",
  };
}

function joined(parts: readonly string[]): string {
  return parts.filter(Boolean).join(separator);
}

function modelVariants(ctx: ExtensionContext): {
  full: string;
  withoutThinking: string;
  modelOnly: string;
} {
  if (!ctx.model) {
    const noModel = palette.sky("no-model");
    return { full: noModel, withoutThinking: noModel, modelOnly: noModel };
  }
  const modelName = sanitizeFooterText(ctx.model.name || ctx.model.id);
  const provider = sanitizeFooterText(ctx.model.provider);
  const modelOnly = palette.sky(modelName);
  const withoutThinking = `${palette.overlay2(`${provider}/`)}${modelOnly}`;
  const thinking = ctx.model.reasoning
    ? ` ${palette.mauve(`(${ctx.thinkingLevel ?? "off"})`)}`
    : "";
  return { full: `${withoutThinking}${thinking}`, withoutThinking, modelOnly };
}

function formatSecondLine(
  ctx: ExtensionContext,
  rates: RateLimitSnapshot,
  cost: number,
  width: number,
): string {
  const model = modelVariants(ctx);
  const context = formatContext(ctx);
  const rate = formatRateLimits(rates);
  const costPart = cost > 0 ? palette.overlay2(`$${cost.toFixed(4)}`) : "";
  const optional = [rate.fiveHour, rate.weekly, costPart];
  let visibleOptional = [...optional];
  let line = joined([model.full, context, ...visibleOptional]);
  for (const index of [2, 1, 0]) {
    if (visibleWidth(line) <= width) return line;
    visibleOptional[index] = "";
    line = joined([model.full, context, ...visibleOptional]);
  }
  if (visibleWidth(line) <= width) return line;
  line = joined([model.withoutThinking, context]);
  if (visibleWidth(line) <= width) return line;
  line = joined([model.modelOnly, context]);
  return visibleWidth(line) <= width
    ? line
    : fitRequiredPair(model.modelOnly, context, width);
}

function statusPriority(key: string): number {
  if (key === "todo") return 0;
  if (key === "subagent-workflow") return 1;
  if (key === "subagent-workflow:usage") return 3;
  return 2;
}

function formatExtensionStatuses(
  footerData: ReadonlyFooterDataProvider,
): string {
  const statuses = [...footerData.getExtensionStatuses().entries()]
    .sort(
      ([leftKey], [rightKey]) =>
        statusPriority(leftKey) - statusPriority(rightKey) ||
        leftKey.localeCompare(rightKey),
    )
    .map(([key, value]) => {
      const clean = sanitizeFooterText(value);
      if (!clean) return "";
      return key === "subagent-workflow" ? `agents ${clean}` : clean;
    })
    .filter(Boolean);
  return statuses.length > 0 ? palette.lavender(statuses.join(" · ")) : "";
}

/** Three-row Catppuccin footer backed only by in-memory Pi and git snapshots during render. */
export class ClaudeFooterComponent implements Component {
  private readonly username = currentUsername();
  private readonly host = shortHostname();
  private disposed = false;

  public constructor(private readonly options: ClaudeFooterComponentOptions) {}

  /** Rebind background git observation when Pi replaces the active session context. */
  public updateContext(ctx: ExtensionContext): void {
    this.options.git.setCwd(ctx.sessionManager.getCwd());
  }

  /** Refresh repository state after Pi reports a lifecycle or tool-completion event. */
  public refreshGit(): void {
    this.options.git.refreshForEvent();
  }

  /** Request a repaint for provider/model lifecycle events without running an idle timer. */
  public requestRender(): void {
    if (!this.disposed) this.options.requestRender();
  }

  public render(width: number): string[] {
    if (width <= 0) return ["", "", ""];
    const ctx = this.options.getContext();
    const git = this.options.git.snapshot();
    const usage = collectSessionUsage(ctx);

    const identity = [
      palette.overlay2("["),
      palette.peach(this.username),
      palette.overlay2("@"),
      palette.red(this.host),
      palette.overlay2("]"),
    ].join("");
    const cwd = palette.peach(
      formatFooterCwd(ctx.sessionManager.getCwd(), homedir()),
    );
    const branch = git.kind === "repository" ? palette.yellow(git.branch) : "";
    const operation = formatGitOperation(git);
    const files = git.kind === "repository" ? formatGitFiles(git.files) : "";
    const firstLine = fitByDropping(
      [identity, cwd, branch, operation, files],
      [4, 3, 2, 0],
      width,
    );

    const secondLine = truncateToWidth(
      formatSecondLine(
        ctx,
        this.options.rateLimits.snapshot(),
        usage.cost,
        width,
      ),
      width,
      palette.overlay2("…"),
    );

    const promptTokens = usage.input + usage.cacheWrite + usage.cacheRead;
    const hitRate =
      promptTokens > 0
        ? Math.round((usage.cacheRead / promptTokens) * 100)
        : undefined;
    const totals = `${palette.overlay2("total")} ${palette.peach(`in:${formatTokens(usage.input)}`)} ${palette.sky(
      `out:${formatTokens(usage.output)}`,
    )}`;
    const cache =
      usage.cacheRead > 0 || usage.cacheWrite > 0
        ? `${palette.overlay2("cache")} ${palette.yellow(`write:${formatTokens(usage.cacheWrite)}`)} ${palette.green(
            `read:${formatTokens(usage.cacheRead)}`,
          )}${hitRate === undefined ? "" : ` ${rateColor(hitRate, `hit:${hitRate}%`)}`}`
        : "";
    const sessionDuration = `${palette.overlay2("session")} ${palette.lavender(
      formatDuration(this.options.sessionDuration.elapsedMilliseconds()),
    )}`;
    const modelDuration = `${palette.overlay2("model")} ${palette.sky(
      formatDuration(this.options.modelDuration.elapsedMilliseconds()),
    )}`;
    const statuses = formatExtensionStatuses(this.options.footerData);
    const thirdParts = [
      totals,
      cache,
      sessionDuration,
      modelDuration,
      statuses,
    ];
    const dropOrder = statuses ? [1, 3, 2, 0] : [1, 3, 2];
    const thirdLine = fitByDropping(thirdParts, dropOrder, width, separator);
    return [firstLine, secondLine, thirdLine];
  }

  public invalidate(): void {}

  /** Stop git watchers, debounce timers, and any running git process. */
  public dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.options.git.dispose();
  }
}

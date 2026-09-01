import type {
  ExtensionAPI,
  ExtensionContext,
  SessionBeforeTreeEvent,
} from "@earendil-works/pi-coding-agent";
import type { Usage } from "@earendil-works/pi-ai";
import { ClaudeFooterComponent } from "./component.js";
import { GitStatusCache } from "./git-status.js";
import {
  ActiveSessionDurationTracker,
  ModelDurationTracker,
  RecentHitRateTracker,
  RecentTokensPerSecondTracker,
  ToolUsageTracker,
  TurnTracker,
  UsageCounter,
} from "./metrics.js";

/** Owns dynamic Pi context, provider timing, and footer component disposal. */
class FooterRuntime {
  private currentContext: ExtensionContext | undefined;
  private component: ClaudeFooterComponent | undefined;
  private readonly modelDuration = new ModelDurationTracker();
  private readonly sessionDuration = new ActiveSessionDurationTracker();
  private readonly tools = new ToolUsageTracker();
  private readonly turns = new TurnTracker();
  private readonly usage = new UsageCounter();
  private readonly hitRate = new RecentHitRateTracker();
  private readonly throughput = new RecentTokensPerSecondTracker();
  private lastTurnModelMilliseconds = 0;
  private treeSummaryProviderActive = false;

  /** Install a fresh component for a started, resumed, forked, or reloaded TUI session. */
  public startSession(ctx: ExtensionContext): void {
    this.currentContext = ctx;
    this.modelDuration.reset();
    this.sessionDuration.reset();
    this.tools.reset();
    this.turns.reset();
    this.tools.prescan(ctx);
    this.usage.prescan(ctx);
    this.hitRate.reset();
    this.throughput.reset();
    this.lastTurnModelMilliseconds = 0;
    this.treeSummaryProviderActive = false;
    this.component?.dispose();
    this.component = undefined;
    if (ctx.mode !== "tui") return;

    ctx.ui.setFooter((tui, _theme, footerData) => {
      const git = new GitStatusCache(this.cwd(ctx), () => tui.requestRender());
      const component = new ClaudeFooterComponent({
        getContext: () => this.currentContext ?? ctx,
        footerData,
        git,
        modelDuration: this.modelDuration,
        tools: this.tools,
        turns: this.turns,
        usage: this.usage,
        hitRate: this.hitRate,
        throughput: this.throughput,
        sessionDuration: this.sessionDuration,
        requestRender: () => tui.requestRender(),
      });
      this.component = component;
      return component;
    });
  }

  /** Update the event context so model, thinking, session entries, and context usage never go stale. */
  public updateContext(ctx: ExtensionContext): void {
    this.currentContext = ctx;
    this.component?.updateContext(ctx);
  }

  /** Begin model wall-time attribution at provider request preparation. */
  public providerRequestStarted(ctx: ExtensionContext): void {
    this.updateContext(ctx);
    if (this.treeSummaryProviderActive) return;
    if (this.modelDuration.start()) this.component?.requestRender();
  }

  /** Finish model wall-time attribution when Pi reports the corresponding work complete. */
  public modelWorkEnded(ctx: ExtensionContext): void {
    this.updateContext(ctx);
    if (this.modelDuration.finish()) this.component?.requestRender();
  }

  /**
   * Exclude branch-summary requests because Pi 0.84.4 has no matching failure event.
   *
   * Successful navigation closes with `session_tree`; cancellation closes through
   * its abort signal. A later user/agent/compaction operation also clears stale
   * state when navigation failed before Pi could emit either event.
   */
  public treeNavigationStarted(
    event: SessionBeforeTreeEvent,
    ctx: ExtensionContext,
  ): void {
    this.updateContext(ctx);
    this.treeSummaryProviderActive =
      event.preparation.userWantsSummary &&
      event.preparation.entriesToSummarize.length > 0;
    if (!this.treeSummaryProviderActive) return;
    event.signal.addEventListener(
      "abort",
      () => {
        this.treeSummaryProviderActive = false;
      },
      { once: true },
    );
  }

  /** Close successful navigation and refresh context-derived values. */
  public treeNavigationEnded(ctx: ExtensionContext): void {
    this.treeSummaryProviderActive = false;
    this.modelWorkEnded(ctx);
  }

  /** Ensure a failed/cancelled tree operation cannot affect a later provider request. */
  public nextUserOperation(ctx: ExtensionContext): void {
    this.treeSummaryProviderActive = false;
    this.updateContext(ctx);
  }

  /** Refresh context-derived fields after a model selection. */
  public modelChanged(ctx: ExtensionContext): void {
    this.updateContext(ctx);
    this.component?.requestRender();
  }

  /** Count one finished tool execution. */
  public toolEnded(name: string, isError: boolean): void {
    this.tools.record(name, isError);
    this.component?.requestRender();
  }

  /** Count one finished turn. */
  public turnEnded(): void {
    const modelMilliseconds = this.modelDuration.elapsedMilliseconds();
    this.throughput.endTurn(modelMilliseconds - this.lastTurnModelMilliseconds);
    this.lastTurnModelMilliseconds = modelMilliseconds;
    this.turns.recordTurn();
    this.hitRate.endTurn();
    this.component?.requestRender();
  }

  /** Accumulate one assistant message's usage into the in-flight hit-rate turn. */
  public hitRateRecorded(usage: Usage | undefined): void {
    this.hitRate.record(usage);
    this.throughput.record(usage);
  }

  /** Count one finished agent run. */
  public agentEnded(): void {
    this.turns.recordAgent();
    this.component?.requestRender();
  }

  /** Add one completed message or summary usage record to the session total. */
  public usageRecorded(usage: Usage | undefined): void {
    this.usage.record(usage);
    this.component?.requestRender();
  }

  /** Refresh context-derived fields after a non-model session change. */
  public contextChanged(ctx: ExtensionContext): void {
    this.updateContext(ctx);
    this.component?.refreshGit();
    this.component?.requestRender();
  }

  /** Refresh repository state after a tool or direct user shell command may have changed files. */
  public repositoryMayHaveChanged(ctx: ExtensionContext): void {
    this.updateContext(ctx);
    this.component?.refreshGit();
  }

  /** Dispose every component-owned watcher/process before Pi replaces the extension runtime. */
  public shutdown(ctx: ExtensionContext): void {
    this.currentContext = ctx;
    this.treeSummaryProviderActive = false;
    this.modelDuration.finish();
    this.component?.dispose();
    this.component = undefined;
    ctx.ui.setFooter(undefined);
  }

  private cwd(ctx: ExtensionContext): string {
    return ctx.sessionManager.getCwd() || ctx.cwd;
  }
}

/** Register the Claude-style footer against Pi 0.84.4 lifecycle and provider events. */
export function registerFooter(pi: ExtensionAPI): void {
  const runtime = new FooterRuntime();
  pi.on("session_start", (_event, ctx) => runtime.startSession(ctx));
  pi.on("session_before_tree", (event, ctx) =>
    runtime.treeNavigationStarted(event, ctx),
  );
  pi.on("session_tree", (event, ctx) => {
    runtime.treeNavigationEnded(ctx);
    runtime.usageRecorded(event.summaryEntry?.usage);
  });
  pi.on("session_before_compact", (_event, ctx) =>
    runtime.nextUserOperation(ctx),
  );
  pi.on("session_compact", (event, ctx) => {
    runtime.modelWorkEnded(ctx);
    runtime.usageRecorded(event.compactionEntry.usage);
  });
  pi.on("session_compact_failed", (_event, ctx) => runtime.modelWorkEnded(ctx));
  pi.on("before_provider_request", (_event, ctx) =>
    runtime.providerRequestStarted(ctx),
  );
  pi.on("message_end", (event, ctx) => {
    if (event.message.role === "assistant") {
      runtime.modelWorkEnded(ctx);
      runtime.usageRecorded(event.message.usage);
      runtime.hitRateRecorded(event.message.usage);
    } else {
      runtime.contextChanged(ctx);
      if (event.message.role === "toolResult")
        runtime.usageRecorded(event.message.usage);
    }
  });
  pi.on("agent_end", (_event, ctx) => {
    runtime.modelWorkEnded(ctx);
    runtime.agentEnded();
  });
  pi.on("turn_end", (_event, ctx) => {
    runtime.updateContext(ctx);
    runtime.turnEnded();
  });
  pi.on("tool_execution_end", (event, ctx) => {
    runtime.updateContext(ctx);
    runtime.toolEnded(event.toolName, event.isError);
  });
  pi.on("model_select", (_event, ctx) => runtime.modelChanged(ctx));
  pi.on("thinking_level_select", (_event, ctx) => runtime.contextChanged(ctx));
  pi.on("before_agent_start", (_event, ctx) => runtime.nextUserOperation(ctx));
  pi.on("input", (_event, ctx) => runtime.nextUserOperation(ctx));
  pi.on("tool_execution_end", (_event, ctx) =>
    runtime.repositoryMayHaveChanged(ctx),
  );
  pi.on("user_bash", (_event, ctx) => runtime.repositoryMayHaveChanged(ctx));
  pi.on("session_shutdown", (_event, ctx) => runtime.shutdown(ctx));
}

export default registerFooter;

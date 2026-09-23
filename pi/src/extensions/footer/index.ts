import type {
  ExtensionAPI,
  ExtensionContext,
  SessionBeforeTreeEvent,
} from "@earendil-works/pi-coding-agent";
import type { AssistantMessageEvent, Usage } from "@earendil-works/pi-ai";
import { performance } from "node:perf_hooks";
import { PROMPT_EDITOR_CONFIGURE, type PromptEditorApi, type EditorStatus } from "../editor/api.js";
import { ModelResponseTracker, SESSION_ACTIVITY_CHANGED } from "./activity.js";
import { ClaudeFooterComponent } from "./component.js";
import { GitStatusCache } from "./git-status.js";
import {
  ActiveSessionDurationTracker,
  ModelDurationTracker,
  PromptRunTracker,
  RecentHitRateTracker,
  RecentTokensPerSecondTracker,
  ToolUsageTracker,
  TurnTracker,
  UsageCounter,
} from "./metrics.js";

/** Braille spinner frames, advanced while the model or a tool is working. */
const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as const;
const SPINNER_INTERVAL_MS = 80;

/** Tracks session metrics and activity for the editor, and owns the footer component. */
class FooterRuntime {
  private currentContext: ExtensionContext | undefined;
  private component: ClaudeFooterComponent | undefined;
  private readonly modelDuration = new ModelDurationTracker();
  private readonly modelResponse = new ModelResponseTracker();
  private readonly promptRun = new PromptRunTracker();
  private readonly sessionDuration = new ActiveSessionDurationTracker();
  private readonly tools = new ToolUsageTracker();
  private readonly turns = new TurnTracker();
  private readonly usage = new UsageCounter();
  private readonly hitRate = new RecentHitRateTracker();
  private readonly throughput = new RecentTokensPerSecondTracker();
  private lastTurnModelMilliseconds = 0;
  private treeSummaryProviderActive = false;
  private readonly activeTools = new Map<string, string>();
  private spinnerFrame = 0;
  private spinnerTimer: ReturnType<typeof setInterval> | undefined;
  private idleStartedAt: number | undefined;
  private idleTimer: ReturnType<typeof setInterval> | undefined;
  private lastPublishedActivity: EditorStatus["activity"] | undefined;

  public constructor(private readonly onActivityChanged: (activity: EditorStatus["activity"]) => void) {}

  /** Install a fresh component for a started, resumed, forked, or reloaded TUI session. */
  public startSession(ctx: ExtensionContext): void {
    this.currentContext = ctx;
    this.modelDuration.reset();
    this.modelResponse.reset();
    this.promptRun.reset();
    this.sessionDuration.reset();
    this.tools.reset();
    this.turns.reset();
    this.tools.prescan(ctx);
    this.usage.prescan(ctx);
    this.hitRate.reset();
    this.throughput.reset();
    this.lastTurnModelMilliseconds = 0;
    this.treeSummaryProviderActive = false;
    this.activeTools.clear();
    this.lastPublishedActivity = undefined;
    this.updateSpinner();
    this.stopIdle();
    this.component?.dispose();
    this.component = undefined;
    if (ctx.mode !== "tui") return;

    // The editor status badge replaces Pi's built-in working loader row.
    ctx.ui.setWorkingVisible(false);

    ctx.ui.setFooter((tui, _theme, footerData) => {
      const git = new GitStatusCache(this.cwd(ctx), () => tui.requestRender());
      const component = new ClaudeFooterComponent({
        getContext: () => this.currentContext ?? ctx,
        footerData,
        git,
        tools: this.tools,
        turns: this.turns,
        promptRun: this.promptRun,
        usage: this.usage,
        hitRate: this.hitRate,
        requestRender: () => tui.requestRender(),
      });
      this.component = component;
      return component;
    });
  }

  /** Supply the editor border with session metrics and current activity. */
  public editorStatus(): EditorStatus {
    const model = this.modelResponse.snapshot();
    const waitingForNextRequest = model.phase === "ready" && this.promptRun.isRunning();
    return {
      sessionMilliseconds: this.sessionDuration.elapsedMilliseconds(),
      ...(this.idleStartedAt === undefined
        ? {}
        : { idleMilliseconds: Math.max(0, performance.now() - this.idleStartedAt) }),
      apiMilliseconds: this.modelDuration.elapsedMilliseconds(),
      tokensPerSecond: this.throughput.tokensPerSecond(),
      timeToFirstTokenMilliseconds: waitingForNextRequest ? undefined : model.timeToFirstTokenMilliseconds,
      activity: this.activityStatus(),
    };
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
    this.modelDuration.start();
    this.modelResponse.startRequest();
    this.updateSpinner();
  }

  /** Observe generated content rather than stream-open or HTTP-header events. */
  public modelResponseUpdated(event: AssistantMessageEvent): void {
    this.modelResponse.record(event);
    this.publishActivity();
    this.component?.requestRender();
  }

  /** Finish model wall-time attribution when Pi reports the corresponding work complete. */
  public modelWorkEnded(ctx: ExtensionContext): void {
    this.updateContext(ctx);
    this.modelDuration.finish();
    this.modelResponse.finishRequest();
    this.updateSpinner();
  }

  /** Compaction streams have their own lifecycle and no ordinary message updates. */
  public compactionStarted(ctx: ExtensionContext): void {
    this.nextUserOperation(ctx);
    this.modelResponse.startCompaction();
    this.updateSpinner();
  }

  /** Both cancellation and success must clear the compacting badge. */
  public compactionEnded(ctx: ExtensionContext): void {
    this.modelResponse.finishCompaction();
    this.modelWorkEnded(ctx);
  }

  /**
   * Exclude branch-summary requests because Pi has no matching failure event.
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
    this.modelResponse.finishRequest();
    this.updateSpinner();
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

  /** Track overlapping tool calls so the editor displays tools until the last call ends. */
  public toolStarted(id: string, name: string): void {
    this.activeTools.set(id, name);
    this.updateSpinner();
  }

  /** Count one finished tool execution and clear its active status. */
  public toolEnded(id: string, name: string, isError: boolean): void {
    this.activeTools.delete(id);
    this.tools.record(name, isError);
    this.updateSpinner();
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

  /** Start a prompt only once Pi enters the agent loop; retries keep its totals. */
  public agentStarted(): void {
    if (!this.promptRun.isRunning()) {
      this.modelResponse.reset();
      this.promptRun.start();
    }
    this.stopIdle();
    this.turns.startAgent();
    this.updateSpinner();
  }

  /** A settled run has no automatic continuation; the next action is up to the user. */
  public agentSettled(): void {
    this.promptRun.finish();
    this.modelResponse.finishRequest();
    this.activeTools.clear();
    this.updateSpinner();
    this.stopIdle();
    this.idleStartedAt = performance.now();
    if (this.currentContext?.mode === "tui") {
      this.idleTimer = setInterval(() => this.component?.requestRender(), 1_000);
    }
    this.component?.requestRender();
  }

  /** Prompt preparation may still fail before agent_start, so no run starts here. */
  public promptSubmitted(): void {
    this.stopIdle();
  }

  /** Count one finished agent run and retain its elapsed wall time. */
  public agentEnded(): void {
    this.activeTools.clear();
    this.updateSpinner();
    this.turns.recordAgent();
    this.component?.requestRender();
  }

  /** Add one completed message or summary usage record to the session total. */
  public usageRecorded(usage: Usage | undefined, source: "parent" | "tool"): void {
    this.usage.record(usage, source);
    if (source === "parent") this.promptRun.record(usage);
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
    this.modelResponse.reset();
    this.promptRun.finish();
    this.activeTools.clear();
    this.updateSpinner();
    this.stopIdle();
    this.component?.dispose();
    this.component = undefined;
    ctx.ui.setFooter(undefined);
  }

  private stopIdle(): void {
    const wasIdle = this.idleStartedAt !== undefined;
    this.idleStartedAt = undefined;
    if (this.idleTimer !== undefined) {
      clearInterval(this.idleTimer);
      this.idleTimer = undefined;
    }
    if (wasIdle) this.component?.requestRender();
  }

  /** One activity decision shared by editor rendering and title subscribers. */
  private activityStatus(): EditorStatus["activity"] {
    const spinner = SPINNER_FRAMES[this.spinnerFrame] ?? SPINNER_FRAMES[0];
    const model = this.modelResponse.snapshot();
    if (model.phase === "compacting") return { kind: "compacting", spinner };
    const toolNames = [...this.activeTools.values()];
    if (toolNames.length > 0) {
      return { kind: "tool", spinner, toolName: toolNames.at(-1)!, toolCount: toolNames.length };
    }
    if (model.phase !== "ready") return { kind: model.phase, spinner };
    return this.promptRun.isRunning() ? { kind: "waiting", spinner } : { kind: "ready" };
  }

  private publishActivity(): void {
    const next = this.activityStatus();
    const previous = this.lastPublishedActivity;
    if (previous?.kind === next.kind) {
      if (next.kind !== "tool") return;
      if (previous.kind === "tool" && previous.toolName === next.toolName && previous.toolCount === next.toolCount) return;
    }
    this.lastPublishedActivity = next;
    this.onActivityChanged(next);
  }

  private updateSpinner(): void {
    const active = this.modelResponse.snapshot().phase !== "ready" || this.activeTools.size > 0 || this.promptRun.isRunning();
    if (active && !this.spinnerTimer && this.currentContext?.mode === "tui") {
      this.spinnerFrame = 0;
      this.spinnerTimer = setInterval(() => {
        this.spinnerFrame = (this.spinnerFrame + 1) % SPINNER_FRAMES.length;
        this.component?.requestRender();
      }, SPINNER_INTERVAL_MS);
    } else if (!active && this.spinnerTimer) {
      clearInterval(this.spinnerTimer);
      this.spinnerTimer = undefined;
    }
    this.publishActivity();
    this.component?.requestRender();
  }

  private cwd(ctx: ExtensionContext): string {
    return ctx.sessionManager.getCwd() || ctx.cwd;
  }
}

/** Connect the editor and footer to Pi's lifecycle and provider events. */
export function registerFooter(pi: ExtensionAPI): void {
  const runtime = new FooterRuntime((activity) => pi.events.emit(SESSION_ACTIVITY_CHANGED, activity));
  pi.events.on(PROMPT_EDITOR_CONFIGURE, (requested) => {
    (requested as PromptEditorApi).useStatus(() => runtime.editorStatus());
  });
  pi.on("session_start", (_event, ctx) => runtime.startSession(ctx));
  pi.on("session_before_tree", (event, ctx) =>
    runtime.treeNavigationStarted(event, ctx),
  );
  pi.on("session_tree", (event, ctx) => {
    runtime.treeNavigationEnded(ctx);
    runtime.usageRecorded(event.summaryEntry?.usage, "parent");
  });
  pi.on("session_before_compact", (_event, ctx) => runtime.compactionStarted(ctx));
  pi.on("session_compact", (event, ctx) => {
    runtime.compactionEnded(ctx);
    runtime.usageRecorded(event.compactionEntry.usage, "parent");
  });
  pi.on("session_compact_failed", (_event, ctx) => runtime.compactionEnded(ctx));
  pi.on("before_provider_request", (_event, ctx) =>
    runtime.providerRequestStarted(ctx),
  );
  pi.on("message_update", (event) => runtime.modelResponseUpdated(event.assistantMessageEvent));
  pi.on("message_end", (event, ctx) => {
    if (event.message.role === "assistant") {
      runtime.modelWorkEnded(ctx);
      runtime.usageRecorded(event.message.usage, "parent");
      runtime.hitRateRecorded(event.message.usage);
    } else {
      runtime.contextChanged(ctx);
      if (event.message.role === "toolResult")
        runtime.usageRecorded(event.message.usage, "tool");
    }
  });
  pi.on("agent_start", () => runtime.agentStarted());
  pi.on("agent_settled", () => runtime.agentSettled());
  pi.on("agent_end", (_event, ctx) => {
    runtime.modelWorkEnded(ctx);
    runtime.agentEnded();
  });
  pi.on("turn_end", (_event, ctx) => {
    runtime.updateContext(ctx);
    runtime.turnEnded();
  });
  pi.on("tool_execution_start", (event, ctx) => {
    runtime.updateContext(ctx);
    runtime.toolStarted(event.toolCallId, event.toolName);
  });
  pi.on("tool_execution_end", (event, ctx) => {
    runtime.updateContext(ctx);
    runtime.toolEnded(event.toolCallId, event.toolName, event.isError);
  });
  pi.on("model_select", (_event, ctx) => runtime.modelChanged(ctx));
  pi.on("thinking_level_select", (_event, ctx) => runtime.contextChanged(ctx));
  pi.on("before_agent_start", (_event, ctx) => {
    runtime.promptSubmitted();
    runtime.nextUserOperation(ctx);
  });
  pi.on("input", (_event, ctx) => runtime.nextUserOperation(ctx));
  pi.on("tool_execution_end", (_event, ctx) =>
    runtime.repositoryMayHaveChanged(ctx),
  );
  pi.on("user_bash", (_event, ctx) => runtime.repositoryMayHaveChanged(ctx));
  pi.on("session_shutdown", (_event, ctx) => runtime.shutdown(ctx));
}

export default registerFooter;

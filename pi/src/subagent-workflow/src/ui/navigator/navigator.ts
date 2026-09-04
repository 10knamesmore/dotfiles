/**
 * The /agents full-screen navigator overlay and its command registration.
 *
 * `/agents` and `/workflows` open the same three-level overlay:
 *   runs ──enter──▶ run detail ──enter──▶ agent transcript
 *        ◀──esc───            ◀──esc────
 *
 * Data comes from the run store (kept current by the runner on every event); the
 * live runner is consulted only for pinning active runs, transcript refresh
 * events, and the steer/stop controls. Everything is guarded on
 * ctx.hasUI - in non-TUI modes the command prints a plain-text run summary.
 */

import { statSync } from "node:fs";
import type { ExtensionAPI, ExtensionCommandContext, ExtensionContext, Theme } from "@earendil-works/pi-coding-agent";
import { matchesKey, parseKey, truncateToWidth, type Component, type TUI } from "@earendil-works/pi-tui";
import type { ChildSession } from "../../runner/child-session.js";
import type { SpawnedRun } from "../../runner/runner.js";
import { EMPTY_USAGE } from "../../store/run-store.js";
import type { SubagentHandle } from "../../types.js";
import { reportDiagnostic } from "../../diagnostics.js";
import { errorMessage } from "../../util.js";
import { formatTokenUsage, hasTokenUsage, type ThemeLike } from "../format.js";
import { sanitizeTerminalText } from "../sanitize.js";
import { suppressInlineImages } from "../suppress-inline-images.js";
import { AgentView } from "./agent-view.js";
import { cycleFilter, footerHint, keyToAction, orderedChildren, runActionAvailability, type RunActionAvailability } from "./controls.js";
import { NavigatorModel, NavigatorState } from "./model.js";
import { pageRunDetail, renderRunDetail, renderRunList } from "./render.js";
import type { ChildRow, RunDetail, RunSummary } from "./store-read.js";
import { readSessionMessages, type TranscriptMessage } from "./transcript.js";

/** The slice of SubagentRunner the navigator needs; small enough to fake in tests. */
export interface NavigatorRunner {
  liveRunIds(): string[];
  runHandles(runId: string): SubagentHandle[];
  liveSession(childId: string): ChildSession | undefined;
  get(childId: string): SubagentHandle | undefined;
  stopRun(runId: string): Promise<void>;
  subscribeSpawns(listener: (run: SpawnedRun) => void): () => void;
}

export type NavigatorOpenContext = Pick<ExtensionContext, "cwd" | "hasUI" | "ui" | "sessionManager" | "modelRegistry" | "model">;

/** Optional run/child to open directly from an external input selection. */
export interface NavigatorOpenTarget {
  runId: string;
  childId?: string;
}

export interface NavigatorFollowUp {
  /** Resolve + spawn; throws with a user-facing message on any refusal. */
  send(runId: string, childId: string, prompt: string, ctx: NavigatorOpenContext): { runId: string; childId: string };
}

interface NavigatorServices {
  runner: NavigatorRunner;
  followUp?: NavigatorFollowUp;
  /** Extract a workflow's display name from its script for run rows. */
  describeWorkflow?: (script: string) => string;
  /** Override the runs root (tests); defaults to the store location. */
  root?: string;
  /** Delegate to the 3b save flow for a workflow run's script. */
  saveWorkflowScript?: (runId: string, ctx: ExtensionCommandContext) => Promise<void>;
}

/** Register `/agents` and its `/workflows` alias, returning their shared open path. */
export function registerNavigator(pi: ExtensionAPI, services: NavigatorServices): (ctx: NavigatorOpenContext, target?: NavigatorOpenTarget) => Promise<void> {
  let isOpen = false;
  const open = async (ctx: NavigatorOpenContext, target?: NavigatorOpenTarget): Promise<void> => {
    if (!ctx.hasUI) return runNavigator(services, ctx, target);
    if (isOpen) return;
    isOpen = true;
    try {
      await runNavigator(services, ctx, target);
    } finally {
      isOpen = false;
    }
  };
  const command = (_args: string, ctx: ExtensionCommandContext) => open(ctx);
  pi.registerCommand("agents", { description: "Browse subagent and workflow runs (live and history)", handler: command });
  pi.registerCommand("workflows", { description: "Browse workflow and subagent runs (alias of /agents)", handler: command });
  return open;
}

async function runNavigator(services: NavigatorServices, ctx: NavigatorOpenContext, target?: NavigatorOpenTarget): Promise<void> {
  const model = new NavigatorModel(
    ctx.cwd,
    { root: services.root, describeWorkflow: services.describeWorkflow },
    (id) => services.runner.liveRunIds().includes(id),
  );
  if (!ctx.hasUI) {
    console.log(formatPlainSummary(model.runs()));
    return;
  }
  return openNavigator(services, ctx, model, target);
}

export function formatPlainSummary(runs: RunSummary[]): string {
  const lines = [`Agents: ${runs.length} run(s)`];
  for (const run of runs) {
    if (run.corrupt) {
      lines.push(`  · ${sanitizeTerminalText(run.label)} ${sanitizeTerminalText(run.runId)}`);
      continue;
    }
    const usage = hasTokenUsage(run.usage) ? ` · ${formatTokenUsage(run.usage)}` : "";
    const health = run.kind === "workflow" && run.status === "completed" && (run.failed > 0 || run.aborted > 0)
      ? `${run.completed} ok${run.failed > 0 ? ` · ${run.failed} failed` : ""}${run.aborted > 0 ? ` · ${run.aborted} aborted` : ""}`
      : `${run.done}/${run.total}`;
    lines.push(`  ${sanitizeTerminalText(run.status).padEnd(9)} ${sanitizeTerminalText(run.label)}  [${run.kind} · ${health}${usage} · ${sanitizeTerminalText(run.runId)}]`);
  }
  return lines.join("\n");
}

function isCommandContext(ctx: NavigatorOpenContext): ctx is ExtensionCommandContext {
  return "waitForIdle" in ctx;
}

function openNavigator(services: NavigatorServices, ctx: NavigatorOpenContext, model: NavigatorModel, target?: NavigatorOpenTarget): Promise<void> {
  const state = new NavigatorState();
  const runner = services.runner;
  if (target) {
    state.seedRun(target.runId);
    if (target.childId) {
      const children = orderedChildren(model.detail(target.runId), "all");
      const childIndex = children.findIndex((child) => child.id === target.childId);
      if (childIndex >= 0) {
        state.setChildCursor(childIndex, children);
        state.drill(model);
      }
    }
  } else {
    // Intersect with the model first: the runner is process-global across cwd
    // generations, so a live run in another cwd must not defeat smart landing.
    const visibleLiveRunIds = orderedLiveRunIds(model.runs(), runner.liveRunIds());
    if (visibleLiveRunIds.length === 1) state.seedRun(visibleLiveRunIds[0]!);
  }

  return ctx.ui.custom<void>(
    (tui: TUI, theme: Theme, _keybindings, done: (result: void) => void) => {
      let agentView: AgentView | undefined;
      let timer: ReturnType<typeof setInterval> | undefined;
      let stopArmedRunId: string | undefined;
      let stoppingRunId: string | undefined;
      const subscriptions: Array<() => void> = [];

      const rerender = () => tui.requestRender();
      const clearSubscriptions = () => {
        for (const unsubscribe of subscriptions.splice(0)) unsubscribe();
      };
      // Re-render (which re-reads the store) whenever any live child emits.
      const resubscribe = () => {
        clearSubscriptions();
        for (const runId of runner.liveRunIds()) {
          for (const handle of runner.runHandles(runId)) subscriptions.push(handle.subscribe(() => rerender()));
        }
      };
      resubscribe();
      const unsubscribeSpawns = runner.subscribeSpawns((run) => {
        if (run.parentSessionId !== ctx.sessionManager.getSessionId()) return;
        resubscribe();
        rerender();
      });

      const disposeAgentView = () => {
        agentView?.dispose();
        agentView = undefined;
      };
      const reportError = (message: string) => {
        ctx.ui.notify(message, "error");
        reportDiagnostic(`[subagent-workflow] ${message}`);
      };
      const openAgentView = () => {
        disposeAgentView();
        if (state.runId && state.childId) {
          const sourceRunId = state.runId;
          const sourceChildId = state.childId;
          agentView = buildAgentView(runner, tui, model, sourceRunId, sourceChildId, reportError, services.followUp ? {
            onMessage: (text) => {
              let target: { runId: string; childId: string };
              try {
                target = services.followUp!.send(sourceRunId, sourceChildId, text, ctx);
              } catch (error) {
                reportError(`Could not message agent ${sanitizeTerminalText(sourceChildId)}: ${sanitizeTerminalText(errorMessage(error))}`);
                return false;
              }

              try {
                state.filter = "all";
                const children = orderedChildren(model.detail(target.runId), state.filter);
                const childIndex = children.findIndex((child) => child.id === target.childId);
                if (childIndex < 0) throw new Error(`spawned child ${target.childId} is missing from run ${target.runId}`);
                state.switchRun(target.runId);
                state.setChildCursor(childIndex, children);
                if (state.drill(model) !== "agent") throw new Error(`spawned child ${target.childId} could not be opened`);
                openAgentView();
              } catch (error) {
                reportError(`Follow-up run ${sanitizeTerminalText(target.runId)} was started, but could not be opened: ${sanitizeTerminalText(errorMessage(error))}`);
              }
              return true;
            },
          } : undefined);
        }
      };
      if (state.level === "agent") openAgentView();

      const cleanup = () => {
        unsubscribeSpawns();
        clearSubscriptions();
        disposeAgentView();
        if (timer) clearInterval(timer);
        timer = undefined;
        restoreImages();
      };
      const finish = () => {
        cleanup();
        done();
      };

      const currentCount = (): number => {
        if (state.level === "runs") return model.runs().length;
        if (state.level === "run" && state.runId) return orderedChildren(model.detail(state.runId), state.filter).length;
        return 0;
      };

      const actionsFor = (detail: RunDetail): RunActionAvailability => runActionAvailability(
        detail,
        runner.liveRunIds().includes(detail.runId),
        runner.runHandles(detail.runId).map((handle) => handle.status),
        services.saveWorkflowScript !== undefined && isCommandContext(ctx),
      );

      const stopRun = async () => {
        const runId = state.currentRunId(model.runs());
        if (!runId) return;
        if (stoppingRunId === runId) return;
        if (!actionsFor(model.detail(runId)).canStop) {
          stopArmedRunId = undefined;
          ctx.ui.notify("Run has no live agents to stop", "info");
          return;
        }
        if (stopArmedRunId !== runId) {
          stopArmedRunId = runId;
          rerender();
          return;
        }
        stopArmedRunId = undefined;
        stoppingRunId = runId;
        rerender();
        // stopRun cancels the run's workflow loop (so no further agents spawn),
        // then aborts every live child - not just the currently-running ones.
        try {
          await runner.stopRun(runId);
          ctx.ui.notify(`Stopped ${runId}`, "info");
          resubscribe();
        } catch (error) {
          ctx.ui.notify(`Could not stop ${runId}: ${sanitizeTerminalText(errorMessage(error))}`, "error");
        } finally {
          stoppingRunId = undefined;
          rerender();
        }
      };

      const saveSelected = async () => {
        const runId = state.currentRunId(model.runs());
        if (!runId) return;
        if (!services.saveWorkflowScript || !isCommandContext(ctx)) {
          ctx.ui.notify("Saving is not available", "error");
          return;
        }
        const detail = model.detail(runId);
        if (!actionsFor(detail).canSave) {
          ctx.ui.notify(saveRefusalMessage(detail), "warning");
          return;
        }
        await services.saveWorkflowScript(runId, ctx);
      };

      const act = (data: string) => {
        const keyId = navigatorKeyId(data);
        // A focused composer owns tab; otherwise AgentView declines it so live-run cycling can handle it.
        if (state.level === "agent" && agentView?.handleInput(data, keyId)) return;
        const action = keyToAction(keyId, state.level);
        if (action.type !== "stop") stopArmedRunId = undefined;
        switch (action.type) {
          case "move":
            if (state.level === "runs") state.moveRun(action.delta, model.runs());
            else if (state.level === "run" && state.runId) {
              state.moveChild(action.delta, orderedChildren(model.detail(state.runId), state.filter));
              state.scroll = undefined;
            } else {
              state.move(action.delta, currentCount());
              state.scroll = undefined;
            }
            break;
          case "pageMove":
            if (state.level === "runs") state.pageMoveRun(action.delta, model.runs(), navigationPageSize(tui));
            else if (state.level === "run" && state.runId) {
              const detail = model.detail(state.runId);
              const children = orderedChildren(detail, state.filter);
              state.reconcileChildren(children);
              const page = pageRunDetail(detail, state.cursor, state.filter, action.delta, navigationPageSize(tui), state.scroll);
              state.setChildCursor(page.cursor, children);
              state.scroll = page.row;
            } else state.pageMove(action.delta, currentCount(), navigationPageSize(tui));
            break;
          case "cycleLive": {
            const runs = model.runs();
            const liveRunIds = orderedLiveRunIds(runs, runner.liveRunIds());
            if (liveRunIds.length < 2) break;
            const currentIndex = liveRunIds.indexOf(state.currentRunId(runs) ?? "");
            const targetIndex = currentIndex < 0
              ? (action.delta === 1 ? 0 : liveRunIds.length - 1)
              : (currentIndex + action.delta + liveRunIds.length) % liveRunIds.length;
            const targetRunId = liveRunIds[targetIndex]!;
            if (state.level === "runs") {
              state.reconcileRuns(runs);
              const targetRow = runs.findIndex((run) => run.runId === targetRunId);
              state.moveRun(targetRow - state.cursor, runs);
            } else {
              disposeAgentView();
              state.switchRun(targetRunId);
            }
            break;
          }
          case "drill":
            if (state.drill(model) === "agent") openAgentView();
            break;
          case "back":
            disposeAgentView();
            if (!state.back()) return finish();
            break;
          case "close":
            return finish();
          case "filter":
            state.filter = cycleFilter(state.filter);
            if (state.level === "run" && state.runId) state.reconcileChildren(orderedChildren(model.detail(state.runId), state.filter));
            else state.clampCursor(currentCount());
            state.scroll = undefined;
            break;
          case "stop":
            void stopRun();
            return;
          case "save":
            void saveSelected();
            return;
          default:
            return;
        }
        rerender();
      };

      // Inline images paint over overlays (see suppress-inline-images.ts);
      // hide them while the navigator is up. Registered last: anything above
      // that throws would reject the factory without running dispose, and the
      // patch would then outlive the overlay.
      const restoreImages = suppressInlineImages(tui);

      const component: Component & { dispose?(): void } = {
        render: (width: number) => {
          const rows = tui.terminal?.rows ?? 24;
          const maxTotal = Math.max(8, Math.floor(rows * 0.9));
          const inner = Math.max(20, width - 4);
          const runs = model.runs();
          const canCycle = orderedLiveRunIds(runs, runner.liveRunIds()).length >= 2;
          const content = renderContent(
            state,
            model,
            runs,
            canCycle,
            agentView,
            theme,
            inner,
            maxTotal - 3,
            actionsFor,
            stopArmedRunId,
          );
          if (content.hasSpinner && !timer) timer = setInterval(rerender, 100);
          if (!content.hasSpinner && timer) {
            clearInterval(timer);
            timer = undefined;
          }
          return boxLines(content.lines, content.footer, inner, maxTotal, theme);
        },
        handleInput: act,
        invalidate: () => {},
        dispose: () => cleanup(),
      };

      return component;
    },
    { overlay: true, overlayOptions: { width: "90%", maxHeight: "90%", anchor: "center", margin: 1 } },
  );
}

interface RenderedContent {
  lines: string[];
  footer: string;
  hasSpinner: boolean;
}

function renderContent(
  state: NavigatorState,
  model: NavigatorModel,
  runs: RunSummary[],
  canCycle: boolean,
  agentView: AgentView | undefined,
  theme: ThemeLike,
  inner: number,
  budget: number,
  actionsFor: (detail: RunDetail) => RunActionAvailability,
  stopArmedRunId?: string,
): RenderedContent {
  const now = Date.now();
  if (state.level === "agent" && agentView) {
    return {
      lines: agentView.render(inner, budget - 2, theme),
      // A focused composer owns tab, so the cycle hint hides while it is open.
      footer: footerHint({
        level: "agent",
        canSteer: agentView.canSteer,
        canMessage: agentView.canMessage,
        canStop: agentView.canStop,
        stopArmed: agentView.isStopArmed,
        canCycle: canCycle && !agentView.composerOpen,
      }, theme),
      hasSpinner: !!state.runId
        && !!state.childId
        && model.detail(state.runId).children.find((child) => child.id === state.childId)?.status === "running",
    };
  }
  if (state.level === "run" && state.runId) {
    const detail = model.detail(state.runId);
    const actions = actionsFor(detail);
    // Refresh renders re-read the detail, and live rows reorder as children
    // start; pin the cursor to the selected child's identity, not its index.
    state.reconcileChildren(orderedChildren(detail, state.filter));
    return {
      lines: renderRunDetail(detail, state.cursor, state.filter, theme, inner, now, budget, state.scroll),
      footer: footerHint({
        level: "run",
        filter: state.filter,
        canCycle,
        ...actions,
        stopArmed: actions.canStop && stopArmedRunId === detail.runId,
      }, theme),
      hasSpinner: detail.children.some((child) => child.status === "running"),
    };
  }
  state.reconcileRuns(runs);
  const selectedRunId = state.currentRunId(runs);
  const actions = selectedRunId ? actionsFor(model.detail(selectedRunId)) : { canStop: false, canSave: false };
  return {
    lines: renderRunList(runs, state.cursor, theme, inner, now, budget),
    footer: footerHint({
      level: "runs",
      canCycle,
      ...actions,
      stopArmed: actions.canStop && stopArmedRunId === selectedRunId,
    }, theme),
    hasSpinner: runs.some((run) => run.status === "running"),
  };
}

function navigatorKeyId(data: string): string | undefined {
  if (matchesKey(data, "shift+j")) return "shift+j";
  if (matchesKey(data, "shift+k")) return "shift+k";
  return (parseKey(data) ?? "").toLowerCase() || undefined;
}

function orderedLiveRunIds(runs: readonly RunSummary[], liveRunIds: readonly string[]): string[] {
  const live = new Set(liveRunIds);
  return runs.filter((run) => live.has(run.runId)).map((run) => run.runId);
}

/** Approximate selectable rows in one overlay viewport, excluding box chrome and headers. */
function navigationPageSize(tui: TUI): number {
  return Math.max(1, Math.floor((tui.terminal?.rows ?? 24) * 0.9) - 7);
}

function lastAssistant(messages: readonly TranscriptMessage[]): TranscriptMessage | undefined {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (messages[index]?.role === "assistant") return messages[index];
  }
  return undefined;
}

function isNewerAssistant(current: TranscriptMessage, persisted: TranscriptMessage | undefined): boolean {
  if (!persisted) return true;
  const currentResponseId = typeof current.responseId === "string" ? current.responseId : undefined;
  const persistedResponseId = typeof persisted.responseId === "string" ? persisted.responseId : undefined;
  if (currentResponseId !== undefined && persistedResponseId !== undefined) return currentResponseId !== persistedResponseId;
  if (typeof current.timestamp !== "number") return false;
  if (typeof persisted.timestamp === "number" && current.timestamp < persisted.timestamp) return false;
  if (typeof persisted.timestamp !== "number" || current.timestamp > persisted.timestamp) return true;
  // Equal id-less timestamps: this is either the persisted message itself
  // crossing the persistence boundary (suppress: splicing it again would
  // duplicate it) or a distinct response whose provider reuses the response
  // timestamp across every delta (keep it visible for its whole stream).
  // Serialized content is the only identity left to tell them apart.
  try {
    return JSON.stringify(current.content) !== JSON.stringify(persisted.content);
  } catch {
    return false;
  }
}

interface AgentFollowUpControls {
  onMessage: (text: string) => boolean;
}

/**
 * Display-time eligibility from the projection the view already renders; the
 * follow-up resolver revalidates authoritatively when the message is sent.
 */
export function isMessageableChild(child: ChildRow | undefined): boolean {
  if (!child?.sessionFile || child.spec.isolation === "worktree") return false;
  return child.status === "completed" || child.status === "failed" || child.status === "aborted";
}

export function buildAgentView(
  runner: NavigatorRunner,
  tui: TUI,
  model: NavigatorModel,
  runId: string,
  childId: string,
  reportError: (message: string) => void = (message) => reportDiagnostic(`[subagent-workflow] ${message}`),
  followUp?: AgentFollowUpControls,
): AgentView {
  // One render pass reads the child row several times (header, live state,
  // transcript path, eligibility). model.detail() bypasses the disk cache, so
  // memoize per synchronous burst: the microtask reset keeps later frames fresh.
  let burstChild: ChildRow | undefined;
  let burstValid = false;
  const child = (): ChildRow | undefined => {
    if (!burstValid) {
      burstChild = model.detail(runId).children.find((row) => row.id === childId);
      burstValid = true;
      queueMicrotask(() => { burstValid = false; });
    }
    return burstChild;
  };
  const header = () => {
    const row = child();
    return { label: row?.label ?? childId, model: row?.model ?? "", thinking: row?.thinking, status: row?.status ?? "pending", usage: row?.usage ?? EMPTY_USAGE() };
  };

  // A queued or constructing child has a durable row before it has a session.
  // Resolve the session lazily so an already-open view becomes live in place.
  // Keeping one AgentView instance also preserves its transcript scroll state.
  let notify: (() => void) | undefined;
  let subscribedSession: ChildSession | undefined;
  let unsubscribeSession: (() => void) | undefined;
  const session = (): ChildSession | undefined => {
    const next = runner.get(childId)?.status === "running" ? runner.liveSession(childId) : undefined;
    if (next === subscribedSession) return next;
    unsubscribeSession?.();
    subscribedSession = next;
    unsubscribeSession = next && notify ? next.subscribe(() => notify?.()) : undefined;
    return next;
  };

  let cachedPath: string | undefined;
  let cachedSize: number | undefined;
  let cachedMtimeMs: number | undefined;
  let cachedMissing = false;
  let cachedMessages: ReturnType<typeof readSessionMessages> = [];
  let combinedPersisted: TranscriptMessage[] | undefined;
  let combinedCurrent: TranscriptMessage | undefined;
  let combinedMessages: TranscriptMessage[] | undefined;
  const withCurrentAssistant = (persisted: TranscriptMessage[], live: ChildSession | undefined): TranscriptMessage[] => {
    const current = live?.currentAssistant;
    if (persisted === combinedPersisted && current === combinedCurrent && combinedMessages) return combinedMessages;
    combinedPersisted = persisted;
    combinedCurrent = current;
    combinedMessages = current && isNewerAssistant(current, lastAssistant(persisted))
      ? [...persisted, current]
      : persisted;
    return combinedMessages;
  };
  const messages = () => {
    const live = session();
    const path = live?.sessionFile ?? child()?.sessionFile;
    if (!path) {
      if (cachedPath !== undefined) {
        cachedPath = undefined;
        cachedSize = undefined;
        cachedMtimeMs = undefined;
        cachedMissing = false;
        cachedMessages = [];
      }
      return withCurrentAssistant(cachedMessages, live);
    }

    try {
      const stat = statSync(path);
      if (path === cachedPath && !cachedMissing && stat.size === cachedSize && stat.mtimeMs === cachedMtimeMs) {
        return withCurrentAssistant(cachedMessages, live);
      }
      cachedPath = path;
      cachedSize = stat.size;
      cachedMtimeMs = stat.mtimeMs;
      cachedMissing = false;
    } catch {
      if (path === cachedPath && cachedMissing) return withCurrentAssistant(cachedMessages, live);
      cachedPath = path;
      cachedSize = undefined;
      cachedMtimeMs = undefined;
      cachedMissing = true;
    }
    cachedMessages = readSessionMessages(path);
    return withCurrentAssistant(cachedMessages, live);
  };

  const runControl = (action: "steer" | "stop", invoke: (handle: SubagentHandle) => Promise<void>): void => {
    const handle = runner.get(childId);
    if (!handle) return;
    void invoke(handle).catch((error: unknown) => {
      reportError(`Could not ${action} agent ${sanitizeTerminalText(childId)}: ${sanitizeTerminalText(errorMessage(error))}`);
    });
  };

  return new AgentView({
    tui,
    header,
    messages,
    live: () => {
      const status = child()?.status;
      return runner.get(childId) !== undefined && (status === "running" || status === "pending");
    },
    onSteer: (text) => runControl("steer", (handle) => handle.steer(text)),
    canMessage: followUp ? () => isMessageableChild(child()) : undefined,
    onMessage: followUp?.onMessage,
    onSteerUnavailable: () => reportError(`Agent ${sanitizeTerminalText(childId)} ended and cannot be steered`),
    onStop: () => runControl("stop", (handle) => handle.abort()),
    subscribe: (listener) => {
      notify = listener;
      session();
      return () => {
        notify = undefined;
        unsubscribeSession?.();
        unsubscribeSession = undefined;
        subscribedSession = undefined;
      };
    },
  });
}

export function saveRefusalMessage(detail: RunDetail): string {
  if (detail.label === "quarantined - crashed mid-resume") {
    return `Workflow run ${detail.runId} is quarantined after a crashed generation commit and cannot be saved`;
  }
  if (detail.kind !== "workflow") return `Run ${detail.runId} is not a workflow and cannot be saved`;
  if (detail.status === "failed" || detail.status === "aborted") {
    return `Workflow run ${detail.runId} did not complete successfully and cannot be saved`;
  }
  if (!detail.hasScript) return `Workflow run ${detail.runId} has no saved script`;
  return `Workflow run ${detail.runId} is not completed and cannot be saved`;
}

const BORDER = "muted";

/** Wrap content in a stable-height titled box, padded to `inner` width. */
export function boxLines(content: string[], footer: string, inner: number, totalRows: number, theme: ThemeLike): string[] {
  const border = (text: string) => theme.fg(BORDER, text);
  const pad = (line: string) => `${border("│")} ${truncateToWidth(line, inner, "", true)} ${border("│")}`;
  const top = border(`╭─${theme.fg("dim", " agents ")}${"─".repeat(Math.max(0, inner - 7))}╮`);
  const bottom = border(`╰${"─".repeat(inner + 2)}╯`);
  const bodyRows = Math.max(1, totalRows - 3);
  const rows = content.slice(0, bodyRows).map(pad);
  while (rows.length < bodyRows) rows.push(pad(""));
  rows.push(pad(footer));
  return [top, ...rows, bottom];
}

/**
 * The /agents navigator and its command registration.
 *
 * `/agents` and `/workflows` open the same three-level browser:
 *   runs ──enter/l──▶ run detail ──enter/l──▶ agent transcript
 *        ◀──esc/h───            ◀──esc/h────
 *
 * Data comes from the run store (kept current by the runner on every event); the
 * live runner is consulted only for pinning active runs, transcript refresh
 * events, and the steer/stop controls. The navigator requires an interactive UI.
 */

import { statSync } from "node:fs";
import type {
  ExtensionAPI,
  ExtensionCommandContext,
  ExtensionContext,
  KeybindingsManager,
  Theme,
} from "@earendil-works/pi-coding-agent";
import { matchesKey, parseKey, truncateToWidth, type TUI, type TuiMouseEvent } from "@earendil-works/pi-tui";
import type { ChildSession } from "../../runner/child-session.js";
import type { SubagentRunner } from "../../runner/runner.js";
import { EMPTY_USAGE } from "../../store/run-store.js";
import type { SubagentHandle } from "../../types.js";
import { reportDiagnostic } from "../../diagnostics.js";
import { errorMessage } from "../../util.js";
import type { ThemeLike } from "../format.js";
import { sanitizeTerminalText } from "../sanitize.js";
import { AgentView } from "./agent-view.js";
import {
  cycleFilter,
  footerHint,
  keyToAction,
  orderedChildren,
  runActionAvailability,
  type RunActionAvailability,
} from "./controls.js";
import { NavigatorModel, NavigatorState } from "./model.js";
import { pageRunDetail, renderRunDetail, renderRunList, scrollRunDetail, scrollRunList } from "./render.js";
import type { ChildRow, RunDetail, RunSummary } from "../../store/run-projection.js";
import { readSessionMessages, type TranscriptMessage } from "../../store/session-transcript.js";

export type NavigatorOpenContext = Pick<
  ExtensionContext,
  "cwd" | "hasUI" | "ui" | "sessionManager" | "modelRegistry" | "model"
>;

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
  runner: SubagentRunner;
  followUp?: NavigatorFollowUp;
}

/** Register `/agents` and its `/workflows` alias, returning their shared open path. */
export function registerNavigator(
  pi: ExtensionAPI,
  services: NavigatorServices,
): (ctx: NavigatorOpenContext, target?: NavigatorOpenTarget) => Promise<void> {
  let isOpen = false;
  const open = async (ctx: NavigatorOpenContext, target?: NavigatorOpenTarget): Promise<void> => {
    if (!ctx.hasUI || isOpen) return;
    isOpen = true;
    try {
      await openNavigator(services, ctx, new NavigatorModel(services.runner), target);
    } finally {
      isOpen = false;
    }
  };
  const command = (_args: string, ctx: ExtensionCommandContext) => open(ctx);
  pi.registerCommand("agents", {
    description: "Browse subagent and workflow runs (live and history)",
    handler: command,
  });
  pi.registerCommand("workflows", {
    description: "Browse workflow and subagent runs (alias of /agents)",
    handler: command,
  });
  return open;
}

function openNavigator(
  services: NavigatorServices,
  ctx: NavigatorOpenContext,
  model: NavigatorModel,
  target?: NavigatorOpenTarget,
): Promise<void> {
  const state = new NavigatorState();
  const { runner } = services;
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
    const visibleLiveRunIds = orderedLiveRunIds(model.runs(), runner.liveRunIds());
    if (visibleLiveRunIds.length === 1) state.seedRun(visibleLiveRunIds[0]!);
  }

  const createView = (tui: TUI, theme: Theme, keybindings: KeybindingsManager, done: () => void) => {
    let agentView: AgentView | undefined;
    let focused = false;
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
    const unsubscribeSpawns = runner.subscribeSpawns(() => {
      resubscribe();
      rerender();
    });

    const disposeAgentView = () => {
      agentView?.dispose();
      agentView = undefined;
    };
    const reportError = (message: string) => {
      if (agentView) agentView.showError(message);
      else ctx.ui.notify(message, "error");
      reportDiagnostic(`[subagent-workflow] ${message}`);
    };
    const openAgentView = () => {
      disposeAgentView();
      if (state.runId && state.childId) {
        const sourceRunId = state.runId;
        const sourceChildId = state.childId;
        agentView = buildAgentView(
          runner,
          { tui, theme, keybindings, cwd: ctx.cwd },
          model,
          sourceRunId,
          sourceChildId,
          reportError,
          services.followUp
            ? {
                onMessage: (text) => {
                  let target: { runId: string; childId: string };
                  try {
                    target = services.followUp!.send(sourceRunId, sourceChildId, text, ctx);
                  } catch (error) {
                    reportError(
                      `Could not message agent ${sanitizeTerminalText(sourceChildId)}: ${sanitizeTerminalText(errorMessage(error))}`,
                    );
                    return false;
                  }

                  try {
                    state.filter = "all";
                    const children = orderedChildren(model.detail(target.runId), state.filter);
                    const childIndex = children.findIndex((child) => child.id === target.childId);
                    if (childIndex < 0)
                      throw new Error(`spawned child ${target.childId} is missing from run ${target.runId}`);
                    state.switchRun(target.runId);
                    state.setChildCursor(childIndex, children);
                    if (state.drill(model) !== "agent")
                      throw new Error(`spawned child ${target.childId} could not be opened`);
                    openAgentView();
                  } catch (error) {
                    reportError(
                      `Follow-up run ${sanitizeTerminalText(target.runId)} was started, but could not be opened: ${sanitizeTerminalText(errorMessage(error))}`,
                    );
                  }
                  return true;
                },
              }
            : undefined,
        );
        agentView.focused = focused;
      }
    };
    if (state.level === "agent") openAgentView();

    const cleanup = () => {
      unsubscribeSpawns();
      clearSubscriptions();
      disposeAgentView();
      if (timer) clearInterval(timer);
      timer = undefined;
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

    const actionsFor = (detail: RunDetail): RunActionAvailability =>
      runActionAvailability(
        runner.liveRunIds().includes(detail.runId),
        runner.runHandles(detail.runId).map((handle) => handle.status),
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

    const act = (data: string) => {
      const keyId = navigatorKeyId(data);
      const action = keyToAction(keyId, state.level);
      if (agentView && (agentView.handleInput(data, keyId) || action.type !== "back")) return;
      if (action.type !== "stop") stopArmedRunId = undefined;
      switch (action.type) {
        case "move":
          if (state.level === "runs") state.moveRun(action.delta, model.runs());
          else if (state.level === "run" && state.runId)
            state.moveChild(action.delta, orderedChildren(model.detail(state.runId), state.filter));
          else state.move(action.delta, currentCount());
          state.scroll = undefined;
          break;
        case "scroll":
          if (state.level === "runs") {
            const runs = model.runs();
            state.reconcileRuns(runs);
            state.scroll = scrollRunList(
              runs.length,
              state.cursor,
              action.amount,
              navigatorRowBudget(tui) - 3,
              state.scroll,
            );
          } else if (state.level === "run" && state.runId) {
            const detail = model.detail(state.runId);
            state.reconcileChildren(orderedChildren(detail, state.filter));
            state.scroll = scrollRunDetail(
              detail,
              state.cursor,
              state.filter,
              action.amount,
              navigatorRowBudget(tui) - 3,
              state.scroll,
            );
          }
          break;
        case "pageMove":
          if (state.level === "runs") {
            state.pageMoveRun(action.delta, model.runs(), navigationPageSize(tui));
            state.scroll = undefined;
          } else if (state.level === "run" && state.runId) {
            const detail = model.detail(state.runId);
            const children = orderedChildren(detail, state.filter);
            state.reconcileChildren(children);
            const page = pageRunDetail(
              detail,
              state.cursor,
              state.filter,
              action.delta,
              navigationPageSize(tui),
              state.scroll,
            );
            state.setChildCursor(page.cursor, children);
            state.scroll = page.row;
          } else state.pageMove(action.delta, currentCount(), navigationPageSize(tui));
          break;
        case "drill":
          if (state.drill(model) === "agent") openAgentView();
          break;
        case "back":
          disposeAgentView();
          if (!state.back()) return finish();
          break;
        case "filter":
          state.filter = cycleFilter(state.filter);
          if (state.level === "run" && state.runId)
            state.reconcileChildren(orderedChildren(model.detail(state.runId), state.filter));
          else state.clampCursor(currentCount());
          state.scroll = undefined;
          break;
        case "stop":
          void stopRun();
          return;
        default:
          return;
      }
      rerender();
    };

    const updateSpinner = (hasSpinner: boolean) => {
      if (hasSpinner && !timer) timer = setInterval(rerender, 100);
      if (!hasSpinner && timer) {
        clearInterval(timer);
        timer = undefined;
      }
    };

    return {
      get focused() {
        return focused;
      },
      set focused(value: boolean) {
        focused = value;
        if (agentView) agentView.focused = value;
      },
      render: (width: number) => {
        const maxTotal = navigatorRowBudget(tui);
        const inner = Math.max(1, width - 2 * BOX_INSET_X);
        const bodyRows = maxTotal - 3;
        const content = agentView
          ? agentView.render(inner, bodyRows)
          : renderContent(runner, state, model, theme, inner, bodyRows, actionsFor, stopArmedRunId);
        updateSpinner(content.hasSpinner);
        return boxLines(content.lines, content.footer, inner, bodyRows, theme);
      },
      handleInput: act,
      handleMouse: (event: TuiMouseEvent) => {
        const x = event.x - BOX_INSET_X;
        const width = event.width - 2 * BOX_INSET_X;
        if (!agentView || x < 0 || x >= width) return undefined;
        return agentView.handleMouse({ ...event, x, y: event.y - 1, width, height: event.height - 2 });
      },
      invalidate: () => {
        agentView?.invalidate();
      },
      dispose: () => cleanup(),
    };
  };

  return ctx.ui.custom<void>(createView, { overlay: true, overlayOptions: { width: "100%" } });
}

interface RenderedContent {
  lines: string[];
  footer: string;
  hasSpinner: boolean;
}

function renderContent(
  runner: SubagentRunner,
  state: NavigatorState,
  model: NavigatorModel,
  theme: ThemeLike,
  inner: number,
  budget: number,
  actionsFor: (detail: RunDetail) => RunActionAvailability,
  stopArmedRunId?: string,
): RenderedContent {
  const now = Date.now();
  if (state.level === "run" && state.runId) {
    const detail = model.detail(state.runId);
    const actions = actionsFor(detail);
    const liveContextUsage = new Map<string, { tokens: number | null; contextWindow: number }>();
    for (const handle of runner.runHandles(detail.runId)) {
      const usage = handle.contextUsage;
      if (usage) liveContextUsage.set(handle.id, { tokens: usage.tokens, contextWindow: usage.contextWindow });
    }
    // Refresh renders re-read the detail, and live rows reorder as children
    // start; pin the cursor to the selected child's identity, not its index.
    state.reconcileChildren(orderedChildren(detail, state.filter));
    return {
      lines: renderRunDetail(
        detail,
        state.cursor,
        state.filter,
        theme,
        inner,
        now,
        budget,
        state.scroll,
        liveContextUsage,
      ),
      footer: footerHint(
        {
          level: "run",
          filter: state.filter,
          ...actions,
          stopArmed: actions.canStop && stopArmedRunId === detail.runId,
        },
        theme,
      ),
      hasSpinner: detail.children.some((child) => child.status === "running"),
    };
  }
  const runs = model.runs();
  state.reconcileRuns(runs);
  const selectedRunId = state.currentRunId(runs);
  const actions = selectedRunId ? actionsFor(model.detail(selectedRunId)) : { canStop: false };
  return {
    lines: renderRunList(runs, state.cursor, theme, inner, now, budget, state.scroll),
    footer: footerHint(
      {
        level: "runs",
        ...actions,
        stopArmed: actions.canStop && stopArmedRunId === selectedRunId,
      },
      theme,
    ),
    hasSpinner: runs.some((run) => run.status === "running"),
  };
}

function navigatorKeyId(data: string): string | undefined {
  if (matchesKey(data, "shift+j")) return "shift+j";
  if (matchesKey(data, "shift+k")) return "shift+k";
  if (matchesKey(data, "shift+g")) return "shift+g";
  return (parseKey(data) ?? "").toLowerCase() || undefined;
}

function orderedLiveRunIds(runs: readonly RunSummary[], liveRunIds: readonly string[]): string[] {
  const live = new Set(liveRunIds);
  return runs.filter((run) => live.has(run.runId)).map((run) => run.runId);
}

function navigatorRowBudget(tui: TUI): number {
  return Math.max(8, Math.floor((tui.terminal?.rows ?? 24) * 0.9));
}

/** Approximate selectable rows in one navigator viewport, excluding box chrome and headers. */
function navigationPageSize(tui: TUI): number {
  return Math.max(1, navigatorRowBudget(tui) - 7);
}

interface AgentFollowUpControls {
  onMessage: (text: string) => boolean;
}

/**
 * Display-time eligibility from the projection the view already renders; the
 * follow-up resolver revalidates authoritatively when the message is sent.
 */
function isMessageableChild(child: ChildRow | undefined): boolean {
  if (!child?.sessionFile) return false;
  return child.status === "completed" || child.status === "failed" || child.status === "aborted";
}

function buildAgentView(
  runner: SubagentRunner,
  ui: { tui: TUI; theme: Theme; keybindings: KeybindingsManager; cwd: string },
  model: NavigatorModel,
  runId: string,
  childId: string,
  reportError: (message: string) => void,
  followUp?: AgentFollowUpControls,
): AgentView {
  // Share one child row across a render's header, transcript and controls.
  // The microtask reset lets the next frame observe runner updates.
  let burstChild: ChildRow | undefined;
  let burstValid = false;
  const child = (): ChildRow | undefined => {
    if (!burstValid) {
      burstChild = model.detail(runId).children.find((row) => row.id === childId);
      burstValid = true;
      queueMicrotask(() => {
        burstValid = false;
      });
    }
    return burstChild;
  };
  const header = () => {
    const row = child();
    return {
      label: row?.label ?? childId,
      model: row?.model ?? "",
      thinking: row?.thinking,
      status: row?.status ?? "pending",
      usage: row?.usage ?? EMPTY_USAGE(),
    };
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
  let cachedMessages: TranscriptMessage[] = [];
  const history = (path: string | undefined): readonly TranscriptMessage[] => {
    if (!path) {
      if (cachedPath !== undefined) {
        cachedPath = undefined;
        cachedSize = undefined;
        cachedMtimeMs = undefined;
        cachedMissing = false;
        cachedMessages = [];
      }
      return cachedMessages;
    }

    try {
      const stat = statSync(path);
      if (path === cachedPath && !cachedMissing && stat.size === cachedSize && stat.mtimeMs === cachedMtimeMs) {
        return cachedMessages;
      }
      cachedPath = path;
      cachedSize = stat.size;
      cachedMtimeMs = stat.mtimeMs;
      cachedMissing = false;
    } catch {
      if (path === cachedPath && cachedMissing) return cachedMessages;
      cachedPath = path;
      cachedSize = undefined;
      cachedMtimeMs = undefined;
      cachedMissing = true;
    }
    return (cachedMessages = readSessionMessages(path));
  };

  const runControl = (action: "steer" | "stop", invoke: (handle: SubagentHandle) => Promise<void>): void => {
    const handle = runner.get(childId);
    if (!handle) return;
    void invoke(handle).catch((error: unknown) => {
      reportError(
        `Could not ${action} agent ${sanitizeTerminalText(childId)}: ${sanitizeTerminalText(errorMessage(error))}`,
      );
    });
  };

  return new AgentView({
    tui: ui.tui,
    theme: ui.theme,
    keybindings: ui.keybindings,
    settingsCwd: ui.cwd,
    cwd: () => child()?.resolved?.cwd ?? ui.cwd,
    header,
    messages: () => {
      const live = session();
      return { history: history(live?.sessionFile ?? child()?.sessionFile), current: live?.currentAssistant };
    },
    result: () => child()?.result,
    live: () => {
      const status = child()?.status;
      return runner.get(childId) !== undefined && (status === "running" || status === "pending");
    },
    onSteer: (text) => runControl("steer", (handle) => handle.steer(text)),
    canMessage: () => !!followUp && isMessageableChild(child()),
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

const BORDER = "muted";
const BOX_INSET_X = 2;

/** Wrap content in a stable-height titled box, padded to `inner` width. */
export function boxLines(
  content: string[],
  footer: string,
  inner: number,
  bodyRows: number,
  theme: ThemeLike,
): string[] {
  const border = (text: string) => theme.fg(BORDER, text);
  const pad = (line: string) => `${border("│")} ${truncateToWidth(line, inner, "", true)} ${border("│")}`;
  const title = " agents ".slice(0, inner + 1);
  const top = border(`╭─${theme.fg("dim", title)}${"─".repeat(inner + 1 - title.length)}╮`);
  const bottom = border(`╰${"─".repeat(inner + 2)}╯`);
  const rows = content.slice(0, bodyRows).map(pad);
  while (rows.length < bodyRows) rows.push(pad(""));
  rows.push(pad(footer));
  return [top, ...rows, bottom];
}

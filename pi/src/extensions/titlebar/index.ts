import type {
  ExtensionAPI,
  ExtensionContext,
} from "@earendil-works/pi-coding-agent";
import {
  type BusyPhase,
  type TitleContext,
  baseTitle,
  busyTitle,
} from "./title.js";

/** Spinner cadence, matching the titlebar spinner example shipped with Pi. */
const FRAME_INTERVAL_MS = 80;

/**
 * Mirror the agent's live state into the terminal title: an animated `model` or
 * `tool` marker while Pi works, and Pi's own `π - <session> - <dir>` title once
 * the agent settles.
 */
export function registerTitlebar(pi: ExtensionAPI): void {
  let phase: BusyPhase | null = null;
  let frameIndex = 0;
  let timer: ReturnType<typeof setInterval> | null = null;
  let paintedContext: ExtensionContext | null = null;

  function titleContext(ctx: ExtensionContext): TitleContext {
    return { sessionName: pi.getSessionName(), cwd: ctx.cwd };
  }

  /** Advance the marker of whichever phase is currently running. */
  function animate(): void {
    const ctx = paintedContext;
    if (phase === null || ctx === null) return;
    frameIndex += 1;
    ctx.ui.setTitle(busyTitle(phase, frameIndex, titleContext(ctx)));
  }

  function showIdle(ctx: ExtensionContext): void {
    if (ctx.mode !== "tui") return;
    phase = null;
    frameIndex = 0;
    paintedContext = null;
    if (timer !== null) {
      clearInterval(timer);
      timer = null;
    }
    ctx.ui.setTitle(baseTitle(titleContext(ctx)));
  }

  function showBusy(ctx: ExtensionContext, next: BusyPhase): void {
    if (ctx.mode !== "tui") return;
    // A phase switch must not restart the animation, so only a fresh run resets it.
    if (phase === null) frameIndex = 0;
    phase = next;
    paintedContext = ctx;
    ctx.ui.setTitle(busyTitle(next, frameIndex, titleContext(ctx)));
    timer ??= setInterval(animate, FRAME_INTERVAL_MS);
  }

  // One user message runs several turns: each turn streams an assistant reply,
  // and every tool call in between hands the agent back to the model.
  pi.on("agent_start", (_event, ctx) => showBusy(ctx, "model"));
  pi.on("turn_start", (_event, ctx) => showBusy(ctx, "model"));
  pi.on("tool_execution_start", (_event, ctx) => showBusy(ctx, "tool"));
  pi.on("agent_settled", (_event, ctx) => {
    // Another extension may already have started the next run.
    if (ctx.isIdle() && !ctx.hasPendingMessages()) showIdle(ctx);
  });

  // Pi rewrites its own title on these events, so a busy marker has to be re-asserted.
  pi.on("session_info_changed", (_event, ctx) => {
    if (phase !== null) showBusy(ctx, phase);
  });
  pi.on("session_start", (_event, ctx) => showIdle(ctx));
  pi.on("session_shutdown", (_event, ctx) => showIdle(ctx));
}

export default registerTitlebar;

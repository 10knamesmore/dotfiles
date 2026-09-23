import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { EditorActivity } from "../editor/api.js";
import { SESSION_ACTIVITY_CHANGED } from "../footer/activity.js";
import { baseTitle, busyTitle, type TitleContext } from "./title.js";

const FRAME_INTERVAL_MS = 80;

/** Show the editor's activity in the terminal title, restoring Pi's title when ready. */
export function registerTitlebar(pi: ExtensionAPI): void {
  let activity: EditorActivity = { kind: "ready" };
  let context: ExtensionContext | undefined;
  let frameIndex = 0;
  let timer: ReturnType<typeof setInterval> | undefined;

  function titleContext(ctx: ExtensionContext): TitleContext {
    return { sessionName: pi.getSessionName(), cwd: ctx.cwd };
  }

  function render(): void {
    if (context?.mode !== "tui") return;
    const title = titleContext(context);
    context.ui.setTitle(activity.kind === "ready" ? baseTitle(title) : busyTitle(activity, frameIndex, title));
  }

  function updateActivity(next: EditorActivity): void {
    activity = next;
    if (activity.kind === "ready") {
      if (timer !== undefined) clearInterval(timer);
      timer = undefined;
      frameIndex = 0;
    } else if (timer === undefined && context?.mode === "tui") {
      // Phase changes keep the animation running; only a new busy period resets it.
      frameIndex = 0;
      timer = setInterval(() => {
        frameIndex += 1;
        render();
      }, FRAME_INTERVAL_MS);
    }
    render();
  }

  pi.events.on(SESSION_ACTIVITY_CHANGED, (next) => {
    if (context?.mode === "tui") updateActivity(next as EditorActivity);
  });
  pi.on("session_start", (_event, ctx) => {
    context = ctx;
    updateActivity({ kind: "ready" });
  });
  pi.on("session_info_changed", (_event, ctx) => {
    context = ctx;
    render();
  });
  pi.on("session_shutdown", () => {
    updateActivity({ kind: "ready" });
    context = undefined;
  });
}

export default registerTitlebar;

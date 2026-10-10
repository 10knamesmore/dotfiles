import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { EditorActivity } from "../editor/api.js";
import { SESSION_ACTIVITY_CHANGED } from "../footer/activity.js";
import { baseTitle, busyTitle, type TitleContext } from "./title.js";

/**
 * Show the editor's activity in the terminal title, restoring Pi's title when ready.
 *
 * The footer publishes one activity event per animation frame; following those
 * events keeps the title on the same clock as the footer badge instead of
 * running a second spinner timer here.
 */
export function registerTitlebar(pi: ExtensionAPI): void {
  let activity: EditorActivity = { kind: "ready" };
  let context: ExtensionContext | undefined;
  let renderedTitle: string | undefined;

  function titleContext(ctx: ExtensionContext): TitleContext {
    return { sessionName: pi.getSessionName(), cwd: ctx.cwd };
  }

  function render(): void {
    if (context?.mode !== "tui") return;
    const title = titleContext(context);
    const next = activity.kind === "ready" ? baseTitle(title) : busyTitle(activity, title);
    if (next === renderedTitle) return;
    renderedTitle = next;
    context.ui.setTitle(next);
  }

  pi.events.on(SESSION_ACTIVITY_CHANGED, (next) => {
    if (context?.mode !== "tui") return;
    activity = next as EditorActivity;
    render();
  });
  pi.on("session_start", (_event, ctx) => {
    context = ctx;
    activity = { kind: "ready" };
    renderedTitle = undefined;
    render();
  });
  pi.on("session_info_changed", (_event, ctx) => {
    context = ctx;
    render();
  });
  pi.on("session_shutdown", () => {
    activity = { kind: "ready" };
    render();
    context = undefined;
    renderedTitle = undefined;
  });
}

export default registerTitlebar;

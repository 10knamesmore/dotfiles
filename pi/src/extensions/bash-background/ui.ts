import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { truncateToWidth } from "@earendil-works/pi-tui";
import { PROMPT_EDITOR_CONFIGURE, type PromptEditorApi } from "../editor/api.js";
import { sanitizeFooterText } from "../footer/format.js";
import { openBackgroundBrowser } from "./browser.js";
import { runningProcesses, subscribeToProcesses, type BackgroundSession } from "./model.js";

const STATUS_KEY = "bash-background";

/** Footer status, editor navigation, and the /backgrounds command share the same process snapshot. */
export function registerBackgroundUi(
  pi: ExtensionAPI,
  session: () => BackgroundSession,
  stop: (pid: number) => void,
): void {
  let ctx: ExtensionContext | undefined;
  let selected = false;
  let lastStatus: string | undefined;
  let unsubscribe: (() => void) | undefined;
  let browserOpen = false;

  const publishStatus = () => {
    if (!ctx?.hasUI) return;
    const running = runningProcesses(session());
    if (running.length === 0) selected = false;
    const command = running.length === 1 ? truncateToWidth(sanitizeFooterText(running[0]!.command), 48, "…") : "";
    const label = running.length === 1 ? command : `bash: ${running.length}`;
    const status = running.length === 0 ? undefined : `${selected ? "❯" : "●"} ${label} ↵`;
    if (status === lastStatus) return;
    ctx.ui.setStatus(STATUS_KEY, status);
    lastStatus = status;
  };

  const clearSelection = () => {
    if (!selected) return;
    selected = false;
    publishStatus();
  };

  const open = async () => {
    if (!ctx || browserOpen || ctx.mode !== "tui") return;
    browserOpen = true;
    try {
      await openBackgroundBrowser(ctx, session(), { stop });
    } finally {
      browserOpen = false;
    }
  };

  pi.events.on(PROMPT_EDITOR_CONFIGURE, (requested) => {
    (requested as PromptEditorApi).useFooterNavigation({
      selectNext() {
        if (runningProcesses(session()).length === 0) return false;
        selected = true;
        publishStatus();
        return true;
      },
      selectPrevious: clearSelection,
      hasSelection: () => selected,
      openSelection() {
        if (!selected || !ctx) return false;
        clearSelection();
        void open().catch((error: unknown) => {
          const message = error instanceof Error ? error.message : String(error);
          ctx?.ui.notify(`Could not open background output: ${sanitizeFooterText(message)}`, "error");
        });
        return true;
      },
      clearSelection,
    });
  });
  pi.registerCommand("backgrounds", {
    description: "Browse live output from background bash commands",
    handler: async (_args, commandContext) => {
      ctx = commandContext;
      await open();
    },
  });
  pi.on("session_start", (_event, sessionContext) => {
    ctx = sessionContext;
    selected = false;
    lastStatus = undefined;
    unsubscribe?.();
    unsubscribe = subscribeToProcesses(session(), publishStatus);
    publishStatus();
  });
  pi.on("session_shutdown", () => {
    unsubscribe?.();
    unsubscribe = undefined;
    selected = false;
    if (ctx && lastStatus !== undefined) ctx.ui.setStatus(STATUS_KEY, undefined);
    ctx = undefined;
    lastStatus = undefined;
  });
}

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { sendSystemNotification } from "./backend.js";
import { createSettledNotificationMessage } from "./message.js";

/** Marker written only into subprocesses launched by the vendored workflow runtime. */
const SUBAGENT_CHILD_MARKER = "PI_SUBAGENT_SHIM_SPEC";

/** Register one settled notification per completed top-level TUI run sequence. */
export function registerSettledNotification(pi: ExtensionAPI): void {
  if (process.env[SUBAGENT_CHILD_MARKER] !== undefined) return;

  let armed = false;
  pi.on("agent_start", (_event, ctx) => {
    armed = ctx.mode === "tui";
  });
  pi.on("agent_settled", (_event, ctx) => {
    if (!armed || ctx.mode !== "tui" || !ctx.isIdle() || ctx.hasPendingMessages()) return;
    armed = false;
    const message = createSettledNotificationMessage(ctx.cwd);

    // Delivery owns a bounded child-process lifetime; Pi must not await the OS notification service.
    void sendSystemNotification(message);
  });
}

export default registerSettledNotification;

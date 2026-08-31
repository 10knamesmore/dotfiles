import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import {
  type NotificationFailureKind,
  sendSystemNotification,
} from "./backend.js";
import {
  createSettledNotificationMessage,
  sanitizeSingleLine,
} from "./message.js";

/** Marker written only into subprocesses launched by the vendored workflow runtime. */
const SUBAGENT_CHILD_MARKER = "PI_SUBAGENT_SHIM_SPEC";

const reportedFailures = new Set<NotificationFailureKind>();

/** Register one settled notification per completed top-level TUI run sequence. */
export function registerSettledNotification(pi: ExtensionAPI): void {
  if (process.env[SUBAGENT_CHILD_MARKER] !== undefined) return;

  let armed = false;
  pi.on("agent_start", (_event, ctx) => {
    armed = ctx.mode === "tui";
  });
  pi.on("agent_settled", (_event, ctx) => {
    if (
      !armed ||
      ctx.mode !== "tui" ||
      !ctx.isIdle() ||
      ctx.hasPendingMessages()
    )
      return;
    armed = false;
    const message = createSettledNotificationMessage(ctx.cwd);

    // Delivery owns a bounded child-process lifetime; Pi must not await the OS notification service.
    void sendSystemNotification(message).then(
      (delivery) => {
        if (delivery.status === "failed")
          reportOnce(delivery.kind, delivery.reason);
      },
      (error: unknown) => reportOnce("unexpected", errorMessage(error)),
    );
  });
}

export default registerSettledNotification;

/** Record each backend failure class once without mutating Pi session state. */
function reportOnce(kind: NotificationFailureKind, reason: string): void {
  if (reportedFailures.has(kind)) return;
  reportedFailures.add(kind);
  console.error(`[pi-distribution] notification ${kind} failed: ${reason}`);
}

/** Convert an unexpected rejection into a bounded one-line diagnostic. */
function errorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return sanitizeSingleLine(message).slice(0, 500) || "unknown error";
}

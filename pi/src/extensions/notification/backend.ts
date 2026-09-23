import { constants } from "node:fs";
import { access } from "node:fs/promises";
import {
  sanitizeSingleLine,
  type SettledNotificationMessage,
} from "./message.js";
import { isDirectKittyTerminal, sendKittyNotification } from "./kitty.js";
import { findExecutable, runNotificationProcess } from "./process.js";

const OSASCRIPT_PATH = "/usr/bin/osascript";
const DISPLAY_NOTIFICATION_SCRIPT = [
  "function run(argv) {",
  "if (argv.length !== 2) throw new Error('expected title and body');",
  "var app = Application.currentApplication();",
  "app.includeStandardAdditions = true;",
  "app.displayNotification(argv[1], { withTitle: argv[0] });",
  "}",
].join("\n");

export type NotificationBackendName =
  | "kitty"
  | "terminal-notifier"
  | "osascript"
  | "notify-send";
export type NotificationFailureKind =
  | NotificationBackendName
  | "unavailable"
  | "unexpected";

/** Result of one best-effort delivery attempt. */
export type NotificationDelivery =
  | {
      /** The selected backend completed without reporting an error. */
      status: "sent";

      /** Backend that completed the delivery attempt. */
      backend: NotificationBackendName;
    }
  | {
      /** No notification was delivered. */
      status: "failed";

      /** Stable failure class used to bound diagnostics. */
      kind: NotificationFailureKind;

      /** Human-readable failure without command output or notification contents. */
      reason: string;
    };

interface NotificationBackend {
  /** Stable backend identity used in diagnostics and smoke evidence. */
  name: NotificationBackendName;

  /** Deliver one already-sanitized notification. */
  send(message: SettledNotificationMessage): Promise<void>;
}

let selectedBackend: Promise<NotificationBackend | undefined> | undefined;

/** Deliver through the first supported backend without throwing into Pi lifecycle code. */
export async function sendSystemNotification(
  message: SettledNotificationMessage,
): Promise<NotificationDelivery> {
  try {
    selectedBackend ??= selectBackend();
    const backend = await selectedBackend;
    if (backend === undefined) {
      return {
        status: "failed",
        kind: "unavailable",
        reason: "no supported notification backend is available",
      };
    }

    try {
      await backend.send(message);
      return { status: "sent", backend: backend.name };
    } catch (error: unknown) {
      return {
        status: "failed",
        kind: backend.name,
        reason: errorMessage(error),
      };
    }
  } catch (error: unknown) {
    return {
      status: "failed",
      kind: "unexpected",
      reason: errorMessage(error),
    };
  }
}

/** Select once per extension lifetime; platform capabilities do not change during a session. */
async function selectBackend(): Promise<NotificationBackend | undefined> {
  if (isDirectKittyTerminal()) {
    return { name: "kitty", send: sendKittyNotification };
  }

  if (process.platform === "darwin") {
    const terminalNotifier = await findExecutable("terminal-notifier");
    if (terminalNotifier !== undefined) {
      return {
        name: "terminal-notifier",
        send: (message) =>
          runNotificationProcess(terminalNotifier, [
            "-title",
            message.title,
            "-message",
            message.body,
            "-group",
            `pi-${process.pid}-settled`,
          ]),
      };
    }
    try {
      await access(OSASCRIPT_PATH, constants.X_OK);
    } catch {
      return undefined;
    }
    return {
      name: "osascript",
      send: (message) =>
        runNotificationProcess(OSASCRIPT_PATH, [
          "-l",
          "JavaScript",
          "-e",
          DISPLAY_NOTIFICATION_SCRIPT,
          message.title,
          message.body,
        ]),
    };
  }

  if (process.platform === "linux") {
    const notifySend = await findExecutable("notify-send");
    if (notifySend !== undefined) {
      return {
        name: "notify-send",
        send: (message) =>
          runNotificationProcess(notifySend, [
            "--app-name=Pi",
            message.title,
            message.body,
          ]),
      };
    }
  }

  return undefined;
}

/** Convert unknown thrown values into a bounded single-line diagnostic. */
function errorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return sanitizeSingleLine(message).slice(0, 500) || "unknown error";
}

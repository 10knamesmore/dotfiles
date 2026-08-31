import type { SettledNotificationMessage } from "./message.js";

const OSC = "\u001b]";
const STRING_TERMINATOR = "\u001b\\";

/** Whether stdout reaches Kitty directly rather than through a multiplexer. */
export function isDirectKittyTerminal(
  environment: NodeJS.ProcessEnv = process.env,
  output: NodeJS.WriteStream = process.stdout,
): boolean {
  if (output.isTTY !== true || environment.KITTY_WINDOW_ID === undefined)
    return false;
  return (
    environment.TMUX === undefined &&
    environment.ZELLIJ === undefined &&
    environment.ZELLIJ_SESSION_NAME === undefined
  );
}

/** Encode title and body as OSC 99 Base64 payloads with no user-controlled metadata. */
export function encodeKittyNotification(
  message: SettledNotificationMessage,
): string {
  const id = `pi-${process.pid}-settled`;
  const title = Buffer.from(message.title, "utf8").toString("base64");
  const body = Buffer.from(message.body, "utf8").toString("base64");
  return (
    `${OSC}99;i=${id}:d=0:e=1:p=title;${title}${STRING_TERMINATOR}` +
    `${OSC}99;i=${id}:d=1:e=1:p=body;${body}${STRING_TERMINATOR}`
  );
}

/** Write one complete Kitty notification and resolve after stdout accepts it. */
export function sendKittyNotification(
  message: SettledNotificationMessage,
): Promise<void> {
  const payload = encodeKittyNotification(message);
  return new Promise((resolve, reject) => {
    try {
      process.stdout.write(payload, (error) => {
        if (error) reject(error);
        else resolve();
      });
    } catch (error: unknown) {
      reject(error instanceof Error ? error : new Error(String(error)));
    }
  });
}

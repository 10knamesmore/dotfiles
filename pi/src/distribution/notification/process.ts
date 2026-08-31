import { spawn } from "node:child_process";
import { constants } from "node:fs";
import { access } from "node:fs/promises";
import { delimiter, isAbsolute, join } from "node:path";

const PROCESS_TIMEOUT_MS = 5_000;

/** Locate an executable through absolute PATH entries without invoking a shell. */
export async function findExecutable(
  name: string,
): Promise<string | undefined> {
  for (const directory of (process.env.PATH ?? "").split(delimiter)) {
    if (!isAbsolute(directory)) continue;
    const candidate = join(directory, name);
    try {
      await access(candidate, constants.X_OK);
      return candidate;
    } catch {}
  }
  return undefined;
}

/** Run a bounded notification command with fixed argv semantics and no shell. */
export function runNotificationProcess(
  command: string,
  args: readonly string[],
): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      shell: false,
      stdio: "ignore",
      windowsHide: true,
    });
    child.unref();
    let settled = false;
    let timeout: NodeJS.Timeout | undefined;

    const finish = (error?: Error): void => {
      if (settled) return;
      settled = true;
      if (timeout !== undefined) clearTimeout(timeout);
      if (error) reject(error);
      else resolve();
    };

    timeout = setTimeout(() => {
      try {
        child.kill("SIGKILL");
      } finally {
        finish(
          new Error(`notification process exceeded ${PROCESS_TIMEOUT_MS}ms`),
        );
      }
    }, PROCESS_TIMEOUT_MS);
    timeout.unref();
    child.on("error", (error: Error) => finish(error));
    child.on("close", (code, signal) => {
      if (code === 0) finish();
      else
        finish(
          new Error(
            `notification process exited with code=${String(code)} signal=${String(signal)}`,
          ),
        );
    });
  });
}

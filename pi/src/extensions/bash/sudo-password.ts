import { execFileSync } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rmdir } from "node:fs/promises";
import { createServer, type Socket } from "node:net";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import { Key, matchesKey, wrapTextWithAnsi } from "@earendil-works/pi-tui";
import { visibleCommand } from "./sudo-approval.js";

type RunWithAskpass<T> = (env: NodeJS.ProcessEnv, signal: AbortSignal) => Promise<T>;

/** Keep the chat editor out of focus while sudo starts and between password attempts. */
export async function withAskpass<T>(
  ctx: ExtensionContext,
  command: string,
  cwd: string,
  signal: AbortSignal,
  run: RunWithAskpass<T>,
): Promise<T> {
  const interrupt = new AbortController();
  const stop = AbortSignal.any([signal, interrupt.signal]);
  const cancel = () => interrupt.abort(new Error("aborted"));
  const outcome = await ctx.ui.custom<{ value: T } | { error: unknown }>((_tui, theme, _keys, done) => {
    void servePasswordPrompts(ctx, command, cwd, stop, run).then(
      (value) => done({ value }),
      (error: unknown) => done({ error }),
    );
    return {
      render: (width) =>
        wrapTextWithAnsi(`Running sudo · ${visibleCommand(cwd)}   ${theme.fg("dim", "Esc Cancel")}`, width),
      handleInput(key) {
        if (matchesKey(key, Key.escape) || matchesKey(key, Key.ctrl("c"))) cancel();
      },
      invalidate() {},
      dispose: cancel,
    };
  });
  if ("error" in outcome) throw outcome.error;
  return outcome.value;
}

/** Each askpass invocation gets a new field, including sudo's retries. NOPASSWD never opens one. */
async function servePasswordPrompts<T>(
  ctx: ExtensionContext,
  command: string,
  cwd: string,
  signal: AbortSignal,
  run: RunWithAskpass<T>,
): Promise<T> {
  const tty = execFileSync("/usr/bin/tty", { stdio: ["inherit", "pipe", "pipe"], encoding: "utf8" }).trimEnd();
  const directory = await mkdtemp("/tmp/pi-askpass-");
  const path = join(directory, "prompt");
  const cancelled = new AbortController();
  const runSignal = AbortSignal.any([signal, cancelled.signal]);
  let prompt = Promise.resolve();
  let attempt = 0;
  const server = createServer((socket) => {
    // sudo waits for the helper to exit before retrying. Restore Pi's reader before handing it off again.
    prompt = prompt
      .then(() =>
        passwordPrompt(ctx, command, cwd, socket, ++attempt, runSignal, () =>
          cancelled.abort(new Error("Authentication cancelled. The command did not run.")),
        ),
      )
      .catch((error: unknown) => {
        socket.destroy();
        cancelled.abort(error);
      });
  });
  try {
    server.listen(path);
    await once(server, "listening");
    return await run(
      {
        SUDO_ASKPASS: fileURLToPath(new URL("./askpass", import.meta.url)),
        PI_ASKPASS_NODE: process.execPath,
        PI_ASKPASS_SOCKET: path,
        PI_ASKPASS_TTY: tty,
      },
      runSignal,
    );
  } finally {
    cancelled.abort();
    await new Promise<void>((resolve) => server.close(() => resolve()));
    await prompt;
    await rmdir(directory);
  }
}

async function passwordPrompt(
  ctx: ExtensionContext,
  command: string,
  cwd: string,
  socket: Socket,
  attempt: number,
  signal: AbortSignal,
  cancel: () => void,
): Promise<void> {
  if (signal.aborted || socket.destroyed) {
    socket.destroy();
    return;
  }
  const closed = new Promise<void>((resolve) => socket.once("close", () => resolve()));
  const stopReader = () => socket.end();
  socket.on("error", cancel);
  signal.addEventListener("abort", stopReader, { once: true });
  let paused = false;
  try {
    await ctx.ui.custom<void>(
      (tui, theme, _keys, done) => {
        if (!process.stdin.isRaw) throw new Error("Password input needs a terminal with echo disabled.");
        process.stdin.pause();
        paused = true;
        let count = 0;
        const messages = createInterface({ input: socket });
        messages.on("error", cancel);
        messages.on("line", (line) => {
          if (line === "cancel") return cancel();
          if (/^\d+$/.test(line)) {
            count = Number(line);
            tui.requestRender();
          }
        });
        void closed.then(() => done());
        // Node's stdin pause stops the underlying tty read on the next tick.
        // Only then may askpass start reading from the same terminal.
        setImmediate(() => {
          if (!signal.aborted && !socket.destroyed) socket.write(tui.terminal.kittyProtocolActive ? "K" : "L");
        });
        return {
          render(width) {
            const border = theme.fg("border", "─".repeat(width));
            const preview = wrapTextWithAnsi(visibleCommand(command), width);
            const height = Math.max(1, Math.floor(tui.terminal.rows / 3));
            return [
              border,
              ...wrapTextWithAnsi(`Run once as ${theme.fg("warning", "root")} · ${visibleCommand(cwd)}`, width),
              "",
              ...preview.slice(0, height),
              ...(preview.length > height ? [theme.fg("dim", "…")] : []),
              "",
              ...wrapTextWithAnsi(
                `${attempt > 1 ? "Retry password" : "Password"}: ${"•".repeat(Math.min(count, width))}▏`,
                width,
              ),
              ...wrapTextWithAnsi(theme.fg("dim", "Enter Submit   Esc Cancel"), width),
              border,
            ];
          },
          invalidate() {},
          dispose() {
            messages.close();
            stopReader();
          },
        };
      },
      { blocked: { kind: "auth", message: "Enter your sudo password" } },
    );
  } finally {
    stopReader();
    await closed;
    signal.removeEventListener("abort", stopReader);
    if (paused) process.stdin.resume();
  }
}

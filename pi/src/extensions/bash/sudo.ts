import { spawn } from "node:child_process";
import { constants } from "node:os";
import type { BashOperations, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { approveRootCommand } from "./sudo-approval.js";
import { withAskpass } from "./sudo-password.js";

// Only one approval or password prompt may own the keyboard at a time.
let terminalAvailable = Promise.resolve();

export function sudoBashOperations(ctx: ExtensionContext, timeoutMs: number): BashOperations {
  return {
    exec(command, cwd, { onData, signal }) {
      const run = terminalAvailable.then(async () => {
        if (signal?.aborted) throw new Error("aborted");
        const approved = await approveRootCommand(ctx, command, cwd, signal);
        if (signal?.aborted) throw new Error("aborted");
        if (!approved) throw new Error("Denied. The command did not run.");

        const deadline = AbortSignal.timeout(Math.ceil(timeoutMs));
        const stop = signal ? AbortSignal.any([signal, deadline]) : deadline;
        try {
          return await withAskpass(ctx, command, cwd, stop, (askpass, runSignal) =>
            runSudo(command, cwd, askpass, onData, runSignal),
          );
        } finally {
          if (signal?.aborted) throw new Error("aborted");
          if (deadline.aborted) throw new Error(`timeout:${timeoutMs / 1000}`);
        }
      });
      terminalAvailable = run.then(
        () => {},
        () => {},
      );
      return run;
    },
  };
}

async function runSudo(
  command: string,
  cwd: string,
  askpass: NodeJS.ProcessEnv,
  onData: (chunk: Buffer) => void,
  signal: AbortSignal,
): Promise<{ exitCode: number }> {
  signal.throwIfAborted();
  const child = spawn(
    "/usr/bin/sudo",
    ["-A", "-k", "-u", "root", "--", "/bin/bash", "--noprofile", "--norc", "-c", command],
    {
      cwd,
      // The approved command has no controlling terminal. Only askpass opens Pi's keyboard.
      detached: true,
      env: { PATH: "/usr/sbin:/usr/bin:/sbin:/bin", TERM: process.env.TERM, LANG: process.env.LANG, ...askpass },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  child.stdout.on("data", onData);
  child.stderr.on("data", onData);
  const stop = () => {
    if (child.exitCode === null && child.signalCode === null && !child.kill("SIGTERM")) return;
    child.stdout.destroy();
    child.stderr.destroy();
  };
  signal.addEventListener("abort", stop, { once: true });
  try {
    const exitCode = await new Promise<number>((resolve, reject) => {
      child.on("error", (error) => {
        if (!child.pid) reject(error);
        // Failing to signal a running sudo process does not mean the command stopped.
        else onData(Buffer.from(`\nCould not signal sudo: ${error.message}. Waiting for it to exit.\n`));
      });
      child.once("close", (code, sig) => resolve(code ?? (sig ? 128 + constants.signals[sig] : 1)));
    });
    signal.throwIfAborted();
    return { exitCode };
  } finally {
    signal.removeEventListener("abort", stop);
  }
}

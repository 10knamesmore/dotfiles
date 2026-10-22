/**
 * Hosts background `bash` commands.
 *
 * Pi's built-in `bash` tool waits for every command, so a dev server or a
 * watcher holds the turn until it exits. This module re-registers `bash` with
 * one extra parameter, `background`. With `background: true` the command starts
 * detached, stdout and stderr go to a log file, and the call returns the pid
 * immediately instead of waiting. Exit notifications steer the session or
 * start a new turn when it is idle.
 *
 * Foreground commands default to 120 seconds and otherwise use Pi's own tool
 * definition. Background commands have no default timeout and swap only process
 * spawning by handing `operations` to that definition. Session
 * environment variables, `shellCommandPrefix`, and `shellPath` therefore keep
 * coming from Pi.
 *
 * Background processes belong to the session: `session_shutdown` kills their
 * process groups.
 */

import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { closeSync, openSync, readFileSync } from "node:fs";
import { appendFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { stripVTControlCharacters } from "node:util";
import {
  type BashOperations,
  type BashToolDetails,
  CONFIG_DIR_NAME,
  createBashToolDefinition,
  type ExtensionAPI,
  type ExtensionContext,
  getAgentDir,
  getShellConfig,
} from "@earendil-works/pi-coding-agent";
import { Box, Text, TruncatedText } from "@earendil-works/pi-tui";
import { Type } from "typebox";
import { createExpandableMessageRenderer } from "@dotfiles/pi-ui/expandable-message";

const DEFAULT_FOREGROUND_TIMEOUT_SECONDS = 120;
const BACKGROUND_EXIT_MESSAGE = "bash-background:exit";

/** Pi keeps its schema private; declare the foreground deadline and background lifetime here. */
const bashSchema = Type.Object({
  command: Type.String({ description: "Shell command to execute" }),
  timeout: Type.Optional(
    Type.Number({
      description: `Timeout in seconds. Foreground commands default to ${DEFAULT_FOREGROUND_TIMEOUT_SECONDS} seconds; an explicit value overrides it. Background commands have no default timeout; an explicit timeout limits their lifetime, and session shutdown also kills them.`,
    }),
  ),
  background: Type.Optional(
    Type.Boolean({
      description:
        "Start the command detached and return its pid instead of waiting; output goes to a log file and an exit notification triggers a turn",
    }),
  ),
});

/** A command that runs detached from the session. */
interface BackgroundProcess {
  pid: number;
  logPath: string;
}

interface BackgroundExit extends BackgroundProcess {
  code: number | null;
  signal: NodeJS.Signals | null;
  timedOut: boolean;
}

interface BackgroundExitMessage extends BackgroundExit {
  command: string;
}

interface BashBackgroundDetails extends BashToolDetails {
  background?: BackgroundProcess;
}

/** The two settings Pi forwards to its built-in `bash` tool (`core/agent-session.js:2186`). */
interface ShellToolSettings {
  shellPath?: string;
  commandPrefix?: string;
}

/** Node's timer limit, and the bound Pi's bash tool applies (`core/tools/bash.js:18`). */
const MAX_TIMEOUT_MS = 2_147_483_647;

/** Register the background-capable `bash` tool in place of Pi's built-in one. */
export function registerBashBackground(pi: ExtensionAPI): void {
  // Each launch captures its session lifetime, including while its exit log is being written.
  let lifetime = { active: true, processes: new Set<number>() };
  pi.on("session_start", () => {
    lifetime = { active: true, processes: new Set<number>() };
  });
  pi.registerMessageRenderer<BackgroundExitMessage>(
    BACKGROUND_EXIT_MESSAGE,
    createExpandableMessageRenderer<BackgroundExitMessage>((message, { expanded, outputPad }, theme) => {
      const result = message.details!;
      const color = result.timedOut || result.signal !== null ? "warning" : result.code === 0 ? "success" : "error";
      const heading = `${theme.fg("muted", expanded ? "▼" : "▶")} ${theme.bold("Background bash")} ${theme.fg("dim", "·")} ${theme.fg(color, backgroundExitStatus(result))}`;
      const command = stripVTControlCharacters(result.command);
      const box = new Box(outputPad, 1, (text) => theme.bg("customMessageBg", text));
      box.addChild(new Text(heading, 0, 0));
      box.addChild(expanded ? new Text(command, 0, 0) : new TruncatedText(command.replace(/\s+/g, " "), 0, 0));
      if (expanded) {
        box.addChild(
          new Text(
            theme.fg(
              "dim",
              `PID ${result.pid} · ${result.signal === null ? `exit ${result.code ?? "unknown"}` : result.signal}\nLog ${result.logPath}`,
            ),
            0,
            0,
          ),
        );
      }
      return box;
    }),
  );
  // Renderers, prompt metadata, and description are Pi's; only execution is ours.
  const builtInBash = createBashToolDefinition(process.cwd());

  pi.registerTool<typeof bashSchema, BashBackgroundDetails | undefined>({
    ...builtInBash,
    description: `${builtInBash.description} Foreground commands default to a ${DEFAULT_FOREGROUND_TIMEOUT_SECONDS}-second timeout; set timeout explicitly to override it. With background: true the command runs detached with no default timeout and the call returns its pid instead of waiting; use it for long-running commands such as dev servers or watchers. An exit notification arrives as a steered message and triggers a turn when idle; do not poll for completion.`,
    parameters: bashSchema,

    async execute(toolCallId, params, signal, onUpdate, ctx) {
      const settings = readShellToolSettings(ctx);
      if (params.background !== true) {
        return createBashToolDefinition(ctx.cwd, settings).execute(
          toolCallId,
          { ...params, timeout: params.timeout ?? DEFAULT_FOREGROUND_TIMEOUT_SECONDS },
          signal,
          onUpdate,
          ctx,
        );
      }

      const owner = lifetime;
      const handoff: { started?: BackgroundProcess } = {};
      const definition = createBashToolDefinition(ctx.cwd, {
        ...settings,
        operations: createBackgroundOperations(
          settings.shellPath,
          owner.processes,
          (started) => {
            handoff.started = started;
          },
          (result) => {
            if (!owner.active) return;
            const details = { ...result, command: params.command };
            pi.sendMessage<BackgroundExitMessage>(
              {
                customType: BACKGROUND_EXIT_MESSAGE,
                content: [
                  `Background bash ${backgroundExitStatus(result)}`,
                  `Command: ${params.command}`,
                  `PID: ${result.pid}`,
                  exitSummary(result.code, result.signal),
                  `Log: ${result.logPath}`,
                ].join("\n"),
                details,
                display: true,
              },
              { deliverAs: "steer", triggerTurn: true },
            );
          },
        ),
      });
      await definition.execute(toolCallId, params, signal, onUpdate, ctx);

      const started = handoff.started;
      if (started === undefined) throw new Error("Background command did not start a process");
      return {
        content: [{ type: "text", text: backgroundReport(started) }],
        details: { background: started },
      };
    },
  });

  pi.on("session_shutdown", () => {
    lifetime.active = false;
    for (const pid of lifetime.processes) killProcessGroup(pid);
    lifetime.processes.clear();
  });
}

/** What the model reads in place of command output. */
function backgroundReport(started: BackgroundProcess): string {
  return [
    `Running in the background: pid ${started.pid}`,
    `log: ${started.logPath}`,
    'The process appends its exit to the log as "[pi-bash-bg] exited with code <n>".',
    "An exit notification will arrive as a steered message and trigger a turn when idle. Do not poll for completion.",
  ].join("\n");
}

/**
 * Spawn detached instead of streaming.
 *
 * Pi's local shell backend pipes output into the tool call, which cannot outlive
 * the session and would block the child once the pipe buffer fills. Background
 * output goes to a file the model can read instead.
 */
function createBackgroundOperations(
  shellPath: string | undefined,
  hostedProcesses: Set<number>,
  onStarted: (started: BackgroundProcess) => void,
  onExited: (result: BackgroundExit) => void,
): BashOperations {
  return {
    exec: async (command, cwd, { env, timeout }) => {
      const timeoutMs = resolveTimeoutMs(timeout);
      const { shell, args, commandTransport } = getShellConfig(shellPath);
      const logPath = join(tmpdir(), `pi-bash-bg-${randomUUID().slice(0, 8)}.log`);
      const logFd = openSync(logPath, "w");
      const commandFromStdin = commandTransport === "stdin";
      const child = spawn(shell, commandFromStdin ? args : [...args, command], {
        cwd,
        detached: true,
        env,
        stdio: [commandFromStdin ? "pipe" : "ignore", logFd, logFd],
        windowsHide: true,
      });
      closeSync(logFd);
      if (commandFromStdin) {
        child.stdin?.on("error", () => {});
        child.stdin?.end(command);
      }

      // A missing working directory and an unusable shell fail asynchronously.
      await new Promise<void>((resolve, reject) => {
        child.once("spawn", () => resolve());
        child.once("error", reject);
      });
      const pid = child.pid;
      if (pid === undefined) throw new Error(`Failed to start background command: ${command}`);

      hostedProcesses.add(pid);
      // Errors after spawn, such as a failed kill, must not crash Pi.
      child.on("error", () => {});
      let timedOut = false;
      const timeoutHandle =
        timeoutMs === undefined
          ? undefined
          : setTimeout(() => {
              timedOut = true;
              killProcessGroup(pid);
            }, timeoutMs);
      child.on("exit", (code, exitSignal) => {
        clearTimeout(timeoutHandle);
        hostedProcesses.delete(pid);
        void appendFile(logPath, `\n${exitSummary(code, exitSignal)}\n`)
          .catch(() => {})
          .then(() => onExited({ pid, logPath, code, signal: exitSignal, timedOut }));
      });

      onStarted({ pid, logPath });
      return { exitCode: 0 };
    },
  };
}

function backgroundExitStatus(result: BackgroundExit): string {
  if (result.timedOut) return "timed out";
  if (result.signal !== null) return `killed by ${result.signal}`;
  return result.code === 0 ? "completed" : `failed · exit ${result.code ?? "unknown"}`;
}

/** The log line that tells the model how a background command ended. */
function exitSummary(code: number | null, signal: NodeJS.Signals | null): string {
  return signal === null ? `[pi-bash-bg] exited with code ${code ?? "unknown"}` : `[pi-bash-bg] killed by ${signal}`;
}

/** Mirror Pi's timeout validation, since the built-in check lives in its local shell backend. */
function resolveTimeoutMs(timeout: number | undefined): number | undefined {
  if (timeout === undefined) return undefined;
  if (!Number.isFinite(timeout) || timeout <= 0) {
    throw new Error("Invalid timeout: must be a finite number of seconds");
  }
  const timeoutMs = timeout * 1000;
  if (timeoutMs > MAX_TIMEOUT_MS) {
    throw new Error(`Invalid timeout: maximum is ${MAX_TIMEOUT_MS / 1000} seconds`);
  }
  return timeoutMs;
}

/**
 * Kill a detached child and everything it spawned. Detached children lead their
 * own process group, so a negative pid reaches descendants too; the second
 * attempt only reaches the shell itself.
 */
function killProcessGroup(pid: number): void {
  try {
    process.kill(-pid, "SIGKILL");
  } catch {
    try {
      process.kill(pid, "SIGKILL");
    } catch {
      // The process already exited.
    }
  }
}

/**
 * Read the shell settings an override must forward. Pi merges these files when
 * it builds its own `bash` tool, with the project layer winning.
 */
function readShellToolSettings(ctx: ExtensionContext): ShellToolSettings {
  const globalSettings = readSettingsFile(join(getAgentDir(), "settings.json"));
  const projectSettings = ctx.isProjectTrusted()
    ? readSettingsFile(join(ctx.cwd, CONFIG_DIR_NAME, "settings.json"))
    : {};
  const shellPath = projectSettings.shellPath ?? globalSettings.shellPath;
  return {
    shellPath: shellPath === undefined ? undefined : expandHome(shellPath),
    commandPrefix: projectSettings.shellCommandPrefix ?? globalSettings.shellCommandPrefix,
  };
}

interface SettingsFile {
  shellPath?: string;
  shellCommandPrefix?: string;
}

function readSettingsFile(path: string): SettingsFile {
  try {
    const parsed: unknown = JSON.parse(readFileSync(path, "utf-8"));
    if (typeof parsed !== "object" || parsed === null) return {};
    const settings = parsed as Record<string, unknown>;
    return {
      shellPath: typeof settings.shellPath === "string" ? settings.shellPath : undefined,
      shellCommandPrefix: typeof settings.shellCommandPrefix === "string" ? settings.shellCommandPrefix : undefined,
    };
  } catch {
    // Pi ignores an unreadable settings file and keeps its own defaults.
    return {};
  }
}

/** Pi expands a leading `~` in `shellPath` (docs/settings.md:196). */
function expandHome(path: string): string {
  if (path === "~") return homedir();
  if (path.startsWith("~/")) return join(homedir(), path.slice(2));
  return path;
}

export default registerBashBackground;

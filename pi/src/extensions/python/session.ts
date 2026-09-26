import { spawn, type ChildProcess } from "node:child_process";
import { closeSync, openSync } from "node:fs";
import { mkdtemp, open, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { Readable, Writable } from "node:stream";
import { StringDecoder } from "node:string_decoder";
import { DEFAULT_MAX_BYTES, DEFAULT_MAX_LINES, truncateTail } from "@earendil-works/pi-coding-agent";
import { logPythonEvent } from "./diagnostics.js";
import {
  isPythonEnvironment,
  pythonWorkerArguments,
  STARTUP_TIMEOUT_MS,
  type PythonEnvironment,
} from "./environment.js";

const INTERRUPT_GRACE_MS = 1_000;
const MAX_TIMEOUT_SECONDS = 2_147_483_647 / 1000;

export type PythonOutcome =
  "completed" | "python_error" | "interrupted" | "timed_out" | "process_exited" | "startup_error" | "output_error";

/** Execution metadata used for UI status and the model's state-loss notice. */
export interface PythonToolDetails {
  outcome: PythonOutcome;
  durationMs: number;
  /** A surviving environment can contain changes made before an error or interruption. */
  environmentAvailable: boolean;
  output: { truncated: boolean; path?: string };
  /** uv and worker infrastructure diagnostics, separate from user code output. */
  diagnosticsPath?: string;
}

export interface PythonExecutionResult {
  output: string;
  details: PythonToolDetails;
}

interface WorkerInfo {
  pid: number;
  environment: PythonEnvironment;
}

type WorkerMessage =
  | (WorkerInfo & { type: "ready" })
  | { type: "started"; callId: string }
  | { type: "completed"; callId: string; outcome: "completed" | "python_error" | "interrupted" };

interface PendingExecution {
  callId: string;
  started: boolean;
  timeoutSeconds: number;
  stopReason?: "interrupted" | "timed_out";
  timer?: ReturnType<typeof setTimeout>;
  forceTimer?: ReturnType<typeof setTimeout>;
  finish: (outcome: PythonOutcome) => void;
  onStarted?: () => void;
}

/** One uv launcher and its worker; the worker leads a separate group for interrupting user subprocesses. */
interface WorkerProcess {
  launcher: ChildProcess;
  requests: Writable;
  events: Readable;
  ready: Promise<WorkerInfo>;
  resolveReady: (info: WorkerInfo) => void;
  rejectReady: (error: Error) => void;
  exited: Promise<void>;
  resolveExited: () => void;
  info?: WorkerInfo;
  pending?: PendingExecution;
  stopping: boolean;
  ended: boolean;
  logPath: string;
}

/** Validate seconds before creating an environment or submitting any code. */
export function validateTimeout(seconds: number): void {
  if (!Number.isFinite(seconds) || seconds <= 0 || seconds > MAX_TIMEOUT_SECONDS) {
    throw new Error(`timeout must be a finite positive number of seconds, at most ${MAX_TIMEOUT_SECONDS}.`);
  }
}

/** Owns one live Python namespace. Calls are serialized by Pi; close permanently disposes this instance. */
export class PythonSession {
  private worker?: WorkerProcess;
  private disposed = false;
  private running = false;

  public constructor(
    private readonly sessionId: string,
    private readonly sessionCwd: string,
    private readonly onStateLost: () => void,
    private readonly onEnvironmentReady: (environment: PythonEnvironment) => void,
  ) {}

  public get available(): boolean {
    return this.worker?.info !== undefined && !this.worker.stopping && !this.worker.ended;
  }

  /** Execute a complete block in the supplied absolute cwd for this call, preserving partial output on failure. */
  public async execute(
    code: string,
    timeoutSeconds: number,
    callId: string,
    cwd: string,
    signal?: AbortSignal,
    onStarted?: () => void,
  ): Promise<PythonExecutionResult> {
    validateTimeout(timeoutSeconds);
    if (this.disposed) throw new Error("This Python session has been closed.");
    if (this.running) throw new Error("Python calls must execute sequentially.");
    this.running = true;
    const started = Date.now();
    let directory: string | undefined;
    let worker: WorkerProcess | undefined;
    let outcome: PythonOutcome = "startup_error";
    let output = "";
    let truncated = false;
    let outputPath: string | undefined;
    this.log({ phase: "execute_start", callId, cwd });
    try {
      if (signal?.aborted) outcome = "interrupted";
      else {
        worker = await this.ensureWorker(signal);
        if (signal?.aborted || this.disposed) outcome = "interrupted";
        else {
          directory = await mkdtemp(join(tmpdir(), "pi-python-output-"));
          const path = join(directory, "output.txt");
          // Create before dispatch so even an immediate process exit has a readable output file.
          const file = await open(path, "wx", 0o600);
          await file.close();
          outcome = await this.submit(worker, code, timeoutSeconds, callId, cwd, path, signal, onStarted);
          const preview = await readOutput(path);
          output = preview.text;
          truncated = preview.truncated;
          if (truncated) outputPath = path;
        }
      }
    } catch (error) {
      outcome = signal?.aborted || this.disposed ? "interrupted" : worker ? "output_error" : "startup_error";
      this.log({ phase: "execution_failure", callId, outcome, reason: errorMessage(error) });
    } finally {
      if (directory && !outputPath) {
        await rm(directory, { recursive: true, force: true }).catch((error: unknown) => {
          this.log({ phase: "output_cleanup_failed", callId, reason: errorMessage(error) });
        });
      }
      this.running = false;
    }
    const details: PythonToolDetails = {
      outcome,
      durationMs: Date.now() - started,
      environmentAvailable: this.available,
      output: { truncated, path: outputPath },
      diagnosticsPath:
        outcome === "startup_error" || outcome === "process_exited" || outcome === "output_error"
          ? (worker ?? this.worker)?.logPath
          : undefined,
    };
    this.log({ phase: "execute_end", callId, outcome, durationMs: details.durationMs });
    return { output, details };
  }

  /** Stop both process groups, including subprocesses still in those groups, before session replacement. */
  public async close(): Promise<void> {
    this.disposed = true;
    const worker = this.worker;
    if (!worker) return;
    this.log({ phase: "shutdown", pid: worker.info?.pid });
    this.terminate(worker);
    await worker.exited;
  }

  private async ensureWorker(signal?: AbortSignal): Promise<WorkerProcess> {
    if (this.available) return this.worker!;
    if (this.worker && !this.worker.ended) await this.worker.exited;
    const directory = await mkdtemp(join(tmpdir(), "pi-python-startup-"));
    if (this.disposed || signal?.aborted) {
      await rm(directory, { recursive: true, force: true });
      throw new Error("Python startup cancelled.");
    }
    const logPath = join(directory, "uv.log");
    const logFd = openSync(logPath, "wx", 0o600);
    let launcher: ChildProcess;
    try {
      launcher = spawn("uv", pythonWorkerArguments(), {
        cwd: this.sessionCwd,
        detached: true,
        stdio: ["ignore", logFd, logFd, "pipe", "pipe"],
      });
    } finally {
      closeSync(logFd);
    }
    let resolveReady!: (info: WorkerInfo) => void;
    let rejectReady!: (error: Error) => void;
    let resolveExited!: () => void;
    const ready = new Promise<WorkerInfo>((resolve, reject) => {
      resolveReady = resolve;
      rejectReady = reject;
    });
    const exited = new Promise<void>((resolve) => {
      resolveExited = resolve;
    });
    const worker: WorkerProcess = {
      launcher,
      requests: launcher.stdio[3] as Writable,
      events: launcher.stdio[4] as Readable,
      ready,
      resolveReady,
      rejectReady,
      exited,
      resolveExited,
      stopping: false,
      ended: false,
      logPath,
    };
    this.worker = worker;
    const decoder = new StringDecoder("utf8");
    let buffer = "";
    worker.events.on("data", (chunk: Buffer) => {
      if (worker.stopping || worker.ended) return;
      buffer += decoder.write(chunk);
      let end: number;
      try {
        while ((end = buffer.indexOf("\n")) !== -1) {
          const line = buffer.slice(0, end);
          buffer = buffer.slice(end + 1);
          this.receive(worker, parseMessage(line));
        }
        if (Buffer.byteLength(buffer) > 64 * 1024) throw new Error("Python control frame exceeded 64 KiB.");
      } catch (error) {
        this.log({ phase: "protocol_failure", reason: errorMessage(error), logPath });
        this.terminate(worker);
      }
    });
    const transportFailure = (error: Error): void => {
      this.log({ phase: "transport_failure", reason: error.message, logPath });
      this.terminate(worker);
    };
    worker.requests.on("error", transportFailure);
    worker.events.on("error", transportFailure);
    worker.events.on("end", () => {
      if (!worker.ended) this.terminate(worker);
    });
    launcher.once("error", (error) => {
      this.log({ phase: "startup_failure", reason: error.message, logPath });
      this.processEnded(worker);
    });
    launcher.once("exit", (code, exitSignal) => {
      this.log({ phase: "process_exit", pid: worker.info?.pid, reason: `code=${code} signal=${exitSignal}`, logPath });
      this.processEnded(worker);
    });
    const abort = (): void => this.terminate(worker);
    const timeout = setTimeout(() => {
      this.log({ phase: "startup_timeout", logPath });
      this.terminate(worker);
    }, STARTUP_TIMEOUT_MS);
    signal?.addEventListener("abort", abort, { once: true });
    if (signal?.aborted) abort();
    this.log({ phase: "startup", pid: launcher.pid, logPath });
    try {
      await ready;
      if (worker.stopping || worker.ended) throw new Error("Python exited during startup.");
      return worker;
    } catch (error) {
      this.terminate(worker);
      await exited;
      throw error;
    } finally {
      clearTimeout(timeout);
      signal?.removeEventListener("abort", abort);
    }
  }

  private submit(
    worker: WorkerProcess,
    code: string,
    timeoutSeconds: number,
    callId: string,
    cwd: string,
    outputPath: string,
    signal?: AbortSignal,
    onStarted?: () => void,
  ): Promise<PythonOutcome> {
    if (signal?.aborted || this.disposed) return Promise.resolve("interrupted");
    if (worker.ended || worker.stopping) return Promise.resolve("process_exited");
    return new Promise((resolve) => {
      const abort = (): void => this.interrupt(worker, "interrupted");
      const pending: PendingExecution = {
        callId,
        started: false,
        timeoutSeconds,
        onStarted,
        finish: (outcome) => {
          if (worker.pending !== pending) return;
          worker.pending = undefined;
          clearTimeout(pending.timer);
          clearTimeout(pending.forceTimer);
          signal?.removeEventListener("abort", abort);
          resolve(pending.stopReason ?? outcome);
        },
      };
      worker.pending = pending;
      // Bound acknowledgement too; the execution deadline starts only on the started event.
      pending.timer = setTimeout(() => this.terminate(worker), STARTUP_TIMEOUT_MS);
      signal?.addEventListener("abort", abort, { once: true });
      worker.requests.write(`${JSON.stringify({ type: "execute", callId, code, cwd, outputPath })}\n`);
    });
  }

  private receive(worker: WorkerProcess, message: WorkerMessage): void {
    if (message.type === "ready") {
      if (worker.info) throw new Error("Python sent duplicate readiness.");
      worker.info = message;
      this.onEnvironmentReady(message.environment);
      worker.resolveReady(message);
      this.log({
        phase: "ready",
        pid: message.pid,
        interpreter: { executable: message.environment.executable, version: message.environment.version },
        packageCount: message.environment.packages.length,
      });
      return;
    }
    const pending = worker.pending;
    if (!pending || pending.callId !== message.callId)
      throw new Error("Python returned an unexpected call identifier.");
    if (message.type === "started") {
      if (pending.started) throw new Error("Python started a call twice.");
      pending.started = true;
      clearTimeout(pending.timer);
      pending.timer = setTimeout(() => this.interrupt(worker, "timed_out"), pending.timeoutSeconds * 1000);
      this.log({ phase: "code_started", callId: pending.callId, pid: worker.info?.pid });
      if (pending.stopReason) this.sendInterrupt(worker);
      pending.onStarted?.();
    } else {
      if (!pending.started) throw new Error("Python completed a call before starting it.");
      pending.finish(message.outcome);
    }
  }

  private interrupt(worker: WorkerProcess, reason: "interrupted" | "timed_out"): void {
    const pending = worker.pending;
    if (!pending || pending.stopReason) return;
    pending.stopReason = reason;
    this.log({ phase: "interrupt", callId: pending.callId, pid: worker.info?.pid, reason });
    if (pending.started) this.sendInterrupt(worker);
    pending.forceTimer = setTimeout(() => this.terminate(worker), INTERRUPT_GRACE_MS);
  }

  private sendInterrupt(worker: WorkerProcess): void {
    if (worker.info) this.signalGroup(worker.info.pid, "SIGINT");
  }

  private terminate(worker: WorkerProcess): void {
    if (worker.ended || worker.stopping) return;
    worker.stopping = true;
    this.log({ phase: "terminate", pid: worker.info?.pid, callId: worker.pending?.callId });
    // Killing the worker group first also stops subprocesses blocked in a native call.
    if (worker.info) this.signalGroup(worker.info.pid, "SIGKILL");
    if (worker.launcher.pid) this.signalGroup(worker.launcher.pid, "SIGKILL");
  }

  private processEnded(worker: WorkerProcess): void {
    if (worker.ended) return;
    worker.ended = true;
    if (worker.info) {
      this.signalGroup(worker.info.pid, "SIGKILL");
      if (!this.disposed) this.onStateLost();
    }
    worker.rejectReady(new Error(`Python did not become ready. Diagnostics: ${worker.logPath}`));
    worker.pending?.finish("process_exited");
    worker.requests.destroy();
    worker.events.destroy();
    worker.resolveExited();
  }

  private signalGroup(pid: number, signal: NodeJS.Signals): void {
    try {
      process.kill(-pid, signal);
    } catch (error) {
      // macOS can report EPERM for an exiting group. Signal errors must not escape event callbacks.
      if ((error as NodeJS.ErrnoException).code !== "ESRCH") {
        this.log({ phase: "signal_failed", pid, reason: `${signal}: ${errorMessage(error)}` });
      }
    }
  }

  private log(event: Omit<Parameters<typeof logPythonEvent>[0], "sessionId">): void {
    logPythonEvent({ sessionId: this.sessionId, ...event });
  }
}

function parseMessage(line: string): WorkerMessage {
  const value: unknown = JSON.parse(line);
  if (!value || typeof value !== "object") throw new Error("Invalid Python control message.");
  const message = value as Record<string, unknown>;
  if (
    message.type === "ready" &&
    Number.isSafeInteger(message.pid) &&
    (message.pid as number) > 0 &&
    isPythonEnvironment(message.environment)
  )
    return message as unknown as WorkerMessage;
  if (
    typeof message.callId === "string" &&
    (message.type === "started" ||
      (message.type === "completed" && ["completed", "python_error", "interrupted"].includes(String(message.outcome))))
  ) {
    return message as unknown as WorkerMessage;
  }
  throw new Error("Invalid Python control message.");
}

/** Read only a bounded tail; oversized output remains on disk for the read tool. */
async function readOutput(path: string): Promise<{ text: string; truncated: boolean }> {
  const file = await open(path, "r");
  try {
    const size = (await file.stat()).size;
    const length = Math.min(size, DEFAULT_MAX_BYTES);
    const buffer = Buffer.alloc(length);
    const { bytesRead } = await file.read(buffer, 0, length, size - length);
    let start = 0;
    if (size > length) {
      while (start < bytesRead && (buffer[start]! & 0xc0) === 0x80) start++;
    }
    const preview = truncateTail(buffer.subarray(start, bytesRead).toString("utf8"), {
      maxBytes: DEFAULT_MAX_BYTES,
      maxLines: DEFAULT_MAX_LINES,
    });
    return { text: preview.content, truncated: size > length || preview.truncated };
  } finally {
    await file.close();
  }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

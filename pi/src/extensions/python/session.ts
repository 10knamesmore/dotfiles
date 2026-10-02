import { spawn, type ChildProcess } from "node:child_process";
import { closeSync, openSync } from "node:fs";
import { mkdtemp, open, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join } from "node:path";
import type { Readable, Writable } from "node:stream";
import { StringDecoder } from "node:string_decoder";
import type { ImageContent } from "@earendil-works/pi-ai";
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
  /** Number of complete image attachments returned with this call. */
  imageCount: number;
  /** Held keys or buttons may remain pressed when failure cleanup did not complete. */
  inputCleanupFailed: boolean;
  /** uv and worker infrastructure diagnostics, separate from user code output. */
  diagnosticsPath?: string;
}

/** Captured text and complete image attachments, including output emitted before a cell failed. */
export interface PythonExecutionResult {
  output: string;
  images: ImageContent[];
  details: PythonToolDetails;
}

/** A completed image file in the current call's temporary output directory. */
interface ImageFile {
  /** Absolute path sent only after the worker has closed the file. */
  path: string;

  /** Native image output supports these encoded formats without resizing. */
  mimeType: "image/png" | "image/jpeg" | "image/gif" | "image/webp";
}

interface SubmittedResult {
  outcome: PythonOutcome;
  images: ImageFile[];
  inputCleanupFailed: boolean;
}

interface WorkerInfo {
  pid: number;
  environment: PythonEnvironment;
}

/** One SDK child process group; terminal and browser groups survive a worker SIGINT. */
interface OwnedProcessGroup {
  kind: "terminal" | "browser";
  id: string;
  pid: number;
}

type ProcessOwnershipMessage = Extract<WorkerMessage, { type: "process_ownership" }>;

type WorkerMessage =
  | (WorkerInfo & { type: "ready" })
  | { type: "started"; callId: string }
  | { type: "completed"; callId: string; outcome: "completed" | "python_error" | "interrupted" }
  | (ImageFile & { type: "image"; callId: string })
  | { type: "input_cleanup_failed"; callId: string; errorType: string }
  | { type: "process_ownership"; kind: "terminal" | "browser"; action: "opened" | "closed"; id: string; pid: number }
  | { type: "startup_error"; reason: string };

interface PendingExecution {
  callId: string;
  started: boolean;
  timeoutSeconds: number;
  /** Files are collected synchronously from fd4 and read after completion, before directory cleanup. */
  images: ImageFile[];
  outputDirectory: string;
  inputCleanupFailed: boolean;
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
  /** Separate SDK process groups, keyed by kind and ownership id. */
  ownedProcessGroups: Map<string, OwnedProcessGroup>;
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
    return !this.disposed && this.worker?.info !== undefined && !this.worker.stopping && !this.worker.ended;
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
    const images: ImageContent[] = [];
    let imageFiles: ImageFile[] = [];
    let inputCleanupFailed = false;
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
          const submitted = await this.submit(worker, code, timeoutSeconds, callId, cwd, path, signal, onStarted);
          outcome = submitted.outcome;
          imageFiles = submitted.images;
          inputCleanupFailed = submitted.inputCleanupFailed;
          const imageReadStarted = Date.now();
          for (const image of imageFiles) {
            try {
              const bytes = await readFile(image.path);
              images.push({ type: "image", data: bytes.toString("base64"), mimeType: image.mimeType });
            } catch (error) {
              outcome = "output_error";
              this.log({ phase: "image_read_failed", callId, reason: errorMessage(error) });
            }
          }
          if (imageFiles.length > 0) {
            this.log({
              phase: "images_read",
              callId,
              imageCount: images.length,
              durationMs: Date.now() - imageReadStarted,
            });
          }
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
      if (directory) {
        // Truncated text remains readable, but its already attached image files can be removed.
        const paths = outputPath ? imageFiles.map((image) => image.path) : [directory];
        for (const path of paths) {
          await rm(path, { recursive: !outputPath, force: true }).catch((error: unknown) => {
            this.log({ phase: "output_cleanup_failed", callId, reason: errorMessage(error) });
          });
        }
      }
      this.running = false;
    }
    const details: PythonToolDetails = {
      outcome,
      durationMs: Date.now() - started,
      environmentAvailable: this.available,
      output: { truncated, path: outputPath },
      imageCount: images.length,
      inputCleanupFailed,
      diagnosticsPath:
        outcome === "startup_error" || outcome === "process_exited" || outcome === "output_error"
          ? (worker ?? this.worker)?.logPath
          : undefined,
    };
    this.log({ phase: "execute_end", callId, outcome, imageCount: images.length, durationMs: details.durationMs });
    return { output, images, details };
  }

  /** Let the worker release virtual input and PTYs before enforcing a bounded shutdown. */
  public async close(): Promise<void> {
    this.disposed = true;
    const worker = this.worker;
    if (!worker || worker.ended) return;
    this.log({ phase: "shutdown", pid: worker.info?.pid });
    if (!worker.stopping) {
      if (worker.pending) this.interrupt(worker, "interrupted");
      // EOF leaves the request loop and runs native atexit cleanup. Disconnecting
      // a virtual keyboard alone does not make Hyprland release its pressed keys.
      worker.requests.end();
    }
    const timeout = setTimeout(() => this.terminate(worker), INTERRUPT_GRACE_MS);
    try {
      await worker.exited;
    } finally {
      clearTimeout(timeout);
    }
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
      ownedProcessGroups: new Map(),
      stopping: false,
      ended: false,
      logPath,
    };
    this.worker = worker;
    const decoder = new StringDecoder("utf8");
    let buffer = "";
    worker.events.on("data", (chunk: Buffer) => {
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
  ): Promise<SubmittedResult> {
    if (signal?.aborted || this.disposed)
      return Promise.resolve({ outcome: "interrupted", images: [], inputCleanupFailed: false });
    if (worker.ended || worker.stopping)
      return Promise.resolve({ outcome: "process_exited", images: [], inputCleanupFailed: false });
    return new Promise((resolve) => {
      const abort = (): void => this.interrupt(worker, "interrupted");
      const pending: PendingExecution = {
        callId,
        started: false,
        timeoutSeconds,
        images: [],
        outputDirectory: dirname(outputPath),
        inputCleanupFailed: false,
        onStarted,
        finish: (outcome) => {
          if (worker.pending !== pending) return;
          worker.pending = undefined;
          clearTimeout(pending.timer);
          clearTimeout(pending.forceTimer);
          signal?.removeEventListener("abort", abort);
          resolve({
            outcome: pending.stopReason ?? outcome,
            images: pending.images,
            inputCleanupFailed: pending.inputCleanupFailed,
          });
        },
      };
      worker.pending = pending;
      // Bound acknowledgement too; the execution deadline starts only on the started event.
      pending.timer = setTimeout(() => this.terminate(worker), STARTUP_TIMEOUT_MS);
      signal?.addEventListener("abort", abort, { once: true });
      try {
        worker.requests.write(`${JSON.stringify({ type: "execute", callId, code, cwd, outputPath })}\n`);
      } catch (error) {
        this.log({ phase: "request_write_failed", callId, reason: errorMessage(error), logPath: worker.logPath });
        pending.finish("process_exited");
        this.terminate(worker);
      }
    });
  }

  private receive(worker: WorkerProcess, message: WorkerMessage): void {
    // Ownership arrives while user code runs, but it must never depend on a pending execution.
    if (message.type === "process_ownership") {
      this.recordProcessOwnership(worker, message);
      return;
    }
    // A terminated worker only still reports process ownership; later control events are stale.
    if (worker.stopping || worker.ended) return;
    if (message.type === "startup_error") {
      if (!worker.info) {
        worker.rejectReady(new Error(`Python worker startup failed: ${message.reason}`));
        this.log({ phase: "startup_failure", reason: message.reason, logPath: worker.logPath });
      }
      return;
    }
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
    } else if (message.type === "image") {
      if (!pending.started) throw new Error("Python emitted an image before starting its call.");
      if (dirname(message.path) !== pending.outputDirectory)
        throw new Error("Python image is outside the active call's output directory.");
      pending.images.push({ path: message.path, mimeType: message.mimeType });
    } else if (message.type === "input_cleanup_failed") {
      if (!pending.started) throw new Error("Python reported input cleanup before starting its call.");
      pending.inputCleanupFailed = true;
      this.log({ phase: "input_cleanup_failed", callId: pending.callId, reason: message.errorType });
    } else {
      if (!pending.started) throw new Error("Python completed a call before starting it.");
      pending.finish(message.outcome);
    }
  }

  /** Track the SDK process groups that must be terminated even after the worker exits. */
  private recordProcessOwnership(worker: WorkerProcess, message: ProcessOwnershipMessage): void {
    const key = `${message.kind}:${message.id}`;
    if (message.action === "closed") {
      worker.ownedProcessGroups.delete(key);
    } else if (worker.stopping || worker.ended) {
      this.log({
        phase: "owned_process_kill",
        resourceKind: message.kind,
        resourceId: message.id,
        pid: message.pid,
        callId: worker.pending?.callId,
      });
      this.signalGroup(message.pid, "SIGKILL");
    } else {
      worker.ownedProcessGroups.set(key, { kind: message.kind, id: message.id, pid: message.pid });
    }
    this.log({
      phase: "process_ownership",
      action: message.action,
      resourceKind: message.kind,
      resourceId: message.id,
      pid: message.pid,
      callId: worker.pending?.callId,
    });
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
    // SDK children lead separate groups so interrupting a cell leaves them alive.
    this.terminateOwnedProcessGroups(worker);
    // Killing the worker group first also stops subprocesses blocked in a native call.
    if (worker.info) this.signalGroup(worker.info.pid, "SIGKILL");
    if (worker.launcher.pid) this.signalGroup(worker.launcher.pid, "SIGKILL");
  }

  /** Kill each owned group once from a snapshot, then forget it so a recycled pid cannot be signalled again. */
  private terminateOwnedProcessGroups(worker: WorkerProcess): void {
    const groups = [...worker.ownedProcessGroups.values()];
    worker.ownedProcessGroups.clear();
    for (const group of groups) {
      this.log({
        phase: "owned_process_kill",
        resourceKind: group.kind,
        resourceId: group.id,
        pid: group.pid,
        callId: worker.pending?.callId,
      });
      this.signalGroup(group.pid, "SIGKILL");
    }
  }

  private processEnded(worker: WorkerProcess): void {
    if (worker.ended) return;
    worker.ended = true;
    // A SIGKILLed worker never runs Python cleanup; terminate its separate SDK groups here.
    this.terminateOwnedProcessGroups(worker);
    if (worker.info) {
      this.signalGroup(worker.info.pid, "SIGKILL");
      if (!this.disposed) this.onStateLost();
    }
    worker.rejectReady(new Error(`Python did not become ready. Diagnostics: ${worker.logPath}`));
    worker.pending?.finish("process_exited");
    worker.requests.destroy();
    // Keep reading: pipe-buffered ownership events may arrive after the exit event.
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

  private log(event: Omit<Parameters<typeof logPythonEvent>[0], "sessionId"> & { imageCount?: number }): void {
    logPythonEvent({ sessionId: this.sessionId, ...event });
  }
}

function parseMessage(line: string): WorkerMessage {
  if (Buffer.byteLength(line) > 64 * 1024) throw new Error("Python control frame exceeded 64 KiB.");
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
  if (
    message.type === "image" &&
    typeof message.callId === "string" &&
    typeof message.path === "string" &&
    isAbsolute(message.path) &&
    typeof message.mimeType === "string" &&
    ["image/png", "image/jpeg", "image/gif", "image/webp"].includes(message.mimeType)
  ) {
    return message as unknown as WorkerMessage;
  }
  if (
    message.type === "input_cleanup_failed" &&
    typeof message.callId === "string" &&
    typeof message.errorType === "string" &&
    message.errorType.length > 0
  ) {
    return message as unknown as WorkerMessage;
  }
  if (
    message.type === "process_ownership" &&
    (message.kind === "terminal" || message.kind === "browser") &&
    (message.action === "opened" || message.action === "closed") &&
    typeof message.id === "string" &&
    message.id.length > 0 &&
    Number.isSafeInteger(message.pid) &&
    (message.pid as number) > 0
  ) {
    return message as unknown as WorkerMessage;
  }
  if (message.type === "startup_error" && typeof message.reason === "string" && message.reason.length > 0) {
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

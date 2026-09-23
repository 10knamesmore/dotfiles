/**
 * Minimal JSONL transport for a `pi --mode rpc` child process. Owns the
 * process: spawn, request/response correlation, event fan-out, and honest
 * exit semantics - every pending and future request rejects once the channel
 * terminally fails or exits, so a dead child can never leave a caller waiting.
 */

import { spawn } from "node:child_process";
import { StringDecoder } from "node:string_decoder";
import { reportDiagnostic } from "../../diagnostics.js";
import { errorMessage } from "../../util.js";

export interface RpcExit {
  code: number | null;
  signal: NodeJS.Signals | null;
}

export interface RpcRequestOptions {
  /** Reject the request after this many ms if the child has not answered. */
  timeoutMs?: number;
  /** Runs synchronously when the successful response frame is dispatched. */
  onResponse?: (response: unknown) => void;
}

/** An explicit negative response rejects the correlated command. */
export class RpcCommandRejectedError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "RpcCommandRejectedError";
  }
}

/** A correlated response whose command or success discriminator is untrustworthy. */
export class RpcProtocolError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "RpcProtocolError";
  }
}

/** A request failed because the owned RPC channel closed. */
export class RpcChannelClosedError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "RpcChannelClosedError";
  }
}

/** The child emitted one protocol frame larger than the transport can accept. */
export class RpcFrameTooLargeError extends RpcChannelClosedError {
  constructor(message: string) {
    super(message);
    this.name = "RpcFrameTooLargeError";
  }
}

/** The transport surface the session adapter consumes; faked in tests. */
export interface ChildRpc {
  request(command: Record<string, unknown>, options?: RpcRequestOptions): Promise<unknown>;
  /** Fire-and-forget write, for protocol messages with no response (extension_ui_response). */
  send(message: Record<string, unknown>): void;
  onEvent(listener: (event: Record<string, unknown>) => void): () => void;
  /** Fires once; immediately when subscribing after exit. */
  onExit(listener: (exit: RpcExit) => void): () => void;
  kill(signal?: NodeJS.Signals): void;
  readonly exited: Promise<RpcExit>;
  /** Bounded tail of the child's stderr, for diagnostics. */
  stderrTail(): string;
}

const STDERR_TAIL_MAX = 4_096;
/** Largest JSONL frame buffered and parsed, excluding its newline, in decoded UTF-16 code units. */
export const RPC_MAX_FRAME_CHARS = 8 * 1024 * 1024;
const RPC_FRAME_TYPE_PREFIX_CHARS = 256;
const MAX_DISCARDED_FRAME_CHARS = 512 * 1024 * 1024;

interface PendingRequest {
  command: unknown;
  resolve: (data: unknown) => void;
  reject: (error: Error) => void;
  timer?: ReturnType<typeof setTimeout>;
  onResponse: ((response: unknown) => void) | undefined;
}

export function spawnChildRpc(command: readonly string[], options: { cwd: string; env?: NodeJS.ProcessEnv }): ChildRpc {
  const [binary, ...args] = command;
  if (!binary) throw new Error("Child RPC spawn requires a non-empty command");
  const child = spawn(binary, args, {
    cwd: options.cwd,
    env: options.env ?? process.env,
    stdio: ["pipe", "pipe", "pipe"],
    detached: true,
  });

  const pending = new Map<string, PendingRequest>();
  const eventListeners = new Set<(event: Record<string, unknown>) => void>();
  const exitListeners = new Set<(exit: RpcExit) => void>();
  let exitInfo: RpcExit | undefined;
  /** Set once process exit bookkeeping and listener notification are complete. */
  let channelDone = false;
  let stderr = "";
  let requestId = 0;
  let stdoutBuffer = "";
  let discardedFrameChars: number | undefined;
  let discardedFrameType: string | undefined;
  let totalDiscardedFrameChars = 0;
  /** Set synchronously on the first terminal fault, before process reap. */
  let channelFailure: Error | undefined;
  // Decode across chunk boundaries so a multibyte UTF-8 sequence split between
  // two reads is not corrupted (plain chunk.toString would mangle it).
  const stdoutDecoder = new StringDecoder("utf8");

  let resolveExited: (exit: RpcExit) => void;
  const exited = new Promise<RpcExit>((resolve) => { resolveExited = resolve; });
  /** After 'exit', bound how long we wait for stdout 'close' before settling. */
  let settleGrace: ReturnType<typeof setTimeout> | undefined;
  const SETTLE_AFTER_EXIT_MS = 250;

  function killProcessGroup(signal: NodeJS.Signals): void {
    try {
      if (child.pid === undefined) throw new Error("child pid unavailable");
      process.kill(-child.pid, signal);
    } catch {
      try { child.kill(signal); } catch { /* already gone */ }
    }
  }

  child.stderr.on("data", (chunk: Buffer) => {
    stderr = (stderr + chunk.toString("utf8")).slice(-STDERR_TAIL_MAX);
  });

  child.stdout.on("data", (chunk: Buffer) => {
    if (channelFailure !== undefined) return; // Frames after a terminal fault are untrustworthy.
    drainStdoutLines(stdoutDecoder.write(chunk));
  });

  function drainStdoutLines(decoded = "", endOfStream = false): void {
    if (channelFailure !== undefined) return;
    let offset = 0;
    while (offset < decoded.length) {
      if (discardedFrameChars !== undefined) {
        const newline = decoded.indexOf("\n", offset);
        const end = newline === -1 ? decoded.length : newline;
        if (discardFrameCharacters(end - offset)) return;
        if (newline === -1) break;
        reportDiscardedFrame();
        offset = newline + 1;
        continue;
      }

      const newline = decoded.indexOf("\n", offset);
      const end = newline === -1 ? decoded.length : newline;
      const segmentLength = end - offset;
      if (stdoutBuffer.length + segmentLength > RPC_MAX_FRAME_CHARS) {
        const characters = stdoutBuffer.length + segmentLength;
        const prefix = stdoutBuffer.length >= RPC_FRAME_TYPE_PREFIX_CHARS
          ? stdoutBuffer.slice(0, RPC_FRAME_TYPE_PREFIX_CHARS)
          : stdoutBuffer + decoded.slice(offset, offset + RPC_FRAME_TYPE_PREFIX_CHARS - stdoutBuffer.length);
        const frameType = /"type":"([^"\\]*)"/.exec(prefix)?.[1];
        stdoutBuffer = "";
        if (frameType === undefined || prefix.includes('"type":"response"')) {
          failOversizedFrame(characters);
          return;
        }
        discardedFrameChars = 0;
        discardedFrameType = frameType;
        if (discardFrameCharacters(characters)) return;
        if (newline === -1) break;
        reportDiscardedFrame();
        offset = newline + 1;
        continue;
      }

      stdoutBuffer += decoded.slice(offset, end);
      if (newline === -1) break;
      const line = stdoutBuffer.trim();
      stdoutBuffer = "";
      if (line.length > 0) {
        let message: Record<string, unknown> | undefined;
        try {
          message = JSON.parse(line) as Record<string, unknown>;
        } catch {
          message = undefined; // Non-protocol stdout noise must not kill the channel.
        }
        if (message) handleMessage(message);
        if (channelFailure !== undefined) return;
      }
      offset = newline + 1;
    }

    if (endOfStream && discardedFrameChars !== undefined) reportDiscardedFrame();
  }

  function discardFrameCharacters(characters: number): boolean {
    if (discardedFrameChars === undefined) return false;
    discardedFrameChars += characters;
    totalDiscardedFrameChars += characters;
    if (totalDiscardedFrameChars <= MAX_DISCARDED_FRAME_CHARS) return false;
    const observedTotal = totalDiscardedFrameChars;
    reportDiscardedFrame();
    const failure = new RpcFrameTooLargeError(
      `Child RPC oversized-frame discard total exceeded the ${MAX_DISCARDED_FRAME_CHARS}-character hard limit (${observedTotal} characters discarded)`,
    );
    failChannel({ code: null, signal: null }, failure.message, failure);
    return true;
  }

  function reportDiscardedFrame(): void {
    if (discardedFrameChars === undefined || discardedFrameType === undefined) return;
    reportDiagnostic(
      `[subagent-workflow] discarded oversized child RPC event frame of type ${JSON.stringify(discardedFrameType)} (${discardedFrameChars} characters; transport cap ${RPC_MAX_FRAME_CHARS})`,
    );
    discardedFrameChars = undefined;
    discardedFrameType = undefined;
  }

  function failOversizedFrame(characters: number): void {
    const failure = new RpcFrameTooLargeError(
      `Child RPC frame exceeded the ${RPC_MAX_FRAME_CHARS}-character transport cap (${characters} characters observed)`,
    );
    failChannel({ code: null, signal: null }, failure.message, failure);
  }

  function handleMessage(message: Record<string, unknown>): void {
    if (channelFailure !== undefined) return;
    if (message.type === "response" && typeof message.id === "string") {
      const request = pending.get(message.id);
      if (!request) return;
      if (message.command !== request.command || typeof message.success !== "boolean") {
        const failure = new RpcProtocolError(
          `Malformed correlated RPC response for ${String(request.command)}: expected matching command and boolean success`,
        );
        failChannel({ code: null, signal: null }, failure.message, failure);
        return;
      }
      pending.delete(message.id);
      if (request.timer) clearTimeout(request.timer);
      if (message.success) {
        try {
          request.onResponse?.(message.data);
        } catch (error) {
          request.reject(error instanceof Error ? error : new Error(String(error)));
          return;
        }
        request.resolve(message.data);
      } else {
        request.reject(new RpcCommandRejectedError(typeof message.error === "string" ? message.error : `RPC ${String(request.command)} failed`));
      }
      return;
    }
    if (typeof message.type === "string") {
      for (const listener of eventListeners) {
        try { listener(message); } catch (error) {
          reportDiagnostic(`[subagent-workflow] child event listener failed: ${errorMessage(error)}`);
        }
      }
    }
  }

  /**
   * Record the first terminal failure and atomically close request/response and
   * event dispatch. Process exit notification remains owned by settleChannel.
   */
  function closeRequestPlane(failure: Error): void {
    if (channelFailure !== undefined) return;
    channelFailure = failure;
    stdoutBuffer = "";
    discardedFrameChars = undefined;
    discardedFrameType = undefined;
    for (const request of pending.values()) {
      if (request.timer) clearTimeout(request.timer);
      request.reject(failure);
    }
    pending.clear();
  }

  /** Settle process exit exactly once and notify exit listeners. */
  function settleChannel(result: RpcExit, reason?: string): void {
    if (settleGrace) { clearTimeout(settleGrace); settleGrace = undefined; }
    if (channelDone) return;
    channelDone = true;
    exitInfo ??= result;
    const detail = reason ? `${reason}. ` : "";
    closeRequestPlane(channelFailure ?? new RpcChannelClosedError(
      `${detail}Child pi channel closed (code ${exitInfo.code ?? "null"}, signal ${exitInfo.signal ?? "null"}). Stderr: ${stderr || "(empty)"}`,
    ));
    resolveExited(exitInfo);
    for (const listener of exitListeners) {
      try { listener(exitInfo); } catch (error) {
        reportDiagnostic(`[subagent-workflow] child exit listener failed: ${errorMessage(error)}`);
      }
    }
    exitListeners.clear();
  }

  /**
   * Force the channel down on a terminal fault. The request plane closes now,
   * but `exited` waits for the existing process reap path when a child exists.
   */
  function failChannel(result: RpcExit, reason: string, failure: Error = new RpcChannelClosedError(reason)): void {
    closeRequestPlane(failure);
    stderr = (stderr + `\n${reason}`).slice(-STDERR_TAIL_MAX);
    const processGone = exitInfo !== undefined || child.exitCode !== null || child.signalCode !== null;
    if (!processGone) killProcessGroup("SIGKILL");
    if (child.pid === undefined) {
      settleChannel(result, reason); // Never spawned; no exit event will come.
      return;
    }
    // exit -> bounded grace -> close settles the channel.
  }

  // Defer settlement (which resolves `exited` and rejects pending) to 'close',
  // after stdout fully ends, so a final buffered response still resolves its
  // request instead of being rejected as abandoned. But bound the wait: a
  // grandchild can keep the stdout pipe open past the child's own exit, so a
  // short grace after 'exit' forces settlement. `exited` thus resolves within
  // ~250ms of real exit, and by then the channel is fully settled - which is
  // what a dispose() awaiting it, and a late onExit subscriber, both rely on.
  child.on("exit", (code, signal) => {
    exitInfo ??= { code, signal };
    if (!channelDone && !settleGrace) {
      settleGrace = setTimeout(() => settleChannel(exitInfo ?? { code, signal }), SETTLE_AFTER_EXIT_MS);
      settleGrace.unref?.();
    }
  });
  child.on("close", (code, signal) => {
    drainStdoutLines(stdoutDecoder.end(), true);
    settleChannel(exitInfo ?? { code, signal });
  });
  child.on("error", (error) => failChannel({ code: null, signal: null }, `child spawn error: ${errorMessage(error)}`));
  // Unhandled stdin errors (EPIPE when the child closes its input while alive)
  // would otherwise crash the whole parent process.
  child.stdin.on("error", (error) => failChannel({ code: null, signal: null }, `child stdin error: ${errorMessage(error)}`));

  return {
    request(command: Record<string, unknown>, requestOptions?: RpcRequestOptions): Promise<unknown> {
      if (channelFailure !== undefined) return Promise.reject(channelFailure);
      const id = `req-${++requestId}`;
      return new Promise((resolve, reject) => {
        const entry: PendingRequest = { command: command.type, resolve, reject, onResponse: requestOptions?.onResponse };
        if (requestOptions?.timeoutMs !== undefined) {
          entry.timer = setTimeout(() => {
            if (!pending.delete(id)) return;
            reject(new Error(`Child RPC ${String(command.type)} timed out after ${requestOptions.timeoutMs}ms`));
          }, requestOptions.timeoutMs);
          entry.timer.unref?.();
        }
        pending.set(id, entry);
        child.stdin.write(`${JSON.stringify({ ...command, id })}\n`, (error) => {
          if (!error) return;
          const failure = new RpcChannelClosedError(
            `Child RPC request write failed: ${errorMessage(error)}`,
            { cause: error },
          );
          // Keep this request in the shared pending set. settleChannel owns timer
          // cleanup and rejects every request with the same terminal failure.
          failChannel({ code: null, signal: null }, failure.message, failure);
        });
      });
    },
    send(message: Record<string, unknown>): void {
      if (channelFailure !== undefined) return;
      child.stdin.write(`${JSON.stringify(message)}\n`, (error) => {
        if (!error) return;
        const failure = new RpcChannelClosedError(
          `Child RPC send write failed: ${errorMessage(error)}`,
          { cause: error },
        );
        failChannel({ code: null, signal: null }, failure.message, failure);
      });
    },
    onEvent(listener) {
      eventListeners.add(listener);
      return () => eventListeners.delete(listener);
    },
    onExit(listener) {
      if (channelDone && exitInfo) {
        listener(exitInfo);
        return () => {};
      }
      exitListeners.add(listener);
      return () => exitListeners.delete(listener);
    },
    kill(signal: NodeJS.Signals = "SIGKILL") {
      if (!channelDone && child.exitCode === null && child.signalCode === null && child.pid !== undefined) killProcessGroup(signal);
    },
    exited,
    stderrTail: () => stderr,
  };
}

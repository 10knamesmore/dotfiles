import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import type { Frame } from "./frame.ts";

type Completion = { type: "done" } | { type: "error"; message: string };
type StreamEvent = ({ type: "frame" } & Frame) | Completion;

function parseEvent(line: string): StreamEvent {
  const event = JSON.parse(line) as StreamEvent | null;
  if (
    event?.type === "done" ||
    (event?.type === "error" && typeof event.message === "string") ||
    (event?.type === "frame" && Array.isArray(event.lines))
  ) {
    return event;
  }
  throw new Error("expected a frame, done, or error event");
}

export interface MermaidStream {
  append(source: string): void;
  finish(source: string): void;
  dispose(): void;
}

export function startMermaidStream(
  width: number,
  onFrame: (frame: Frame) => void,
  onEnd: (diagnostic?: string) => void,
): MermaidStream {
  const columns = Math.max(1, Math.min(4096, Math.floor(width)));
  const child = spawn("mermaid-text", ["--stream", "--width", String(columns)]);
  const stdout = createInterface({ input: child.stdout, crlfDelay: Infinity });
  let stderr = "";
  let processError: string | undefined;
  let protocolError: string | undefined;
  let completion: Completion | undefined;
  let hasFrame = false;
  let disposed = false;

  child.stderr.setEncoding("utf8").on("data", (chunk: string) => {
    stderr += chunk;
  });
  child.on("error", (error: NodeJS.ErrnoException) => {
    processError = error.code === "ENOENT" ? "executable not found on PATH" : error.message;
  });
  const recordIoError = (error: Error) => {
    processError ??= error.message;
  };
  child.stdin.on("error", recordIoError);
  child.stderr.on("error", recordIoError);
  stdout.on("error", recordIoError);

  stdout.on("line", (line) => {
    if (disposed || protocolError !== undefined) return;
    let event;
    try {
      event = parseEvent(line);
      if (completion) throw new Error("output after the final stream event");
    } catch (error) {
      protocolError = `invalid stream output (${(error as Error).message})`;
      child.kill();
      return;
    }
    if (event.type === "frame") {
      hasFrame = true;
      onFrame(event);
    } else {
      completion = event;
    }
  });

  child.once("close", (code, signal) => {
    if (disposed) return;
    if (completion?.type === "error") {
      onEnd(completion.message);
    } else if (completion?.type === "done" && hasFrame && code === 0 && processError === undefined && !protocolError) {
      onEnd();
    } else {
      const cliError = stderr.trim().replace(/^mermaid-text:\s*/u, "");
      const exitError = signal
        ? `terminated by ${signal}`
        : code !== 0
          ? `exited with code ${code}`
          : "no completed diagram";
      onEnd(cliError || protocolError || processError || exitError);
    }
  });

  return {
    append: (source) => {
      child.stdin.write(source);
    },
    finish: (source) => {
      child.stdin.end(source);
    },
    dispose() {
      disposed = true;
      stdout.close();
      child.stdin.destroy();
      child.kill();
    },
  };
}

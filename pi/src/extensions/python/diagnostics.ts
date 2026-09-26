import { appendFileSync, mkdirSync, renameSync, statSync } from "node:fs";
import { join } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";

interface PythonEvent {
  sessionId: string;
  phase: string;
  callId?: string;
  cwd?: string;
  pid?: number;
  durationMs?: number;
  outcome?: string;
  reason?: string;
  logPath?: string;
  interpreter?: { executable: string; version: string };
  packageCount?: number;
}

/** Record process and execution metadata without copying code, output, or variables. */
export function logPythonEvent(event: PythonEvent): void {
  try {
    const directory = join(getAgentDir(), "python");
    mkdirSync(directory, { recursive: true, mode: 0o700 });
    const path = join(directory, "events.jsonl");
    let bytes = 0;
    try {
      bytes = statSync(path).size;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
    if (bytes >= 5 * 1024 * 1024) renameSync(path, `${path}.1`);
    appendFileSync(path, `${JSON.stringify({ time: new Date().toISOString(), ...event })}\n`, { mode: 0o600 });
  } catch {
    // Diagnostic storage must not break an otherwise usable Python environment.
  }
}

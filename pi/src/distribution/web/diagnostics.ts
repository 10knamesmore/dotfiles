import { appendFileSync, mkdirSync, renameSync, statSync } from "node:fs";
import { join } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import type { WebErrorCode } from "./errors.js";
import type { SearchProvider } from "./search/types.js";

/** Request lifecycle metadata only. Queries, URLs, page bodies, headers, and credentials never enter the log. */
interface WebEvent {
  tool: "websearch" | "webfetch";
  callId: string;
  phase: "start" | "complete" | "error";
  provider?: SearchProvider;
  cached?: boolean;
  durationMs?: number;
  sourceCount?: number;
  contentChars?: number;
  errorCode?: WebErrorCode;
  httpStatus?: number;
}

/** Keep request traces off Pi's TUI terminal, retaining at most the current and previous 5 MiB log. */
export function logWebEvent(event: WebEvent): void {
  try {
    const directory = join(getAgentDir(), "web");
    mkdirSync(directory, { recursive: true, mode: 0o700 });
    const path = join(directory, "requests.jsonl");
    let bytes = 0;
    try {
      bytes = statSync(path).size;
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
    if (bytes >= 5 * 1024 * 1024) renameSync(path, `${path}.1`);
    appendFileSync(path, `${JSON.stringify({ time: new Date().toISOString(), ...event })}\n`, { mode: 0o600 });
  } catch {
    // A diagnostic write failure must not interrupt a tool call or corrupt the TUI.
  }
}

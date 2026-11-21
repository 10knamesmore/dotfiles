/** Write workflow diagnostics to a file without interfering with Pi's terminal or RPC output. */

import { appendFileSync, chmodSync, mkdirSync, renameSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { sanitizeTerminalText } from "./ui/sanitize.js";

const DIAGNOSTICS_LOG_CAP_BYTES = 5 * 1024 * 1024;

export function reportDiagnostic(message: string): void {
  const sanitized = sanitizeTerminalText(message);
  try {
    const path = join(getAgentDir(), "subagent-workflow", "diagnostics.log");
    mkdirSync(dirname(path), { recursive: true, mode: 0o700 });
    let size = 0;
    let exists = false;
    try {
      size = statSync(path).size;
      exists = true;
    } catch {
      // First write; the append below creates the file.
    }
    if (exists) chmodSync(path, 0o600);
    const line = `${new Date().toISOString()} ${sanitized}\n`;
    if (exists && size + Buffer.byteLength(line) > DIAGNOSTICS_LOG_CAP_BYTES) renameSync(path, `${path}.1`);
    appendFileSync(path, line, { mode: 0o600 });
  } catch {
    // Diagnostics must never throw, and never fall back to the TUI's terminal.
  }
}

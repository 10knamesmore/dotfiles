/**
 * Parent-process diagnostics. pi-tui owns the terminal during a TUI session:
 * a stray console.error goes to stderr on that same terminal, bypassing the
 * renderer and corrupting the frame. Every internal failure path reports
 * through here instead - to a log file during a TUI session, to stderr
 * otherwise (headless modes, where stderr is the right surface and cannot
 * corrupt anything).
 */

import { appendFileSync, chmodSync, mkdirSync, renameSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { sanitizeTerminalText } from "./ui/sanitize.js";

const DIAGNOSTICS_LOG_CAP_BYTES = 5 * 1024 * 1024;
let tuiSession = false;

/**
 * Record whether a TUI owns the terminal, and so where diagnostics may go.
 *
 * Takes a value rather than latching one-way. A one-way latch is process-wide
 * state that nothing can put back, which silently couples any two tests sharing
 * this module: once one marks a session, every later test that asserts on
 * console.error sees diagnostics go to the log file instead, and passes or fails
 * on file ordering alone.
 */
export function setTuiSession(active = true): void {
  tuiSession = active;
}

export function reportDiagnostic(message: string): void {
  const sanitized = sanitizeTerminalText(message);
  if (!tuiSession) {
    console.error(sanitized);
    return;
  }
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

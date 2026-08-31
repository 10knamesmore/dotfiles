import { truncateToWidth, type Component } from "@earendil-works/pi-tui";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage } from "../util.js";
import { sanitizeTerminalText } from "./sanitize.js";

/** Shown in place of a view that threw. Kept short, and clamped to the width. */
const NOTICE = "[subagent-workflow] render failed";

/**
 * Run a line producer so a throw degrades to a notice instead of killing pi.
 *
 * Renderers are called synchronously from inside pi's render loop, so an
 * exception there surfaces as an uncaughtException and takes the whole TUI down.
 * That alone would be bad; what makes it unrecoverable is that the input is
 * persisted. Tool-result `details` and custom entry `data` both round-trip
 * through the session JSONL, so a value that crashes the renderer once crashes
 * again on every resume, and the only way out is hand-editing the session file.
 *
 * Renderers here must therefore be total. Shape checks at each call site are the
 * first line of defence and keep the output useful; this is the backstop that
 * holds for the fields nobody thought to check, including details written by a
 * different version of this extension.
 *
 * The notice itself is width-clamped, and that is not decoration: pi-tui aborts
 * the process on any line wider than the terminal, from inside its paint and
 * after it has stopped the terminal, where nothing here can catch it. An
 * unclamped notice would turn every guarded throw into the crash this exists to
 * prevent, on any terminal narrower than the message.
 */
export function guardedLines(what: string, build: (width: number) => string[]): (width: number) => string[] {
  // A payload that throws throws on every frame, so report the first occurrence
  // and each genuinely new one, rather than filling the diagnostics log at the
  // repaint rate and rotating away the informative first hit.
  let reported: string | undefined;
  return (width: number) => {
    try {
      return build(Math.max(1, width));
    } catch (error) {
      const message = `[subagent-workflow] ${what} render failed: ${sanitizeTerminalText(errorMessage(error))}`;
      if (message !== reported) {
        reported = message;
        reportDiagnostic(message);
      }
      return [truncateToWidth(NOTICE, Math.max(1, width))];
    }
  };
}

/** A static pi-tui Component that recomputes its lines from the viewport width. */
export function linesComponent(build: (width: number) => string[], what: string): Component {
  return {
    render: guardedLines(what, build),
    invalidate: () => {},
  };
}

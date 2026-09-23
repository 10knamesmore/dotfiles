import type { AgentToolResult, Theme, ToolRenderResultOptions } from "@earendil-works/pi-coding-agent";
import { highlightCode } from "@earendil-works/pi-coding-agent";
import { Text } from "@earendil-works/pi-tui";
import { stripVTControlCharacters } from "node:util";
import type { PythonExecutionResult, PythonOutcome, PythonToolDetails } from "./session.js";

const OUTCOME_LABELS: Record<PythonOutcome, string> = {
  completed: "Completed",
  python_error: "Python error",
  interrupted: "Interrupted",
  timed_out: "Execution timed out",
  process_exited: "Python process exited",
  startup_error: "Python environment could not start",
  output_error: "Python output could not be read",
};

/** Model-facing results include state availability even when an interrupted cell made partial changes. */
export function executionText(result: PythonExecutionResult): string {
  const { details, output } = result;
  const lines = [OUTCOME_LABELS[details.outcome]];
  if (details.environmentAvailable) {
    if (details.outcome !== "completed") lines.push("The Python environment is still available. Changes made before the failure remain.");
  } else {
    lines.push("No live Python environment remains. The next call starts empty; initialize any required variables again.");
  }
  if (output) lines.push("", output);
  else if (details.outcome === "completed") lines.push("No output. Use print() to display values.");
  if (details.output.truncated) lines.push(`\n[Output truncated to the last 2000 lines / 50 KiB. Full output: ${details.output.path}]`);
  if (details.diagnosticsPath) lines.push(`\nRuntime diagnostics: ${details.diagnosticsPath}`);
  return lines.join("\n");
}

/** Show the code with Pi's Python syntax highlighting and the selected execution limit. */
export function renderPythonCall(args: { code?: string; timeout?: number }, theme: Theme, context: { expanded: boolean }): Text {
  const source = clean(args.code ?? "");
  const lines = source.split("\n");
  const preview = context.expanded ? source : lines.slice(0, 6).join("\n");
  const more = !context.expanded && lines.length > 6 ? theme.fg("dim", `\n… ${lines.length - 6} more lines`) : "";
  return new Text(
    theme.fg("toolTitle", theme.bold("python")) + theme.fg("muted", ` · ${args.timeout ?? 60}s`) +
    (preview ? `\n${highlightCode(preview, "python").join("\n")}` : "") + more,
    0, 0,
  );
}

/** UI failure labels come from execution status; infrastructure diagnostics stay in their log file. */
export function renderPythonResult(
  result: AgentToolResult<PythonToolDetails | undefined>, options: ToolRenderResultOptions, theme: Theme,
  context: { isError: boolean },
): Text {
  if (options.isPartial) {
    const text = result.content.filter(part => part.type === "text").map(part => part.text).join("\n");
    return new Text(theme.fg("muted", clean(text)), 0, 0);
  }
  const details = result.details;
  if (!details) return new Text(theme.fg(context.isError ? "error" : "muted", context.isError ? "Python call failed." : "Python call completed."), 0, 0);
  const status = `${OUTCOME_LABELS[details.outcome]} · ${(details.durationMs / 1000).toFixed(2)}s` +
    (details.environmentAvailable ? "" : " · environment unavailable");
  const color = details.outcome === "completed" ? "success" : "error";
  const text = clean(result.content.filter(part => part.type === "text").map(part => part.text).join("\n"));
  const preview = options.expanded ? text : text.split("\n").slice(1, 9).join("\n");
  return new Text(theme.fg(color, status) + (preview ? `\n${preview}` : ""), 0, 0);
}

function clean(text: string): string {
  return stripVTControlCharacters(text).replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g, "");
}

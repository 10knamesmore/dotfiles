import { stripVTControlCharacters } from "node:util";
import type { AgentToolResult, Theme, ToolRenderResultOptions } from "@earendil-works/pi-coding-agent";
import { Text } from "@earendil-works/pi-tui";
import type { SearchProvider } from "./search/types.js";

/** Small result summaries kept in the session; full fetched content belongs in the page cache. */
export type WebToolDetails =
  | { kind: "search"; provider: SearchProvider; sourceCount: number }
  | { kind: "fetch"; responseId: string; title: string; offset: number; nextOffset?: number; totalChars: number };

/** Render untrusted query text and URLs without terminal control sequences. */
export function renderWebCall(name: string, argument: string, theme: Theme): Text {
  return new Text(
    theme.fg("toolTitle", theme.bold(`${name} `)) + theme.fg("muted", displayText(argument).slice(0, 180)),
    0,
    0,
  );
}

/** UI status comes from result metadata; backend failure text is never copied into the terminal view. */
export function renderWebResult(
  result: AgentToolResult<WebToolDetails>,
  options: ToolRenderResultOptions,
  theme: Theme,
  context: { isError: boolean },
): Text {
  if (context.isError) return new Text(theme.fg("error", "Web request failed."), 0, 0);
  if (options.isPartial) return new Text(theme.fg("muted", "Loading…"), 0, 0);
  const details = result.details;
  if (!details) return new Text(theme.fg("muted", "Web request completed."), 0, 0);
  const summary =
    details.kind === "search"
      ? `${details.provider} · ${details.sourceCount} sources`
      : `${displayText(details.title)} · ${details.totalChars} characters${details.nextOffset !== undefined ? " · more available" : ""}`;
  if (!options.expanded) return new Text(theme.fg("success", summary), 0, 0);
  const text = result.content
    .filter((part) => part.type === "text")
    .map((part) => part.text)
    .join("\n");
  return new Text(`${theme.fg("success", summary)}\n${displayText(text)}`, 0, 0);
}

function displayText(text: string): string {
  return stripVTControlCharacters(text).replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g, "");
}

import { DynamicBorder, getMarkdownTheme, type ExtensionAPI, type Theme } from "@earendil-works/pi-coding-agent";
import { Box, Markdown, Spacer, Text, TruncatedText } from "@earendil-works/pi-tui";
import { DELIVERY_MESSAGE_TYPE, type DeliveryMessageDetails, type DeliveryRunResult } from "../delivery-message.js";
import { countStatuses, formatDuration, formatTokenUsage, statusGlyph } from "./format.js";
import { sanitizeTerminalText } from "./sanitize.js";
import { createExpandableMessageRenderer } from "@dotfiles/pi-ui/expandable-message";

export function registerDeliveryMessageRenderer(pi: ExtensionAPI): void {
  pi.registerMessageRenderer<DeliveryMessageDetails>(
    DELIVERY_MESSAGE_TYPE,
    createExpandableMessageRenderer<DeliveryMessageDetails>((message, { expanded, outputPad }, theme) => {
      const { runs, catchUp } = message.details!;
      const box = new Box(outputPad, 1, (text) => theme.bg("customMessageBg", text));
      let summary: string;
      if (catchUp) {
        const counts = countStatuses(
          runs.map((run) => (run.status === "completed-with-failures" ? "failed" : run.status)),
        );
        summary =
          theme.fg("accent", "↪ Recovered background deliveries") +
          theme.fg("dim", ` · ${counts.completed} completed · `) +
          theme.fg(counts.failed > 0 ? "error" : "dim", `${counts.failed} failed`) +
          theme.fg(counts.aborted > 0 ? "warning" : "dim", ` · ${counts.aborted} aborted`);
      } else {
        summary = runHeading(runs[0]!, theme);
      }
      box.addChild(new TruncatedText(`${theme.fg("muted", expanded ? "▼" : "▶")} ${summary}`, 0, 0));
      if (expanded) {
        for (const run of runs) {
          if (catchUp) {
            box.addChild(new Spacer(1));
            box.addChild(new Text(runHeading(run, theme), 0, 0));
          }
          appendRunResult(box, run, theme);
        }
      }
      return box;
    }),
  );
}

function runHeading(run: DeliveryRunResult, theme: Theme): string {
  const marker =
    run.status === "completed-with-failures"
      ? theme.fg("warning", "!")
      : statusGlyph(run.status, theme, 0, false);
  return (
    `${marker} ${theme.bold(sanitizeTerminalText(run.label))}` +
    theme.fg("dim", ` · ${run.kind} · ${run.status.replaceAll("-", " ")}`)
  );
}

function appendRunResult(container: Box, run: DeliveryRunResult, theme: Theme): void {
  appendNotices(
    container,
    run.failures,
    run.status === "aborted" ? "Stopped" : "Error",
    run.status === "aborted" ? "warning" : "error",
    theme,
  );
  appendNotices(container, run.recovery, "Recovery", "accent", theme);
  appendNotices(container, run.warnings, "Warning", "warning", theme);

  if (run.body) {
    container.addChild(new Spacer(1));
    const body = sanitizeTerminalText(run.body.text, run.body.text.length, true);
    container.addChild(
      new Markdown(run.body.format === "json" ? `\`\`\`json\n${body}\n\`\`\`` : body, 0, 0, getMarkdownTheme()),
    );
    if (run.body.truncated) {
      container.addChild(new Text(theme.fg("warning", "Result preview truncated"), 0, 0));
    }
  }

  const metadata: string[] = [];
  const execution = [
    run.model,
    run.thinkingLevel,
    ...(run.durationMs === undefined ? [] : [formatDuration(run.durationMs)]),
  ].filter((value): value is string => value !== undefined);
  if (execution.length > 0) metadata.push(execution.join(" · "));
  if (run.usage) {
    metadata.push(
      `${formatTokenUsage(run.usage)} · $${run.usage.cost.toFixed(4)} · ${run.usage.turns} ${run.usage.turns === 1 ? "turn" : "turns"}`,
    );
  }
  if (run.activity) {
    const tools = Object.entries(run.activity.tools).map(([name, count]) => `${name} × ${count}`);
    metadata.push(
      tools.length > 0
        ? `Tools: ${tools.join(" · ")}${run.activity.complete ? "" : " · incomplete"}`
        : run.activity.complete
          ? "No tool calls"
          : "Tool activity unavailable",
    );
  }
  if (run.runId) metadata.push(`Run: ${run.runId}`);
  const artifacts = run.artifacts ?? [];
  if (metadata.length > 0 || artifacts.length > 0) {
    container.addChild(new Spacer(1));
    container.addChild(new DynamicBorder((line) => theme.fg("dim", line)));
    for (const line of metadata) {
      container.addChild(new Text(theme.fg("dim", sanitizeTerminalText(line)), 0, 0));
    }
    if (artifacts.length > 0) {
      container.addChild(new Spacer(1));
      for (const artifact of artifacts) {
        container.addChild(
          new Text(theme.fg("muted", `${sanitizeTerminalText(artifact.label)}: ${sanitizeTerminalText(artifact.path)}`), 0, 0),
        );
      }
    }
  }
}

function appendNotices(
  container: Box,
  notices: readonly string[] | undefined,
  heading: string,
  color: "error" | "warning" | "accent",
  theme: Theme,
): void {
  if (!notices?.length) return;
  container.addChild(new Spacer(1));
  container.addChild(new Text(theme.fg(color, heading), 0, 0));
  for (const notice of notices) {
    container.addChild(new Text(theme.fg(color, sanitizeTerminalText(notice, notice.length, true)), 0, 0));
  }
}

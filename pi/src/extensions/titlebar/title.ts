import { basename } from "node:path";
import type { EditorActivity } from "../editor/api.js";
import { sanitizeFooterText } from "../footer/format.js";

/**
 * Prefix Pi writes in front of its own terminal title. Pi derives it from the
 * package config and the deployed package declares no `piConfig.name`, so the
 * literal default is stable for this distribution.
 */
const TITLE_PREFIX = "π";

/** Tool execution keeps a distinct spinner from model work. */
const MODEL_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as const;
const TOOL_FRAMES = ["◐", "◓", "◑", "◒"] as const;

type BusyActivity = Exclude<EditorActivity, { kind: "ready" }>;

/** Session identity Pi shows in the title. */
export interface TitleContext {
  sessionName: string | undefined;
  cwd: string;
}

/** Pi's own title, restored whenever the agent is idle. */
export function baseTitle(context: TitleContext): string {
  const directory = basename(context.cwd);
  return context.sessionName
    ? `${TITLE_PREFIX} - ${context.sessionName} - ${directory}`
    : `${TITLE_PREFIX} - ${directory}`;
}

/** Busy title: animated phase marker in front of the base title. */
export function busyTitle(activity: BusyActivity, frameIndex: number, context: TitleContext): string {
  const frames = activity.kind === "tool" ? TOOL_FRAMES : MODEL_FRAMES;
  const frame = frames[frameIndex % frames.length] ?? frames[0];
  const label =
    activity.kind === "tool"
      ? activity.toolCount > 1
        ? `TOOLS ${activity.toolCount}`
        : `TOOL · ${sanitizeFooterText(activity.toolName)}`
      : activity.kind.toUpperCase();
  return `${frame} ${label} · ${baseTitle(context)}`;
}

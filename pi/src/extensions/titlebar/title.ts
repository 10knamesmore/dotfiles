import { basename } from "node:path";

/**
 * Prefix Pi writes in front of its own terminal title. Pi derives it from the
 * package config and the deployed package declares no `piConfig.name`, so the
 * literal default is stable for this distribution.
 */
const TITLE_PREFIX = "π";

/** Animation frames per busy phase; the shapes differ so model and tool work look distinct. */
const SPINNER_FRAMES = {
  model: ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
  tool: ["◐", "◓", "◑", "◒"],
} as const;

/** Work the agent can be busy with, in the order one turn performs it. */
export type BusyPhase = keyof typeof SPINNER_FRAMES;

/** Word naming each busy phase in the title. */
const PHASE_LABELS: Record<BusyPhase, string> = {
  model: "model",
  tool: "tool",
};

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
export function busyTitle(
  phase: BusyPhase,
  frameIndex: number,
  context: TitleContext,
): string {
  const frames = SPINNER_FRAMES[phase];
  const frame = frames[frameIndex % frames.length] ?? frames[0];
  return `${frame} ${PHASE_LABELS[phase]} · ${baseTitle(context)}`;
}

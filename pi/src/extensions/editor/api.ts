import type { AgentInputNavigation } from "./navigation.js";

/** UI capabilities offered to other extensions for the current Pi session. */
export interface PromptEditorApi {
  useAgentNavigation(navigation: AgentInputNavigation): void;
  useStatus(readStatus: () => EditorStatus): void;
  useTodoStatus(readTodoStatus: () => string | undefined): void;
  useWorkflowUsage(readWorkflowUsage: () => string | undefined): void;
}

export type EditorActivity =
  | { kind: "model"; spinner: string }
  | { kind: "tool"; spinner: string; toolName: string; toolCount: number }
  | { kind: "ready" };

export interface EditorStatus {
  sessionMilliseconds: number;
  /** Time since the last fully settled run; absent before a run or while the next prompt is processing. */
  idleMilliseconds?: number;
  apiMilliseconds: number;
  tokensPerSecond?: number;
  activity: EditorActivity;
}

/** The editor requests contributions after all extension factories have registered. */
export const PROMPT_EDITOR_CONFIGURE = "dotfiles:prompt-editor:configure";

import type { FastMode } from "@dotfiles/pi-config";
import type { Theme } from "@earendil-works/pi-coding-agent";
import type { Component, TuiMouseEvent, TuiMouseEventResult } from "@earendil-works/pi-tui";

/** Keyboard-selectable work below the input editor or in the footer to its right. */
export interface InputNavigation {
  selectNext(): boolean;
  selectPrevious(): void;
  hasSelection(): boolean;
  openSelection(): boolean;
  clearSelection(): void;
  /** Expand or collapse the selected row when that navigation supports it. */
  setExpanded?(expanded: boolean): boolean;
}

/** UI capabilities offered to other extensions for the current Pi session. */
export interface PromptEditorApi {
  useAgentNavigation(navigation: InputNavigation): void;
  /** Navigation for a status entry visited after the last below-editor agent. */
  useFooterNavigation(navigation: InputNavigation): void;
  useStatus(readStatus: () => EditorStatus): void;
  useFastMode(readFastMode: () => FastMode | undefined): void;
  useWorkflowUsage(readWorkflowUsage: () => string | undefined): void;
  createReplyBody(options: ReplyBodyOptions): ReplyBodyComponent;
}

export type EditorActivity =
  | { kind: "waiting" | "thinking" | "writing" | "compacting"; spinner: string }
  | { kind: "tool"; spinner: string; toolName: string; toolCount: number }
  | { kind: "ready" };

export interface EditorStatus {
  sessionMilliseconds: number;
  /** Time since the last fully settled run; absent before a run or while the next prompt is processing. */
  idleMilliseconds?: number;
  apiMilliseconds: number;
  tokensPerSecond?: number;
  /** First generated content of the current or last ordinary model request. */
  timeToFirstTokenMilliseconds?: number;
  activity: EditorActivity;
}

/** The editor requests contributions after all extension factories have registered. */
export const PROMPT_EDITOR_CONFIGURE = "dotfiles:prompt-editor:configure";

/** Footer publishes the editor's activity when its phase or active tools change. */
export const SESSION_ACTIVITY_CHANGED = "dotfiles:session-activity:changed";

export interface ReplyBodyOptions {
  requestRender(): void;
  /** Return true when the side conversation accepts the follow-up; false keeps the action retryable. */
  sendFollowUp(text: string): boolean;
  /** Override the nested editor. The signal aborts when its OpenUI block is removed. */
  prompt?(title: string, initial: string, signal: AbortSignal): Promise<string | undefined>;
}

export interface ReplyBodyComponent extends Component {
  setText(text: string, isStreaming?: boolean): void;
  /** Returns true when an embedded block consumed the key. */
  handleInput(data: string): boolean;
  /** Use the width and full height from render(), with coordinates relative to the unscrolled body. */
  handleMouse(event: TuiMouseEvent): TuiMouseEventResult | undefined;
  dispose(): void;
}

export type ReplyBlockScope = Required<ReplyBodyOptions>;

export interface ReplyBlock extends Component {
  copyRenderedText(): string | undefined;
  handleInput?(data: string): boolean;
  dispose?(): void;
}

export type ReplyBlockRenderer = (
  source: string,
  id: string,
  theme: Theme,
  scope?: ReplyBlockScope,
) => { component: ReplyBlock } | { errors: readonly string[] };

export type RegisterReplyBlock = (language: string, render: ReplyBlockRenderer) => void;

export const REPLY_BLOCKS_CONFIGURE = "dotfiles:reply-blocks:configure";

/** The side conversation dispatches input to its own widgets while open. */
export const SIDE_CONVERSATION_OPEN = "dotfiles:side-conversation:open";

import type { FastMode } from "@dotfiles/pi-config";
import type { Theme } from "@earendil-works/pi-coding-agent";
import type { Component, TuiMouseEvent, TuiMouseEventResult } from "@earendil-works/pi-tui";
import type { CodeBlockPointer } from "./code-block.ts";

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
  readonly blockPointer: CodeBlockPointer;
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
  | { kind: "tool"; spinner: string; label: string }
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
  /** Use the width and full height from render(), with coordinates relative to the unscrolled body. */
  handleMouse(event: TuiMouseEvent): TuiMouseEventResult | undefined;
  dispose(): void;
}

export type ReplyBlockScope = Required<ReplyBodyOptions>;

export interface ReplyBlock extends Component {
  copyRenderedText(): string | undefined;
  dispose?(): void;
}

export type ReplyBlockRenderer = (
  source: string,
  id: string,
  host: ReplyBlockHost,
  scope?: ReplyBlockScope,
) => { component: ReplyBlock } | { errors: readonly string[] };

export interface StreamingBlockInput {
  /** Open snapshots retain complete line endings; closed snapshots omit the newline before the closer. */
  source: string;
  /** Terminal columns allocated to the complete block, including its frame. */
  width: number;
  phase: "open" | "closed";
}

export interface StreamingReplyBlock extends ReplyBlock {
  update(input: StreamingBlockInput): void;
  dispose(): void;
}

export interface ReplyBlockHost {
  requestRender(): void;
  theme: Theme;
  pointer: CodeBlockPointer;
}

export type ReplyBlockRegistration = ReplyBlockRenderer | { createStream(host: ReplyBlockHost): StreamingReplyBlock };

export type RegisterReplyBlock = (language: string, registration: ReplyBlockRegistration) => void;

export const REPLY_BLOCKS_CONFIGURE = "dotfiles:reply-blocks:configure";

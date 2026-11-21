import type { AssistantMessage, JsonValue, ToolCall } from "@earendil-works/pi-ai";
import {
  AssistantMessageComponent,
  getMarkdownTheme,
  ToolExecutionComponent,
  UserMessageComponent,
  type SettingsManager,
} from "@earendil-works/pi-coding-agent";
import { Container, Markdown, Text, type TUI } from "@earendil-works/pi-tui";
import { FOLLOW_UP_PROMPT_PREFIX, type TranscriptMessage } from "../../store/session-transcript.js";
import type { SubagentResult } from "../../types.js";
import { isRecord } from "../../util.js";
import {
  sanitizeTerminalText,
  sanitizeTerminalTextTailChunks,
  TerminalTextSanitizer,
  UNTRUSTED_FIELD_MAX,
} from "../sanitize.js";

type TranscriptResult = Pick<SubagentResult, "status" | "text" | "structured">;

function messageTextParts(content: unknown): string[] {
  if (typeof content === "string") return [content];
  if (!Array.isArray(content)) return [];
  return content.flatMap((part: unknown) =>
    isRecord(part) && part.type === "text" && typeof part.text === "string" ? [part.text] : [],
  );
}

function displayText(chunks: Iterable<string>): string {
  const { text, elided } = sanitizeTerminalTextTailChunks(chunks, UNTRUSTED_FIELD_MAX, true, true);
  return elided ? `… earlier content not shown\n${text}` : text;
}

function displayArguments(args: unknown): ToolCall["arguments"] {
  const json = JSON.stringify(args);
  if (!json) return {};
  if (json.length > UNTRUSTED_FIELD_MAX) return { args: displayText([json]) };
  // Pi's generic renderer displays argument keys and top-level strings verbatim when expanded.
  const clean: JsonValue = JSON.parse(json, (_, value: unknown) => {
    if (typeof value === "string") return sanitizeTerminalText(value, UNTRUSTED_FIELD_MAX, true);
    if (!isRecord(value)) return value;
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [sanitizeTerminalText(key), item]));
  });
  return isRecord(clean) ? (clean as ToolCall["arguments"]) : { args: clean };
}

function displayAssistant(message: TranscriptMessage): AssistantMessage {
  const sanitizer = new TerminalTextSanitizer();
  const content: AssistantMessage["content"] = [];
  const parts = typeof message.content === "string" ? [{ type: "text", text: message.content }] : message.content;
  if (Array.isArray(parts)) {
    for (const part of parts as unknown[]) {
      if (!isRecord(part)) continue;
      if (part.type === "toolCall" && typeof part.id === "string" && typeof part.name === "string") {
        content.push({
          type: "toolCall",
          id: part.id,
          name: sanitizeTerminalText(part.name),
          arguments: displayArguments(part.arguments),
        });
      } else if (part.type === "text" || part.type === "thinking") {
        const raw = part.type === "text" ? part.text : part.thinking;
        if (typeof raw !== "string") continue;
        const { text, elided } = sanitizer.sanitizeTail(raw, UNTRUSTED_FIELD_MAX, true);
        const value = elided ? `… earlier content not shown\n${text}` : text;
        content.push(part.type === "text" ? { type: "text", text: value } : { type: "thinking", thinking: value });
      }
    }
  }
  const source = message as AssistantMessage;
  const errorMessage = typeof source.errorMessage === "string" ? displayText([source.errorMessage]) : undefined;
  return { ...source, content, errorMessage };
}

function sameResponse(a: TranscriptMessage, b: TranscriptMessage): boolean {
  if (a.responseId && b.responseId) return a.responseId === b.responseId;
  return a.timestamp !== undefined && a.timestamp === b.timestamp;
}

function unpublishedAssistant(current: AssistantMessage | undefined, latest: TranscriptMessage | undefined) {
  if (!current || !latest) return current;
  if (current.responseId && latest.responseId) return current.responseId === latest.responseId ? undefined : current;
  if (latest.timestamp === undefined || current.timestamp > latest.timestamp) return current;
  if (current.timestamp === latest.timestamp && JSON.stringify(current.content) !== JSON.stringify(latest.content))
    return current;
  return undefined;
}

/** Binds the child-written history and current RPC response to Pi's message components. */
export class AgentTranscript extends Container {
  private readonly history = new Container();
  private readonly stream = new Container();
  private readonly outcome = new Container();
  private messages?: readonly TranscriptMessage[];
  private lastAssistant?: TranscriptMessage;
  private live?: boolean;
  private structured = false;
  private current?: AssistantMessage;
  private streamingComponent?: AssistantMessageComponent;
  private resultKey = "";
  private toolsExpanded = false;
  private hideThinking: boolean;
  private readonly markdownTheme;
  private readonly outputPad;

  constructor(
    private readonly tui: TUI,
    private readonly cwd: () => string,
    settings: SettingsManager,
  ) {
    super();
    this.hideThinking = settings.getHideThinkingBlock();
    this.markdownTheme = { ...getMarkdownTheme(), codeBlockIndent: settings.getCodeBlockIndent() };
    this.outputPad = settings.getOutputPad();
    this.addChild(this.history);
    this.addChild(this.stream);
    this.addChild(this.outcome);
  }

  update(
    messages: readonly TranscriptMessage[],
    current: AssistantMessage | undefined,
    live: boolean,
    result: TranscriptResult | undefined,
  ): void {
    const final = result?.status === "completed" ? result : undefined;
    const structured = final !== undefined && Object.hasOwn(final, "structured");
    if (messages !== this.messages || live !== this.live || structured !== this.structured) {
      this.messages = messages;
      this.live = live;
      this.structured = structured;
      this.lastAssistant = undefined;
      for (let index = messages.length - 1; index >= 0; index--) {
        const message = messages[index]!;
        if (message.role !== "assistant" || message.omitted) continue;
        this.lastAssistant = message;
        break;
      }
      this.renderHistory(messages, live);
    }

    const streaming = unpublishedAssistant(current, this.lastAssistant);
    if (streaming !== this.current) {
      this.stream.clear();
      if (streaming) {
        if (!this.current || !sameResponse(this.current, streaming)) this.streamingComponent = this.assistant();
        const component = this.streamingComponent!;
        const message = displayAssistant(streaming);
        component.updateContent(message, true);
        this.stream.addChild(component);
        for (const part of message.content) {
          if (part.type === "toolCall") this.stream.addChild(this.tool(part.name, part.id, part.arguments));
        }
      } else this.streamingComponent = undefined;
      this.current = streaming;
    }

    const finalText = structured
      ? (JSON.stringify(final.structured, null, 2) ?? "")
      : final?.text && final.text !== messageTextParts(this.lastAssistant?.content).join("\n")
        ? final.text
        : "";
    const resultKey = structured ? `json:${finalText}` : finalText;
    if (resultKey === this.resultKey) return;
    this.resultKey = resultKey;
    this.outcome.clear();
    if (finalText) {
      const text = displayText([finalText]);
      this.outcome.addChild(
        structured
          ? new Text(`Result\n${text}`, this.outputPad, 0)
          : new Markdown(text, this.outputPad, 0, this.markdownTheme),
      );
    }
  }

  toggleTools(): void {
    this.toolsExpanded = !this.toolsExpanded;
    for (const component of [...this.history.children, ...this.stream.children]) {
      if (component instanceof ToolExecutionComponent) component.setExpanded(this.toolsExpanded);
    }
  }

  toggleThinking(): void {
    this.hideThinking = !this.hideThinking;
    for (const component of [...this.history.children, ...this.stream.children]) {
      if (component instanceof AssistantMessageComponent) component.setHideThinkingBlock(this.hideThinking);
    }
  }

  private renderHistory(messages: readonly TranscriptMessage[], live: boolean): void {
    this.history.clear();
    const tools = new Map<string, ToolExecutionComponent>();
    for (const message of messages) {
      if (message.omitted || message.role === "omission") {
        this.history.addChild(new Text(displayText(messageTextParts(message.content)), this.outputPad, 0));
      } else if (message.role === "user") {
        const raw = messageTextParts(message.content).join("\n");
        const text = displayText([
          raw.startsWith(FOLLOW_UP_PROMPT_PREFIX) ? raw.slice(FOLLOW_UP_PROMPT_PREFIX.length) : raw,
        ]);
        if (text.trim()) this.history.addChild(new UserMessageComponent(text, this.markdownTheme, this.outputPad));
      } else if (message.role === "assistant") {
        const assistant = displayAssistant(message);
        // A successful report_result stops the child's remaining turn via RPC abort.
        if (this.structured && message === this.lastAssistant && assistant.stopReason === "aborted")
          assistant.stopReason = "stop";
        const component = this.assistant();
        component.updateContent(assistant);
        this.history.addChild(component);
        for (const part of assistant.content) {
          if (part.type !== "toolCall") continue;
          const tool = this.tool(part.name, part.id, part.arguments);
          this.history.addChild(tool);
          tools.set(part.id, tool);
          if (assistant.stopReason === "aborted" || assistant.stopReason === "error") {
            tool.updateResult({
              content: [
                {
                  type: "text",
                  text: assistant.errorMessage ?? (assistant.stopReason === "aborted" ? "Operation aborted" : "Error"),
                },
              ],
              isError: true,
            });
          } else if (!live || message !== this.lastAssistant) {
            tool.updateResult({ content: [{ type: "text", text: "Result unavailable" }], isError: false }, true);
          }
        }
      } else if (message.role === "toolResult" && typeof message.isError === "boolean") {
        const id = message.toolCallId ?? "";
        const match = tools.get(id);
        const name = typeof message.toolName === "string" ? sanitizeTerminalText(message.toolName) : "tool";
        const tool = match ?? this.tool(name, id);
        if (!match) this.history.addChild(tool);
        tool.updateResult({
          content: [{ type: "text", text: displayText(messageTextParts(message.content)) }],
          isError: message.isError,
        });
      }
    }
  }

  private assistant(): AssistantMessageComponent {
    return new AssistantMessageComponent(undefined, this.hideThinking, this.markdownTheme, undefined, this.outputPad);
  }

  private tool(name: string, id: string, args?: ToolCall["arguments"]): ToolExecutionComponent {
    // An empty renderer definition selects Pi's expandable generic call and result.
    const tool = new ToolExecutionComponent(
      name,
      id,
      args,
      { outputPad: this.outputPad, showImages: false },
      {},
      this.tui,
      this.cwd(),
    );
    if (this.toolsExpanded) tool.setExpanded(true);
    return tool;
  }
}

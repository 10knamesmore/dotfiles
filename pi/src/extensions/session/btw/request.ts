import { currentFastMode, openAIServiceTier } from "@dotfiles/pi-config/openai-service-tier";
import { contentText, toToolDeclaration, type Context, type Message } from "@earendil-works/pi-ai";
import {
  convertToLlm,
  type ContextWithSystemEvent,
  type ExtensionAPI,
  type ExtensionContext,
} from "@earendil-works/pi-coding-agent";

export interface Exchange {
  question: string;
  answer: string;
  askedAt: number;
}

interface MainRequest {
  sessionId: string;
  leafId: string;
  messages: ContextWithSystemEvent["messages"];
}

const SIDE_INSTRUCTIONS = [
  "Answer the current side question in Markdown. Use the main conversation above as context; previousSideExchanges belong to this separate thread.",
  "Tools are unavailable. If you need information that the conversation does not provide, say what is missing.",
  "When a visual helps, use a top-level fenced `openui` block. Start with `root = Stack([...])`. Write one `name = Expression` per line, with double-quoted strings, bracketed arrays, and positional component arguments. Use `null` to skip an optional argument. Every named expression other than root must be referenced; forward references are allowed.",
  `For example:
\`\`\`openui
root = Stack([flow])
flow = Dag([DagNode("in", "Input"), DagNode("out", "Output")], [["in", "out"]], "LR", "Data flow")
\`\`\`
Each Dag edge is exactly [fromId, toId]. For labeled arrows, use a SequenceDiagram with SeqMessage(fromId, toId, label).`,
  `Other component forms (square-bracketed parameters are optional):
Text(text [, variant])
Table(columnNames, rows)
BarChart(labels, values [, title, unit])
LineChart(labels, [Series(name, values)])
Erd([Entity(name, fieldDescriptions)], [Relation(fromName, toName, "1:N")])
SequenceDiagram([SeqParticipant(id, label)], [SeqMessage(fromId, toId, label)])
InfraMap([InfraZone(name, [InfraNode(id, label)])], [InfraLink(fromId, toId, label)] [, title, direction])
Button(label, @ToAssistant(followUpText))
Choice(question, options)
Actions inside a side reply send their follow-up to the side conversation.`,
].join("\n\n");

function readMainMessages(ctx: ExtensionContext, main: MainRequest | undefined): MainRequest["messages"] {
  const session = ctx.sessionManager;
  const projection = session.buildSessionProjection();
  if (main?.sessionId === session.getSessionId()) {
    if (main.leafId === session.getLeafId()) return main.messages;
    const branch = session.getBranch();
    const anchor = branch.findIndex((entry) => entry.id === main.leafId);
    const tail = branch.slice(anchor + 1);
    // A compaction or edit after the captured request can replace messages before the anchor.
    if (anchor >= 0 && tail.every((entry) => entry.type !== "compaction" && entry.type !== "context_edit")) {
      const ids = new Set(tail.map((entry) => entry.id));
      return [...main.messages, ...projection.entries.flatMap(({ sourceEntry, messages }) => ids.has(sourceEntry.id) ? messages : [])];
    }
  }
  return projection.messages;
}

function withoutImages(message: Message): Message {
  if (
    (message.role !== "user" && message.role !== "toolResult") ||
    typeof message.content === "string" ||
    !message.content.some((block) => block.type === "image")
  ) return message;

  const placeholder = "Image reading is disabled.";
  const content = message.content
    .map((block) => block.type === "image" ? { type: "text" as const, text: placeholder } : block)
    .filter((block, index, blocks) => {
      const previous = blocks[index - 1];
      return !(block.text === placeholder && previous?.text === placeholder);
    });
  return { ...message, content };
}

export function registerSideRequests(pi: ExtensionAPI): (
  ctx: ExtensionContext,
  question: string,
  history: readonly Exchange[],
  signal: AbortSignal,
  onText: (text: string) => void,
) => Promise<string> {
  let main: MainRequest | undefined;
  const clearMain = () => { main = undefined; };
  pi.on("session_start", clearMain);
  pi.on("session_tree", clearMain);
  pi.on("session_shutdown", clearMain);
  pi.on("context_with_system", (event, ctx) => {
    if (ctx.mode !== "tui") return;
    const leafId = ctx.sessionManager.getLeafId();
    if (leafId) main = { sessionId: ctx.sessionManager.getSessionId(), leafId, messages: event.messages };
  });

  return async (ctx, question, history, signal, onText) => {
    const canceled = () => new Error("Side question canceled.");
    if (signal.aborted) throw canceled();
    const model = ctx.model;
    if (!model) throw new Error("Select a model before asking a side question.");

    const settings = pi.getSettings();
    const transcript = convertToLlm(readMainMessages(ctx, main));
    const messages = settings.images?.blockImages ? transcript.map(withoutImages) : transcript;
    const context: Context = { messages };
    if (messages[0]?.role !== "system") {
      const definitions = new Map(pi.getAllTools().map((tool) => [tool.name, tool]));
      context.systemPrompt = ctx.getSystemPrompt();
      context.tools = pi.getActiveTools().flatMap((name) => {
        const tool = definitions.get(name);
        return tool ? [toToolDeclaration(tool)] : [];
      });
    }

    const previousSideExchanges = history.slice(-20).map(({ question, answer, askedAt }) => ({
      askedAt: new Date(askedAt).toISOString(), question, answer,
    }));
    messages.push({
      role: "user",
      content: `${SIDE_INSTRUCTIONS}\n\n${JSON.stringify({ previousSideExchanges, question })}`,
      timestamp: Date.now(),
    });

    const fastMode = currentFastMode(ctx);
    const thinkingLevel = ctx.thinkingLevel ?? pi.getThinkingLevel();
    const requestAbort = new AbortController();
    try {
      const stream = ctx.modelRegistry.streamSimple(model, context, {
        signal: AbortSignal.any([signal, requestAbort.signal]),
        sessionId: ctx.sessionManager.getSessionId(),
        disableContinuation: true,
        // Codex WebSocket continuation uses the session's cache key for its connection state.
        transport: model.api === "openai-codex-responses" ? "sse" : settings.transport,
        reasoning: thinkingLevel === "off" ? undefined : thinkingLevel,
        thinkingBudgets: settings.thinkingBudgets,
        onPayload: fastMode
          ? (payload) => ({ ...(payload as object), service_tier: openAIServiceTier(fastMode) })
          : undefined,
      });

      let displayed = "";
      const publish = (text: string) => {
        if (text === displayed) return;
        displayed = text;
        onText(text);
      };
      const blocks: string[] = [];
      const toolUse = () => new Error("The side question model attempted to call a tool.");
      for await (const event of stream) {
        if (signal.aborted) throw canceled();
        switch (event.type) {
          case "toolcall_start":
          case "toolcall_delta":
          case "toolcall_end":
            throw toolUse();
          case "text_delta":
            blocks[event.contentIndex] = (blocks[event.contentIndex] ?? "") + event.delta;
            publish(Object.values(blocks).join("\n"));
            break;
          case "text_end":
            blocks[event.contentIndex] = event.content;
            publish(Object.values(blocks).join("\n"));
            break;
        }
      }

      const result = await stream.result();
      if (signal.aborted || result.stopReason === "aborted") throw canceled();
      if (result.stopReason === "toolUse" || result.content.some((block) => block.type === "toolCall")) throw toolUse();
      if (result.stopReason === "length") throw new Error("The side answer reached the model’s output limit.");
      if (result.stopReason !== "stop") {
        throw new Error(result.errorMessage || `The side answer did not complete (${result.stopReason}).`);
      }
      const answer = contentText(result.content);
      if (!answer.trim()) throw new Error("The model returned no text for the side question.");
      publish(answer);
      if (signal.aborted) throw canceled();
      return answer;
    } catch (error) {
      if (signal.aborted) throw canceled();
      throw error instanceof Error ? error : new Error(String(error));
    } finally {
      requestAbort.abort();
    }
  };
}

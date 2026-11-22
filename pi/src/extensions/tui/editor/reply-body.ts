import {
  AssistantMessageComponent,
  copyToClipboard,
  ExtensionEditorComponent,
  getMarkdownTheme,
  type ExtensionContext,
} from "@earendil-works/pi-coding-agent";
import {
  Container,
  Markdown,
  sliceByColumn,
  truncateToWidth,
  visibleWidth,
  type Component,
  type TUI,
  type TuiMouseEvent,
} from "@earendil-works/pi-tui";
import {
  CODE_COPY_MARKER,
  REPLY_BLOCK_MARKER,
  type CodeHighlight,
  type ReplyCodeBlock,
  type ReplyMarkdown,
} from "./reply-markdown.js";
import { ACTION_STYLES, COPIED_LABEL, COPY_LABEL, FEEDBACK_DURATION_MS } from "@dotfiles/pi-ui/action-feedback";
import { PaddedCodeBlock, type CodeBlockPointer } from "@dotfiles/pi-ui/code-block";
import type {
  ReplyBlock,
  ReplyBlockHost,
  ReplyBlockRegistration,
  ReplyBlockRenderer,
  ReplyBlockScope,
  ReplyBodyComponent,
  ReplyBodyOptions,
  StreamingBlockInput,
  StreamingReplyBlock,
} from "@dotfiles/pi-ui/prompt-editor";

interface CopyButton {
  row: number;
  column: number;
  width: number;
  code: string;
}

interface ButtonReference {
  owner: CodeButtons;
  index: number;
}

interface ButtonFeedback {
  hovered: ButtonReference | undefined;
  copied: ButtonReference | undefined;
  hover(button: ButtonReference | undefined): void;
  copy(button: ButtonReference, code: string): Promise<void>;
  style(label: string, copied: boolean): string;
}

class CodeButtons implements Component {
  private readonly displayLines: string[];
  private readonly buttons: CopyButton[] = [];

  constructor(
    lines: readonly string[],
    private readonly feedback: ButtonFeedback,
  ) {
    this.displayLines = lines.map((line, row) =>
      line.replace(CODE_COPY_MARKER, (_marker, columns: string, encoded: string, offset: number) => {
        this.buttons.push({
          row,
          column: visibleWidth(line.slice(0, offset)),
          width: Number(columns),
          code: Buffer.from(encoded, "base64").toString("utf8"),
        });
        return "";
      }),
    );
  }

  render(): string[] {
    const { hovered, copied } = this.feedback;
    if (hovered?.owner !== this && copied?.owner !== this) return this.displayLines;
    const display = [...this.displayLines];
    this.buttons.forEach((button, index) => {
      const isCopied = copied?.owner === this && copied.index === index;
      const isHovered = hovered?.owner === this && hovered.index === index;
      if (!isCopied && !isHovered) return;
      const line = display[button.row]!;
      const label = truncateToWidth(isCopied ? COPIED_LABEL : COPY_LABEL, button.width, "");
      display[button.row] =
        sliceByColumn(line, 0, button.column) +
        this.feedback.style(label, isCopied) +
        sliceByColumn(line, button.column + button.width, visibleWidth(line) - button.column - button.width);
    });
    return display;
  }

  handleMouse(event: TuiMouseEvent): ReturnType<Container["handleMouse"]> {
    const index = this.buttons.findIndex(
      (target) => event.y === target.row && event.x >= target.column && event.x < target.column + target.width,
    );
    const button = index === -1 ? undefined : { owner: this, index };
    if (event.type === "move" || event.type === "press") this.feedback.hover(button);
    if (button) {
      if (event.type === "click" && event.button === "left") {
        void this.feedback.copy(button, this.buttons[index]!.code);
      }
      return {
        handled: true,
        target: {
          component: this,
          originX: event.screenX - event.x,
          originY: event.screenY - event.y,
          width: event.width,
          height: event.height,
        },
      };
    }
    return undefined;
  }

  invalidate(): void {}
}

let nextReplyBlockId = 0;
type BlockIdentity = Pick<ReplyCodeBlock, "key" | "occurrence">;
type CachedReplyBlock =
  ReturnType<ReplyBlockRenderer> | { component: StreamingReplyBlock; input: StreamingBlockInput | undefined };

interface TranscriptText {
  text: string;
  isStreaming: boolean;
}

export class ReplyBody extends Container {
  private sourceLines: readonly string[] | undefined;
  private readonly blockCache = new Map<string, CachedReplyBlock>();
  private inlineBlocks: { identity: BlockIdentity; component: ReplyBlock }[] = [];
  private highlight: CodeHighlight | undefined;
  private paint: CodeHighlight["paint"] | undefined;
  private transcript: TranscriptText | undefined;
  private disposed = false;

  constructor(
    public markdown: Markdown,
    private readonly feedback: ButtonFeedback,
    private readonly renderer: ReplyMarkdown,
    private readonly blockRenderers: ReadonlyMap<string, ReplyBlockRegistration>,
    private readonly host: ReplyBlockHost,
    private readonly scope?: ReplyBlockScope,
  ) {
    super();
    markdown.invalidate();
  }

  bindTranscript(markdown: Markdown, transcript: TranscriptText): void {
    if (
      this.markdown !== markdown ||
      this.transcript?.text !== transcript.text ||
      this.transcript.isStreaming !== transcript.isStreaming
    ) {
      // Trimmed Markdown can stay identical while the fence gains a newline or the reply ends.
      markdown.setText(transcript.text.trim());
    }
    this.markdown = markdown;
    this.transcript = transcript;
  }

  override render(width: number): readonly string[] {
    if (this.disposed) return [];
    const nextBlocks: typeof this.inlineBlocks = [];
    const usedKeys = new Set<string>();
    const lines = this.renderer.renderBody(
      (block) => {
        const registration = this.blockRenderers.get(block.language);
        if (!registration) return undefined;
        const { phase, source, width: frameWidth } = block;
        const closedOnly = typeof registration === "function";
        if (closedOnly && phase === "open") return undefined;
        if (!closedOnly && !block.isStreaming && phase === "open") return { rawSource: source };
        if (!closedOnly && phase === "open" && !source.trim()) return undefined;

        const key = JSON.stringify(closedOnly ? [block.key, block.occurrence] : [block.ordinal, block.language]);
        usedKeys.add(key);
        let result = this.blockCache.get(key);
        if (!result) {
          const host: ReplyBlockHost = {
            ...this.host,
            requestRender: () => {
              if (!this.disposed && result && this.blockCache.get(key) === result) this.host.requestRender();
            },
          };
          result = closedOnly
            ? registration(block.text, `reply-block-${++nextReplyBlockId}`, host, this.scope)
            : { component: registration.createStream(host), input: undefined };
          this.blockCache.set(key, result);
        }
        if ("errors" in result) return result;
        if ("input" in result) {
          const previous = result.input;
          if (previous?.source !== source || previous.width !== frameWidth || previous.phase !== phase) {
            const input = { source, width: frameWidth, phase };
            result.component.update(input);
            result.input = input;
          }
        }
        const slot = nextBlocks.length;
        nextBlocks.push({
          identity: { key: block.key, occurrence: block.occurrence },
          component: result.component,
        });
        return { slot };
      },
      this.highlight,
      this.transcript?.text,
      () => this.markdown.render(width),
    );
    if (lines !== this.sourceLines) {
      for (const [key, block] of this.blockCache) {
        if (usedKeys.has(key)) continue;
        this.blockCache.delete(key);
        if ("component" in block) block.component.dispose?.();
      }
      this.sourceLines = lines;
      this.inlineBlocks = nextBlocks;
      this.clear();
      let start = 0;
      lines.forEach((line, row) => {
        const marker = REPLY_BLOCK_MARKER.exec(line);
        const block = marker && nextBlocks[Number(marker[1])];
        if (!marker || !block) return;
        if (start < row) this.addChild(new CodeButtons(lines.slice(start, row), this.feedback));
        const highlight = this.highlight;
        const { key, occurrence } = block.identity;
        const paint = highlight?.key === key && highlight.occurrence === occurrence ? highlight.paint : undefined;
        const paddingX = visibleWidth(line.slice(0, marker.index));
        this.addChild(new PaddedCodeBlock(block.component, paddingX, paint));
        start = row + 1;
      });
      if (start < lines.length) this.addChild(new CodeButtons(lines.slice(start), this.feedback));
    }
    const rendered = super.render(width);
    return this.paint ? rendered.map(this.paint) : rendered;
  }

  setPaint(paint: CodeHighlight["paint"] | undefined): void {
    this.paint = paint;
  }

  override handleMouse(event: TuiMouseEvent): ReturnType<Container["handleMouse"]> {
    const result = super.handleMouse(event);
    if (event.type === "move" || event.type === "press") {
      if (!(result?.target?.component instanceof CodeButtons)) this.feedback.hover(undefined);
      if (!result || result.target.component instanceof CodeButtons) this.host.pointer.leave?.();
    }
    return result;
  }

  copyRenderedBlock({ key, occurrence }: BlockIdentity): string | undefined {
    return this.inlineBlocks
      .find(({ identity }) => identity.key === key && identity.occurrence === occurrence)
      ?.component.copyRenderedText();
  }

  setHighlight(highlight: CodeHighlight | undefined): void {
    if (
      this.highlight?.key === highlight?.key &&
      this.highlight?.occurrence === highlight?.occurrence &&
      this.highlight?.paint === highlight?.paint
    )
      return;
    this.highlight = highlight;
    this.markdown.invalidate();
  }

  override invalidate(): void {
    this.markdown.invalidate();
    super.invalidate();
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    for (const block of this.blockCache.values()) {
      if ("component" in block) block.component.dispose?.();
    }
    this.blockCache.clear();
    this.inlineBlocks = [];
    this.clear();
  }
}

function createButtonFeedback(requestRender: () => void, ui: ExtensionContext["ui"]) {
  let disposed = false;
  let resetCopied: ReturnType<typeof setTimeout> | undefined;
  const feedback: ButtonFeedback = {
    hovered: undefined,
    copied: undefined,
    hover(button) {
      if (feedback.hovered?.owner === button?.owner && feedback.hovered?.index === button?.index) return;
      feedback.hovered = button;
      requestRender();
    },
    async copy(button, code) {
      try {
        await copyToClipboard(code);
        if (disposed) return;
        feedback.copied = button;
        if (resetCopied) clearTimeout(resetCopied);
        resetCopied = setTimeout(() => {
          feedback.copied = undefined;
          resetCopied = undefined;
          requestRender();
        }, FEEDBACK_DURATION_MS);
        requestRender();
      } catch {
        if (!disposed) ui.notify("Could not write to the clipboard. Check clipboard access and try again.", "error");
      }
    },
    style: (label, copied) => ui.theme.style(label, ACTION_STYLES[copied ? "success" : "active"]),
  };
  return {
    feedback,
    dispose() {
      disposed = true;
      if (resetCopied) clearTimeout(resetCopied);
      feedback.hovered = undefined;
      feedback.copied = undefined;
    },
  };
}

async function promptInOverlay(
  ui: ExtensionContext["ui"],
  title: string,
  initial: string,
  signal: AbortSignal,
): Promise<string | undefined> {
  if (signal.aborted) return undefined;
  let cancel: (() => void) | undefined;
  const onAbort = () => cancel?.();
  try {
    return await ui.custom<string | undefined>(
      (tui, _theme, keys, done) => {
        cancel = () => done(undefined);
        return new ExtensionEditorComponent(tui, keys, title, initial, done, cancel);
      },
      {
        overlay: true,
        blocked: { kind: "question", message: title },
        onHandle: () => {
          if (signal.aborted) onAbort();
          else signal.addEventListener("abort", onAbort, { once: true });
        },
      },
    );
  } finally {
    signal.removeEventListener("abort", onAbort);
  }
}

export function createReplyBody(
  options: ReplyBodyOptions,
  ui: ExtensionContext["ui"],
  renderer: ReplyMarkdown,
  blockRenderers: ReadonlyMap<string, ReplyBlockRegistration>,
  pointer: CodeBlockPointer,
  outputPad: number,
): ReplyBodyComponent {
  let disposed = false;
  const scope: ReplyBlockScope = {
    requestRender: () => {
      if (!disposed) options.requestRender();
    },
    sendFollowUp: (text) => !disposed && options.sendFollowUp(text),
    prompt: (title, initial, cancelled) =>
      options.prompt ? options.prompt(title, initial, cancelled) : promptInOverlay(ui, title, initial, cancelled),
  };
  const buttons = createButtonFeedback(scope.requestRender, ui);
  let streaming = false;
  let text = "";
  const markdown = new Markdown("", outputPad, 0, getMarkdownTheme(), undefined, {
    transform: (source, availableWidth) =>
      renderer.transform(source, {
        messageType: "assistant",
        isStreaming: streaming,
        availableWidth,
      }),
  });
  const body = new ReplyBody(
    markdown,
    buttons.feedback,
    renderer,
    blockRenderers,
    { theme: ui.theme, pointer, requestRender: scope.requestRender },
    scope,
  );
  return {
    setText(next, isStreaming = false) {
      if (disposed || (next === text && isStreaming === streaming)) return;
      text = next;
      streaming = isStreaming;
      markdown.setText(next);
      scope.requestRender();
    },
    render: (width) => (disposed ? [] : body.render(width)),
    handleMouse: (event) => (disposed ? undefined : body.handleMouse(event)),
    invalidate: () => body.invalidate(),
    dispose() {
      if (disposed) return;
      disposed = true;
      body.dispose();
      buttons.dispose();
    },
  };
}

/** The widget owns the wrappers for the current transcript. */
export function createReplyBodies(
  tui: TUI,
  ctx: ExtensionContext,
  renderer: ReplyMarkdown,
  blockRenderers: ReadonlyMap<string, ReplyBlockRegistration>,
  pointer: CodeBlockPointer,
) {
  const requestRender = () => tui.requestRender();
  const host: ReplyBlockHost = { theme: ctx.ui.theme, requestRender, pointer };
  const buttons = createButtonFeedback(requestRender, ctx.ui);
  const replies = new Map<AssistantMessageComponent, { children: Component[]; bodies: Map<number, ReplyBody> }>();

  const attach = (reply: AssistantMessageComponent) => {
    const content = reply.children[0];
    if (!(content instanceof Container)) return;
    const previous = replies.get(reply);
    if (previous?.children === content.children) return;

    // Pi trims each text part and wraps its thinking Markdown separately.
    type Message = Parameters<AssistantMessageComponent["updateContent"]>[0];
    const { lastMessage: message, isStreaming } = reply as unknown as {
      readonly lastMessage?: Message;
      readonly isStreaming: boolean;
    };
    if (!message) return;
    const bodies = new Map<number, ReplyBody>();
    const parts = message.content.entries();
    content.children.forEach((child, row) => {
      const markdown = child instanceof ReplyBody ? child.markdown : child;
      if (!(markdown instanceof Markdown)) return;
      for (const [index, part] of parts) {
        if (part.type !== "text" || !part.text.trim()) continue;
        const body =
          previous?.bodies.get(index) ?? new ReplyBody(markdown, buttons.feedback, renderer, blockRenderers, host);
        body.bindTranscript(markdown, { text: part.text, isStreaming });
        bodies.set(index, body);
        content.children[row] = body;
        return;
      }
    });
    if (previous) {
      for (const [index, body] of previous.bodies) if (!bodies.has(index)) body.dispose();
    }
    replies.set(reply, { children: content.children, bodies });
  };

  const detach = (reply: AssistantMessageComponent, bodies: Map<number, ReplyBody>) => {
    const content = reply.children[0];
    if (content instanceof Container) {
      content.children.forEach((child, index) => {
        if (child instanceof ReplyBody) {
          content.children[index] = child.markdown;
          child.markdown.invalidate();
        }
      });
    }
    for (const body of bodies.values()) body.dispose();
    replies.delete(reply);
  };

  // Pi mounts the document first in both TUI modes. Observe it before its chat rows render:
  // updateContent rebuilds the assistant's children even when it reuses their Markdown.
  const document = tui.children[0];
  const reconcile: Component = {
    render(): string[] {
      if (!(document instanceof Container)) return [];
      const removed = new Set(replies.keys());
      const visit = (parent: Container) => {
        for (const child of parent.children) {
          if (child instanceof AssistantMessageComponent) {
            removed.delete(child);
            attach(child);
          } else if (child instanceof Container) {
            visit(child);
          }
        }
      };
      visit(document);
      for (const reply of removed) detach(reply, replies.get(reply)!.bodies);
      return [];
    },
    invalidate() {},
  };
  if (document instanceof Container) document.children.unshift(reconcile);
  return {
    render: () => [],
    invalidate() {},
    dispose() {
      if (document instanceof Container) document.removeChild(reconcile);
      buttons.dispose();
      for (const [reply, { bodies }] of replies) detach(reply, bodies);
    },
  };
}

import {
  AssistantMessageComponent,
  copyToClipboard,
  ExtensionEditorComponent,
  getMarkdownTheme,
  type ExtensionContext,
  type Theme,
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
import type {
  ReplyBlock,
  ReplyBlockRenderer,
  ReplyBlockScope,
  ReplyBodyComponent,
  ReplyBodyOptions,
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

function highlightBlock(component: ReplyBlock, paint: CodeHighlight["paint"]): Component {
  return {
    render: (width) => component.render(width).map(paint),
    invalidate: () => component.invalidate(),
    handleMouse: (event) => component.handleMouse?.(event),
  };
}

export class ReplyBody extends Container {
  private sourceLines: readonly string[] | undefined;
  private blockCache = new Map<string, ReturnType<ReplyBlockRenderer>>();
  private inlineBlocks: { identity: BlockIdentity; component: ReplyBlock }[] = [];
  private highlight: CodeHighlight | undefined;
  private paint: CodeHighlight["paint"] | undefined;

  constructor(
    readonly markdown: Markdown,
    private readonly feedback: ButtonFeedback,
    private readonly renderer: ReplyMarkdown,
    private readonly blockRenderers: ReadonlyMap<string, ReplyBlockRenderer>,
    private readonly theme: Theme,
    private readonly scope?: ReplyBlockScope,
  ) {
    super();
  }

  override render(width: number): readonly string[] {
    const nextBlocks: typeof this.inlineBlocks = [];
    const usedKeys = this.scope && new Set<string>();
    const lines = this.renderer.renderBody(
      (block) => {
        const render = this.blockRenderers.get(block.language);
        if (!render) return undefined;
        const key = JSON.stringify([block.key, block.occurrence]);
        usedKeys?.add(key);
        let result = this.blockCache.get(key);
        if (!result) {
          result = render(block.text, `reply-block-${++nextReplyBlockId}`, this.theme, this.scope);
          this.blockCache.set(key, result);
        }
        if ("errors" in result) return result;
        const slot = nextBlocks.length;
        nextBlocks.push({ identity: block, component: result.component });
        return { slot };
      },
      this.highlight,
      () => this.markdown.render(width),
    );
    if (lines !== this.sourceLines) {
      if (usedKeys) {
        for (const [key, block] of this.blockCache) {
          if (usedKeys.has(key)) continue;
          if ("component" in block) block.component.dispose?.();
          this.blockCache.delete(key);
        }
      }
      this.sourceLines = lines;
      this.inlineBlocks = nextBlocks;
      this.clear();
      let start = 0;
      lines.forEach((line, row) => {
        const marker = REPLY_BLOCK_MARKER.exec(line);
        const block = marker && nextBlocks[Number(marker[1])];
        if (!block) return;
        if (start < row) this.addChild(new CodeButtons(lines.slice(start, row), this.feedback));
        const highlight = this.highlight;
        const { key, occurrence } = block.identity;
        this.addChild(
          highlight?.key === key && highlight.occurrence === occurrence
            ? highlightBlock(block.component, highlight.paint)
            : block.component,
        );
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
    if ((event.type === "move" || event.type === "press") && !(result?.target?.component instanceof CodeButtons))
      this.feedback.hover(undefined);
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

  handleBlockInput(data: string): boolean {
    return this.inlineBlocks.some(({ component }) => component.handleInput?.(data));
  }

  dispose(): void {
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
  blockRenderers: ReadonlyMap<string, ReplyBlockRenderer>,
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
  const body = new ReplyBody(markdown, buttons.feedback, renderer, blockRenderers, ui.theme, scope);
  return {
    setText(next, isStreaming = false) {
      if (disposed || (next === text && isStreaming === streaming)) return;
      text = next;
      streaming = isStreaming;
      markdown.setText(next);
      scope.requestRender();
    },
    render: (width) => (disposed ? [] : body.render(width)),
    handleInput: (data) => !disposed && body.handleBlockInput(data),
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

/** An empty UI component keeps body wrappers attached as Pi creates or rebuilds assistant replies. */
export function createReplyBodies(
  tui: TUI,
  ctx: ExtensionContext,
  renderer: ReplyMarkdown,
  blockRenderers: ReadonlyMap<string, ReplyBlockRenderer>,
) {
  let disposed = false;
  const wrappers = new WeakMap<Markdown, ReplyBody>();
  const buttons = createButtonFeedback(() => tui.requestRender(), ctx.ui);

  const attach = (parent: Pick<Container, "children">, body = false): boolean => {
    let changed = false;
    parent.children.forEach((child, index) => {
      if (child instanceof ReplyBody) return;
      if (body && child instanceof Markdown) {
        let wrapper = wrappers.get(child);
        if (!wrapper) {
          wrapper = new ReplyBody(child, buttons.feedback, renderer, blockRenderers, ctx.ui.theme);
          wrappers.set(child, wrapper);
          child.invalidate();
        }
        parent.children[index] = wrapper;
        changed = true;
      } else if (child instanceof Container) {
        changed = attach(child, body || child instanceof AssistantMessageComponent) || changed;
      }
    });
    return changed;
  };

  const detach = (parent: Pick<Container, "children">) => {
    parent.children.forEach((child, index) => {
      if (child instanceof ReplyBody) {
        parent.children[index] = child.markdown;
        child.markdown.invalidate();
      } else if (child instanceof Container) {
        detach(child);
      }
    });
  };

  return {
    render(): string[] {
      if (!disposed && attach(tui)) tui.requestRender();
      return [];
    },
    invalidate() {},
    dispose() {
      disposed = true;
      buttons.dispose();
      detach(tui);
    },
  };
}

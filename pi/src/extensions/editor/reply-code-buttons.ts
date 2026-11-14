import { AssistantMessageComponent, copyToClipboard, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import {
  Container,
  Markdown,
  sliceByColumn,
  truncateToWidth,
  visibleWidth,
  type TUI,
  type TuiMouseEvent,
} from "@earendil-works/pi-tui";
import { CODE_COPY_MARKER } from "./reply-markdown.js";

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

/** Wrap only body Markdown; Pi retains ownership of the assistant and its streaming updates. */
class CodeButtons extends Container {
  private sourceLines: string[] | undefined;
  private displayLines: string[] = [];
  private buttons: CopyButton[] = [];

  constructor(
    markdown: Markdown,
    private readonly feedback: ButtonFeedback,
  ) {
    super();
    this.addChild(markdown);
  }

  override render(width: number): string[] {
    const lines = this.children[0]!.render(width);
    if (lines !== this.sourceLines) {
      this.buttons = [];
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
      this.sourceLines = lines;
    }
    const { hovered, copied } = this.feedback;
    if (hovered?.owner !== this && copied?.owner !== this) return this.displayLines;
    const display = [...this.displayLines];
    this.buttons.forEach((button, index) => {
      const isCopied = copied?.owner === this && copied.index === index;
      const isHovered = hovered?.owner === this && hovered.index === index;
      if (!isCopied && !isHovered) return;
      const line = display[button.row]!;
      const label = truncateToWidth(isCopied ? "-[✓ ok]-" : "-[copy]-", button.width, "");
      display[button.row] =
        sliceByColumn(line, 0, button.column) +
        this.feedback.style(label, isCopied) +
        sliceByColumn(line, button.column + button.width, visibleWidth(line) - button.column - button.width);
    });
    return display;
  }

  override handleMouse(event: TuiMouseEvent): ReturnType<Container["handleMouse"]> {
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
    return super.handleMouse(event);
  }

  override invalidate(): void {
    this.sourceLines = undefined;
    super.invalidate();
  }
}

/** An empty UI component keeps button wrappers attached as Pi creates or rebuilds reply bodies. */
export function createReplyCodeButtons(tui: TUI, ctx: ExtensionContext) {
  let disposed = false;
  let resetCopied: ReturnType<typeof setTimeout> | undefined;
  const feedback: ButtonFeedback = {
    hovered: undefined,
    copied: undefined,
    hover(button) {
      if (feedback.hovered?.owner === button?.owner && feedback.hovered?.index === button?.index) return;
      feedback.hovered = button;
      tui.requestRender();
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
          tui.requestRender();
        }, 1500);
        tui.requestRender();
      } catch {
        ctx.ui.notify("Could not write to the clipboard. Check clipboard access and try again.", "error");
      }
    },
    style: (label, copied) =>
      ctx.ui.theme.style(label, { fg: copied ? "success" : "accent", bold: true, underline: !copied }),
  };

  const attach = (parent: Pick<Container, "children">, body = false): boolean => {
    let changed = false;
    parent.children.forEach((child, index) => {
      if (child instanceof CodeButtons) return;
      if (body && child instanceof Markdown) {
        parent.children[index] = new CodeButtons(child, feedback);
        changed = true;
      } else if (child instanceof Container) {
        changed = attach(child, body || child instanceof AssistantMessageComponent) || changed;
      }
    });
    return changed;
  };

  const detach = (parent: Pick<Container, "children">) => {
    parent.children.forEach((child, index) => {
      if (child instanceof CodeButtons) {
        parent.children[index] = child.children[0]!;
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
      if (resetCopied) clearTimeout(resetCopied);
      detach(tui);
    },
  };
}

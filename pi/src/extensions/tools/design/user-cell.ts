import { UserMessageComponent, type ExtensionAPI, type ExtensionUIContext } from "@earendil-works/pi-coding-agent";
import {
  Container,
  Markdown,
  Spacer,
  truncateToWidth,
  visibleWidth,
  type Component,
  type TUI,
} from "@earendil-works/pi-tui";
import { designContextSummary, displayDesignContext } from "./context.js";

const WIDGET = "design-user-cells";

// Pi keeps the source text and padding on the cell so it can rebuild the Markdown.
interface CellSource {
  readonly text: string;
  readonly outputPad: number;
}

class WithDesignSummary implements Component {
  private cached?: { source: readonly string[]; lines: string[]; width: number };

  constructor(
    readonly body: Markdown,
    private readonly summary: string,
    private readonly padding: number,
    private readonly ui: ExtensionUIContext,
  ) {}

  render(width: number): readonly string[] {
    const source = this.body.render(width);
    if (this.cached?.source === source && this.cached.width === width) return this.cached.lines;

    const theme = this.ui.theme;
    const pad = Math.min(this.padding, Math.max(0, Math.floor((width - 1) / 2)));
    const details = this.summary ? theme.fg("dim", ` · ${this.summary}`) : "";
    const label = truncateToWidth(theme.fg("accent", "◈ Design") + details, width - pad * 2);
    const row = theme.bg("userMessageBg", " ".repeat(pad) + label + " ".repeat(width - pad - visibleWidth(label)));
    // Markdown supplies the cell's top and bottom padding. An attachment can also be its sole text.
    const blank = theme.bg("userMessageBg", " ".repeat(width));
    const lines = source.length ? [...source.slice(0, -1), row, source[source.length - 1]!] : [blank, row, blank];
    this.cached = { source, lines, width };
    return lines;
  }

  invalidate(): void {
    this.cached = undefined;
    this.body.invalidate();
  }
}

function mountDesignSummaries(tui: TUI, ui: ExtensionUIContext, outputPad: () => number) {
  // The transcript is the final container in Pi's document, in both TUI modes.
  const document = tui.children[0];
  const chat = document instanceof Container ? document.children.at(-1) : undefined;
  if (!(document instanceof Container) || !(chat instanceof Container)) return;

  const summaries = new WeakMap<UserMessageComponent, string | undefined>();
  let userStart = chat.children.length;
  const decorate = (cell: UserMessageComponent) => {
    const source = cell as unknown as CellSource;
    if (!summaries.has(cell)) summaries.set(cell, designContextSummary(source.text));
    const summary = summaries.get(cell);
    const body = cell.children[0];
    if (summary !== undefined && body instanceof Markdown) {
      cell.children[0] = new WithDesignSummary(body, summary, source.outputPad, ui);
    }
  };
  const reconcile: Component = {
    render() {
      for (const cell of chat.children) if (cell instanceof UserMessageComponent) decorate(cell);
      return [];
    },
    invalidate() {},
  };
  document.children.unshift(reconcile);

  return {
    messageStarted() {
      userStart = chat.children.length;
    },
    attach(text: string) {
      const cell = chat.children.slice(userStart).find((child) => child instanceof UserMessageComponent);
      if (cell) {
        // Pi creates the live cell before message_end adds the attachment.
        summaries.set(cell, designContextSummary(text));
        decorate(cell);
      } else {
        if (chat.children.length) chat.addChild(new Spacer(1));
        chat.addChild(new UserMessageComponent(text, undefined, outputPad(), [displayDesignContext]));
      }
      tui.requestRender();
    },
    dispose() {
      document.removeChild(reconcile);
      for (const cell of chat.children) {
        if (cell instanceof UserMessageComponent && cell.children[0] instanceof WithDesignSummary) {
          cell.children[0] = cell.children[0].body;
        }
      }
    },
  };
}

export function registerDesignUserCells(pi: ExtensionAPI): (text: string) => void {
  let transcript: ReturnType<typeof mountDesignSummaries>;
  pi.registerMarkdownTransformer(displayDesignContext, { messageTypes: ["user"] });
  pi.on("session_start", (_event, ctx) => {
    if (ctx.mode !== "tui") return;
    ctx.ui.setWidget(
      WIDGET,
      (tui) => {
        const mounted = mountDesignSummaries(tui, ctx.ui, () => pi.getSettings().outputPad ?? 1);
        transcript = mounted;
        return {
          render: () => [],
          invalidate() {},
          dispose() {
            mounted?.dispose();
            if (transcript === mounted) transcript = undefined;
          },
        };
      },
      { placement: "belowEditor" },
    );
  });
  pi.on("message_start", ({ message }) => {
    if (message.role === "user") transcript?.messageStarted();
  });
  return (text) => transcript?.attach(text);
}

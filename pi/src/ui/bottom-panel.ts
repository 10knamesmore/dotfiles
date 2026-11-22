import type { ExtensionUIContext, KeybindingsManager, Theme } from "@earendil-works/pi-coding-agent";
import { CURSOR_MARKER, type Component, type Container, type TUI } from "@earendil-works/pi-tui";

export interface BottomPanel extends Omit<Component, "render"> {
  focused?: boolean;
  render(width: number, height: number): readonly string[];
  dispose?(): void;
}
type PanelFactory = (tui: TUI, theme: Theme, keys: KeybindingsManager, done: () => void) => BottomPanel;

export function fillPanelRows(lines: readonly string[], height: number): string[] {
  return Array.from({ length: height }, (_, row) => lines[row] ?? "");
}

// Pi composites overlays after rendering the base containers. Read their measured
// heights from that frame so opening a panel never re-renders the transcript.
function renderedRows(component: Component): number {
  return (component as Container)["childLines"].lines.length;
}

export function openBottomPanel(
  ui: Pick<ExtensionUIContext, "custom">,
  create: PanelFactory,
  { heightPercent = 90 }: { heightPercent?: number } = {},
): Promise<void> {
  return ui.custom<void>(
    (tui, theme, keys, done) => {
      const panel = create(tui, theme, keys, done);
      let contentTop = 0;
      let contentHeight = 0;
      return {
        get focused() {
          return panel.focused ?? false;
        },
        set focused(value: boolean) {
          if (panel.focused !== undefined) panel.focused = value;
        },
        render(width) {
          const maxRows = Math.max(1, Math.min(tui.terminal.rows, Math.floor(tui.terminal.rows * heightPercent / 100)));
          const lines = panel.render(width, maxRows);
          contentHeight = lines.length;
          const cursorRow = lines.length > maxRows ? lines.findIndex((line) => line.includes(CURSOR_MARKER)) : -1;
          const firstRow = Math.max(0, cursorRow - maxRows + 1);
          const content = lines.length > maxRows ? lines.slice(firstRow, firstRow + maxRows) : lines;
          // InteractiveMode mounts editor, below-editor widgets and footer as the last three roots.
          const dockRows = tui.children.slice(-3).reduce((rows, part) => rows + renderedRows(part), 0);
          // Regular mode lays short conversations out from the top of the terminal.
          const trailingRows = tui.mode === "regular" ? Math.max(0, tui.terminal.rows - renderedRows(tui)) : 0;
          const minHeight = Math.max(maxRows, Math.min(tui.terminal.rows - 1, dockRows + trailingRows));
          const padding = Math.max(0, minHeight - content.length);
          contentTop = padding - firstRow;
          return padding ? [...Array<string>(padding).fill(""), ...content] : content;
        },
        handleInput: (data) => panel.handleInput?.(data),
        handleMouse: (event) =>
          event.y < contentTop
            ? undefined
            : panel.handleMouse?.({
                ...event,
                y: event.y - contentTop,
                height: contentHeight,
              }),
        invalidate: () => panel.invalidate(),
        dispose: () => panel.dispose?.(),
      };
    },
    { overlay: true, overlayOptions: { anchor: "bottom-center", width: "100%" } },
  );
}

/**
 * "View script" overlay: a read-only, scrollable, syntax-neutral view of a
 * workflow script shown from the launch-approval dialog.
 *
 * The overlay renders a fixed-height window (header + numbered body + footer) so
 * it never depends on knowing the terminal height inside render(); the body slice
 * scrolls with the arrow / page keys. The pure line builder is unit tested with
 * the PLAIN theme; the Component wrapper is a thin key-to-scroll shell.
 */

import type { Theme } from "@earendil-works/pi-coding-agent";
import { Key, matchesKey, truncateToWidth, type Component, type TUI } from "@earendil-works/pi-tui";
import type { ThemeLike } from "./format.js";
import { sanitizeTerminalText } from "./sanitize.js";
import { suppressInlineImages } from "./suppress-inline-images.js";

/** Total overlay rows (header + body window + footer). Small enough for short terminals. */
export const OVERLAY_HEIGHT = 22;
const HEADER_ROWS = 1;
const FOOTER_ROWS = 1;

export function bodyHeight(height: number): number {
  return Math.max(1, height - HEADER_ROWS - FOOTER_ROWS);
}

export function clampScroll(scrollTop: number, totalLines: number, viewHeight: number): number {
  const max = Math.max(0, totalLines - viewHeight);
  return Math.max(0, Math.min(scrollTop, max));
}

/** Build the overlay lines: exactly `height` rows so the overlay never clips the footer. */
export function scriptOverlayLines(script: string, scrollTop: number, height: number, width: number, theme: ThemeLike): string[] {
  const lines = script.replace(/\n+$/, "").split("\n");
  const cap = Math.max(20, width);
  const view = bodyHeight(height);
  const top = clampScroll(scrollTop, lines.length, view);
  const numWidth = String(lines.length).length;

  const header = truncateToWidth(theme.fg("dim", `workflow script · ${lines.length} lines · ↑↓ PgUp/PgDn scroll · Esc close`), cap);
  const body: string[] = [];
  for (let row = 0; row < view; row += 1) {
    const index = top + row;
    if (index >= lines.length) {
      body.push("");
      continue;
    }
    const gutter = theme.fg("dim", String(index + 1).padStart(numWidth));
    body.push(truncateToWidth(`${gutter}  ${sanitizeTerminalText(lines[index] ?? "")}`, cap));
  }
  const shown = Math.min(view, Math.max(0, lines.length - top));
  const footer = truncateToWidth(theme.fg("dim", `lines ${top + 1}-${top + shown} of ${lines.length}`), cap);
  return [header, ...body, footer];
}

/** Factory for ctx.ui.custom: a focusable overlay that scrolls the script and closes on Esc/q. */
export function scriptOverlayFactory(script: string): (tui: TUI, theme: Theme, keybindings: unknown, done: (value: undefined) => void) => Component & { focused: boolean; dispose(): void } {
  const total = script.replace(/\n+$/, "").split("\n").length;
  const view = bodyHeight(OVERLAY_HEIGHT);
  return (tui, theme, _keybindings, done) => {
    let scrollTop = 0;
    // Inline images paint over overlays (see suppress-inline-images.ts).
    const restoreImages = suppressInlineImages(tui);
    const close = (): void => {
      restoreImages();
      done(undefined);
    };
    const move = (delta: number): void => {
      scrollTop = clampScroll(scrollTop + delta, total, view);
      tui.requestRender();
    };
    return {
      focused: true,
      render: (width: number) => scriptOverlayLines(script, scrollTop, OVERLAY_HEIGHT, width, theme),
      handleInput: (data: string) => {
        if (matchesKey(data, Key.escape) || data === "q") return close();
        if (matchesKey(data, Key.up) || data === "k") return move(-1);
        if (matchesKey(data, Key.down) || data === "j") return move(1);
        if (matchesKey(data, Key.pageUp)) return move(-view);
        if (matchesKey(data, Key.pageDown) || data === " ") return move(view);
        if (matchesKey(data, Key.home)) return move(-total);
        if (matchesKey(data, Key.end)) return move(total);
      },
      invalidate: () => {},
      // Host teardown paths close overlays without resolving done(); the
      // disposer is idempotent, so covering both costs nothing.
      dispose: () => restoreImages(),
    };
  };
}

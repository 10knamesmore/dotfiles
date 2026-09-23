/**
 * Blank inline-image lines in the visible viewport while an overlay is mounted.
 *
 * pi-tui composites overlays over text lines but deliberately skips image
 * lines (compositeLineAt returns the base line untouched when it carries an
 * image escape), and kitty graphics placements default to z=0, which draws
 * above text - so an inline image under the /agents panel always paints over
 * it, and pi-tui re-transmits the placement on every panel repaint. Upstream
 * declined to blank covered image blocks in compositeOverlays (pi issue
 * #6995), so while an overlay of ours is up this installs a render wrapper on
 * the live renderer that blanks viewport image lines: pi-tui's own
 * differential pass then deletes the kitty placement, the overlay composites
 * normally, and unmounting re-transmits the image the same way.
 *
 * The `tui` handed to ctx.ui.custom factories is not necessarily the renderer
 * itself. Since pi 0.84 it is a stable Proxy (createInteractiveTuiReference)
 * whose property reads return fresh forwarding closures that re-resolve the
 * CURRENT method at call time - so capturing `tui.render` and calling it from
 * a wrapper assigned back onto `tui` recurses into the wrapper itself, and
 * the proxy has no deleteProperty/getOwnPropertyDescriptor traps, so `delete`
 * and Object.hasOwn silently miss the real renderer. Both are avoided by
 * construction here:
 * - The original is taken from the PROTOTYPE CHAIN (the proxy forwards
 *   getPrototypeOf), which yields the pristine class method - never a
 *   forwarding closure, never an installed wrapper.
 * - The wrapper is a plain method using `this`, so it acts on whichever
 *   renderer pi invokes it on.
 * - Restore assigns the prototype method back (assignment forwards through
 *   the proxy; an own property equal to the prototype method is inert), so
 *   repeated open/close cycles never stack wrappers.
 *
 * Three deliberate scope limits:
 * - Viewport only: blanking an image line above the viewport moves
 *   firstChanged above prevViewportTop, which sends pi-tui down its
 *   fullRender(true) path - a screen clear that erases the terminal's
 *   scrollback. A multi-row image straddling the viewport top therefore keeps
 *   its placement; the panel's one-row margin keeps that off the panel itself.
 * - ALL viewport images are blanked, not only rows the panel covers:
 *   extensions cannot see overlay geometry, and the panel covers ~90% of the
 *   screen anyway.
 * - Not in fullscreen (TuiAltScreen, pi 0.84 tui-mode setting): its render
 *   loop draws through a layout root rather than this.render, so a wrapper
 *   here would never be invoked. The image-over-panel bug does exist there;
 *   fixing it would mean patching a different seam with its own hazards, for
 *   a non-default mode. Suppression no-ops instead of pretending.
 *
 * Delete this file once pi-tui blanks covered image blocks itself.
 */

import type { TUI } from "@earendil-works/pi-tui";

// pi-tui does not export its isImageLine; these are the two escape prefixes
// it recognizes (kitty APC and iTerm2 OSC 1337 inline files). The ESC byte is
// spelled out so the prefixes cannot match ordinary text.
const ESC = String.fromCharCode(0x1b);
const KITTY = `${ESC}_G`;
const ITERM2 = `${ESC}]1337;File=`;

type Render = (width: number) => string[];

/** The class render method, found by walking the (proxy-forwarded) prototype chain. */
function prototypeRender(tui: TUI): Render | undefined {
  for (let proto = Object.getPrototypeOf(tui); proto; proto = Object.getPrototypeOf(proto)) {
    const descriptor = Object.getOwnPropertyDescriptor(proto, "render");
    if (typeof descriptor?.value === "function") return descriptor.value as Render;
  }
  return undefined;
}

interface PatchEntry {
  depth: number;
  original: Render;
}

const patched = new WeakMap<TUI, PatchEntry>();

/**
 * Suppress viewport image lines until the returned disposer runs. Refcounted,
 * so nested or overlapping overlays share one patch.
 */
export function suppressInlineImages(tui: TUI): () => void {
  // Fullscreen never dispatches through this.render (see scope limits above);
  // installing a wrapper there would be inert, so do not.
  if ((tui as { mode?: unknown }).mode === "fullscreen") return () => {};
  let entry = patched.get(tui);
  if (!entry) {
    const original = prototypeRender(tui);
    // Best-effort mitigation of a cosmetic bug: a host whose render is not an
    // ordinary prototype method (test fakes, future pi-tui shapes) just keeps
    // its images.
    if (!original) return () => {};
    entry = { depth: 0, original };
    patched.set(tui, entry);
    (tui as { render: Render }).render = function (this: TUI, width: number): string[] {
      const lines = original.call(this, width);
      try {
        // Self-heal: pi's own teardown paths (e.g. /reload) hide overlays
        // without running component dispose, which would leave this wrapper
        // active forever. With no overlay mounted there is nothing to
        // protect, so pass through untouched.
        if (typeof this.hasOverlay === "function" && !this.hasOverlay()) return lines;
        const start = Math.max(0, lines.length - (this.terminal?.rows ?? 24));
        for (let index = start; index < lines.length; index += 1) {
          const line = lines[index]!;
          if (line.includes(KITTY) || line.includes(ITERM2)) lines[index] = "";
        }
      } catch {
        // This runs inside pi's render loop, where a throw kills the process.
        // No cosmetic touch-up is worth that; fall through with whatever the
        // real render produced.
      }
      return lines;
    };
  }
  entry.depth += 1;
  let disposed = false;
  return () => {
    if (disposed) return;
    disposed = true;
    const current = patched.get(tui);
    if (!current) return;
    current.depth -= 1;
    if (current.depth > 0) return;
    // Assignment, not delete: pi's TUI reference proxy forwards set but has
    // no deleteProperty trap. An own property holding the prototype method is
    // behaviorally identical to no own property at all.
    (tui as { render: Render }).render = current.original;
    patched.delete(tui);
    // The restored render changes the blanked lines back; request the pass
    // that re-transmits the image.
    tui.requestRender();
  };
}

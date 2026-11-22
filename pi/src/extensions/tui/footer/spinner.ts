/**
 * Shared animation clock for the footer badge and the terminal title.
 *
 * The footer samples this clock for every published activity frame and the
 * titlebar renders those frames instead of running its own timer, so both
 * displays always show the same glyph in the same phase.
 */
export const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as const;

/** 6 frames per second, the cadence the terminal title used before the two were merged. */
export const SPINNER_INTERVAL_MS = Math.round(1000 / 6);

/** Frame for one wall-clock instant, so repeated samples share a phase. */
export function spinnerFrameAt(timeMs: number): string {
  return SPINNER_FRAMES[Math.floor(timeMs / SPINNER_INTERVAL_MS) % SPINNER_FRAMES.length] ?? SPINNER_FRAMES[0];
}

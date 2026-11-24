const SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as const;
const SPINNER_INTERVAL_MS = Math.round(1000 / 6);

/** Sample the terminal title's spinner at its own cadence. */
export function spinnerFrameAt(timeMs: number): string {
  return SPINNER_FRAMES[Math.floor(timeMs / SPINNER_INTERVAL_MS) % SPINNER_FRAMES.length] ?? SPINNER_FRAMES[0];
}

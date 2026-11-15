import type { ThemeStyle } from "@earendil-works/pi-coding-agent";

export const COPY_LABEL = "-[copy]-";
export const COPIED_LABEL = "-[✓ ok]-";
export const FEEDBACK_DURATION_MS = 1500;

export const ACTION_STYLES = {
  idle: { fg: "mdLink" },
  active: { fg: "accent", bold: true, underline: true },
  success: { fg: "success", bold: true },
  pending: { fg: "muted" },
  error: { fg: "error", bold: true },
} satisfies Record<string, ThemeStyle>;

export type ActionAppearance = keyof typeof ACTION_STYLES;

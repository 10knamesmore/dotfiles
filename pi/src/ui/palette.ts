import { foregroundAnsi, rgbColor, type RgbColorValue } from "@earendil-works/pi-tui";

/** Catppuccin Mocha colors shared by the Pi footer and prompt editor. */
export const paletteColors = {
  red: rgbColor(243, 139, 168),
  peach: rgbColor(250, 179, 135),
  yellow: rgbColor(249, 226, 175),
  green: rgbColor(166, 227, 161),
  lavender: rgbColor(180, 190, 254),
  sky: rgbColor(137, 220, 235),
  overlay2: rgbColor(147, 153, 178),
  mauve: rgbColor(203, 166, 247),
} as const;

function foreground(color: RgbColorValue, text: string): string {
  return `${foregroundAnsi(color, "truecolor")}${text}\x1b[0m`;
}

export const palette = {
  red: (text: string): string => foreground(paletteColors.red, text),
  peach: (text: string): string => foreground(paletteColors.peach, text),
  yellow: (text: string): string => foreground(paletteColors.yellow, text),
  green: (text: string): string => foreground(paletteColors.green, text),
  lavender: (text: string): string => foreground(paletteColors.lavender, text),
  sky: (text: string): string => foreground(paletteColors.sky, text),
  overlay2: (text: string): string => foreground(paletteColors.overlay2, text),
  mauve: (text: string): string => foreground(paletteColors.mauve, text),
};

export const separator = ` ${palette.overlay2("|")} `;

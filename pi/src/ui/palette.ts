/** Catppuccin Mocha colors shared by the Pi footer and prompt editor. */
const RGB = {
  red: "243;139;168",
  peach: "250;179;135",
  yellow: "249;226;175",
  green: "166;227;161",
  lavender: "180;190;254",
  sky: "137;220;235",
  overlay2: "147;153;178",
  mauve: "203;166;247",
} as const;

function foreground(rgb: string, text: string): string {
  return `\x1b[38;2;${rgb}m${text}\x1b[0m`;
}

function statusBadge(background: string, text: string): string {
  return `\x1b[48;2;${background};38;2;17;17;27m${text}\x1b[0m`;
}

/** Apply shared Catppuccin colors to footer rows and editor status badges. */
export const palette = {
  red: (text: string): string => foreground(RGB.red, text),
  peach: (text: string): string => foreground(RGB.peach, text),
  yellow: (text: string): string => foreground(RGB.yellow, text),
  green: (text: string): string => foreground(RGB.green, text),
  lavender: (text: string): string => foreground(RGB.lavender, text),
  sky: (text: string): string => foreground(RGB.sky, text),
  overlay2: (text: string): string => foreground(RGB.overlay2, text),
  mauve: (text: string): string => foreground(RGB.mauve, text),
  modelBadge: (text: string): string => statusBadge(RGB.sky, text),
  toolBadge: (text: string): string => statusBadge(RGB.peach, text),
  readyBadge: (text: string): string => statusBadge(RGB.overlay2, text),
};

export const separator = ` ${palette.overlay2("|")} `;

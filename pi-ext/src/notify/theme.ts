/**
 * 卡片自带的调色板与 ANSI 生成。
 *
 * 不走宿主的 theme API：一来它只给「token + 文本 → 着色后的串」，拿不到 RGB，
 * 而边框倒计时的渐变带需要在两色之间逐格插值；二来它对未知 token 直接抛异常，
 * 而这里要按角色名严格校验后再落色。代价是不跟随宿主主题切换。
 */

export type Rgb = readonly [number, number, number];

/** Catppuccin Mocha。与终端 / bat / fzf 同一套色板。 */
export const PALETTE = {
  base: [0x1e, 0x1e, 0x2e],
  /** 骨架色：边框倒计时蔓延的暗端。 */
  surface1: [0x45, 0x47, 0x5a],
  overlay: [0x6c, 0x70, 0x86],
  subtext: [0xa6, 0xad, 0xc8],
  text: [0xcd, 0xd6, 0xf4],
  accent: [0xcb, 0xa6, 0xf7],
  red: [0xf3, 0x8b, 0xa8],
  yellow: [0xf9, 0xe2, 0xaf],
  green: [0xa6, 0xe3, 0xa1],
  peach: [0xfa, 0xb3, 0x87],
} as const satisfies Record<string, Rgb>;

/** 可作为 span `fg` 写的角色名。 */
export type RoleName =
  | "text"
  | "subtext"
  | "overlay"
  | "accent"
  | "red"
  | "yellow"
  | "green"
  | "peach";

const ROLES: Record<RoleName, Rgb> = {
  text: PALETTE.text,
  subtext: PALETTE.subtext,
  overlay: PALETTE.overlay,
  accent: PALETTE.accent,
  red: PALETTE.red,
  yellow: PALETTE.yellow,
  green: PALETTE.green,
  peach: PALETTE.peach,
};

/**
 * 解析 fg：角色名或 `#rrggbb`。
 *
 * 只认这 8 个角色名和恰好 6 位的十六进制，其余一律报错而非静默降级——写错颜色
 * 名要当场知道，不是渲染成默认色蒙混过去。
 */
export function parseFg(name: string): Rgb {
  const role = ROLES[name as RoleName];
  if (role) return role;

  const hex = name.startsWith("#") ? name.slice(1) : undefined;
  if (hex?.length === 6) {
    const r = Number.parseInt(hex.slice(0, 2), 16);
    const g = Number.parseInt(hex.slice(2, 4), 16);
    const b = Number.parseInt(hex.slice(4, 6), 16);
    if (!Number.isNaN(r) && !Number.isNaN(g) && !Number.isNaN(b))
      return [r, g, b];
  }
  throw new Error(
    `unknown fg ${JSON.stringify(name)}, expected a theme role ` +
      `(${Object.keys(ROLES).join("/")}) or "#rrggbb"`,
  );
}

/** `(a*(d-n) + b*n) / d`，逐通道整数插值。 */
function lerpByte(a: number, b: number, num: number, denom: number): number {
  const d = Math.max(1, denom);
  const n = Math.min(num, d);
  return Math.round((a * (d - n) + b * n) / d);
}

export function lerpColor(from: Rgb, to: Rgb, num: number, denom: number): Rgb {
  return [
    lerpByte(from[0], to[0], num, denom),
    lerpByte(from[1], to[1], num, denom),
    lerpByte(from[2], to[2], num, denom),
  ];
}

export interface Modifiers {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  dim?: boolean;
}

/**
 * 给文本套上前景色与修饰位。
 *
 * 只重置颜色与修饰（39 / 22 / 23 / 24），不发整串 SGR reset——宿主 TUI 会在每行
 * 末尾补 reset，行内用全量 reset 会把外层可能存在的背景一并清掉。
 */
export function paint(text: string, fg: Rgb, mods: Modifiers = {}): string {
  if (text === "") return "";
  const open: string[] = [`38;2;${fg[0]};${fg[1]};${fg[2]}`];
  const close: string[] = ["39"];
  if (mods.bold) {
    open.push("1");
    close.push("22");
  }
  if (mods.dim) {
    open.push("2");
    close.push("22");
  }
  if (mods.italic) {
    open.push("3");
    close.push("23");
  }
  if (mods.underline) {
    open.push("4");
    close.push("24");
  }
  return `\x1b[${open.join(";")}m${text}\x1b[${close.join(";")}m`;
}

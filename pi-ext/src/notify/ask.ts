/**
 * 交互审批卡片：与只读通知同一套画框，但走 overlay，能收键盘并 await 出决策。
 *
 * 用于权限审批这类「必须由人拍板」的场合：调用方 await 拿到用户选择，超时按
 * `timeoutValue` 落定（审批场景通常是拒绝——超时不应视为放行）。
 */

import { Key, matchesKey } from "@earendil-works/pi-tui";

import { type CardKind, type FrameOptions, accentNameOf, renderFrame } from "./frame.ts";
import type { Line, Span } from "./span.ts";

export interface Choice<T> {
  value: T;
  label: string;
  /** 右侧灰字提示，如后果说明。 */
  hint?: string;
  /** 危险项标红，避免手滑选中。 */
  danger?: boolean;
}

export interface AskOptions<T> extends FrameOptions {
  choices: Choice<T>[];
  /** 初始选中项下标；缺省 0。 */
  defaultIndex?: number;
  /** 倒计时秒数；缺省不限时。 */
  ttlSecs?: number;
  /** 超时落定的值；缺省 null。审批场景应显式给「拒绝」。 */
  timeoutValue?: T;
}

/** 只依赖用得到的那部分宿主 ctx。 */
export interface AskHost {
  ui: {
    custom<T>(
      factory: (
        tui: { requestRender(): void },
        theme: unknown,
        keybindings: unknown,
        done: (value: T) => void,
      ) => { render(width: number): string[]; invalidate(): void; handleInput?(data: string): void },
      opts?: { overlay?: boolean; overlayOptions?: Record<string, unknown> },
    ): Promise<T>;
  };
}

/** 倒计时重绘间隔，与只读卡片一致。 */
const TICK_MS = 100;

/** 选项行：选中项带 ❯ 前缀并染级别色，危险项标红。 */
function choiceLine<T>(choice: Choice<T>, selected: boolean, kind: CardKind | undefined): Line {
  const accent = accentNameOf(kind);
  const spans: Span[] = [
    { text: selected ? "❯ " : "  ", fg: selected ? accent : "overlay" },
    {
      text: choice.label,
      fg: selected ? accent : choice.danger ? "red" : "text",
      bold: selected,
    },
  ];
  if (choice.hint) spans.push({ text: choice.hint, fg: "overlay", align: "right" });
  return spans;
}

/**
 * 弹出审批卡片，await 得到用户决策。
 *
 * 键位：↑↓ / kj 移动，Enter 确认，Esc 取消（返回 null），1-9 直选。
 */
export function askCard<T>(ctx: AskHost, opts: AskOptions<T>): Promise<T | null> {
  return ctx.ui.custom<T | null>(
    (tui, _theme, _kb, done) => {
      let index = Math.min(Math.max(0, opts.defaultIndex ?? 0), opts.choices.length - 1);
      const startedAt = Date.now();
      const ttlMs = opts.ttlSecs ? opts.ttlSecs * 1000 : undefined;

      let dismiss: ReturnType<typeof setTimeout> | undefined;
      let tick: ReturnType<typeof setInterval> | undefined;
      /** 定时器必须在 done 前收掉：overlay 关闭后组件即销毁，回调再触发就没人接了。 */
      const finish = (value: T | null) => {
        if (dismiss) clearTimeout(dismiss);
        if (tick) clearInterval(tick);
        done(value);
      };

      if (ttlMs) {
        dismiss = setTimeout(() => finish(opts.timeoutValue ?? null), ttlMs);
        tick = setInterval(() => tui.requestRender(), TICK_MS);
      }

      return {
        invalidate() {},

        render(avail: number): string[] {
          const progress = ttlMs ? Math.floor(((Date.now() - startedAt) * 1000) / ttlMs) : 0;
          const body: Line[] = [...opts.body];
          if (body.length > 0) body.push("");
          for (const [i, choice] of opts.choices.entries()) {
            body.push(choiceLine(choice, i === index, opts.kind));
          }
          return renderFrame({ ...opts, body }, avail, progress);
        },

        handleInput(data: string) {
          const last = opts.choices.length - 1;
          if (matchesKey(data, Key.up) || data === "k") {
            index = index === 0 ? last : index - 1;
          } else if (matchesKey(data, Key.down) || data === "j") {
            index = index === last ? 0 : index + 1;
          } else if (matchesKey(data, Key.enter)) {
            finish(opts.choices[index]?.value ?? null);
            return;
          } else if (matchesKey(data, Key.escape)) {
            finish(null);
            return;
          } else if (/^[1-9]$/.test(data)) {
            const pick = Number(data) - 1;
            if (pick <= last) {
              finish(opts.choices[pick]?.value ?? null);
              return;
            }
          }
          tui.requestRender();
        },
      };
    },
    { overlay: true },
  );
}

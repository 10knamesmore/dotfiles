/**
 * 只读通知卡片：推到宿主的 widget 区常驻展示，不接收键盘。
 *
 * 同 id 再推一次即替换、不堆叠——直接复用 setWidget 的 id 语义，无需自己维护栈。
 * 带 ttl 的到时自动退场，否则驻留到显式 dismissCard。
 *
 * 需要用户做选择时用 ask.ts 的 askCard（overlay，能收键盘）。
 */

import { type FrameOptions, renderFrame } from "./frame.ts";

export interface CardOptions extends FrameOptions {
  /** 同 id 替换不堆叠；缺省 "default"。 */
  id?: string;
  /** 到时自动退场；缺省驻留，须显式 dismissCard。 */
  ttlSecs?: number;
}

/** 只依赖用得到的那部分宿主 ctx，便于测试时塞假对象。 */
export interface CardHost {
  ui: {
    setWidget(
      id: string,
      widget?: (tui: { requestRender(): void }) => {
        render(width: number): string[];
        invalidate(): void;
      },
      opts?: { placement?: "aboveEditor" | "belowEditor" },
    ): void;
  };
}

/** 倒计时重绘间隔。边框至多百余格，再快也看不出差别，白烧 CPU。 */
const TICK_MS = 100;

interface Live {
  dismiss: ReturnType<typeof setTimeout>;
  tick?: ReturnType<typeof setInterval>;
}
const live = new Map<string, Live>();

function clearTimers(id: string): void {
  const entry = live.get(id);
  if (!entry) return;
  clearTimeout(entry.dismiss);
  if (entry.tick) clearInterval(entry.tick);
  live.delete(id);
}

export function showCard(ctx: CardHost, opts: CardOptions): void {
  const id = opts.id ?? "default";
  clearTimers(id);

  const widgetId = `notify-card:${id}`;
  const startedAt = Date.now();
  const ttlMs = opts.ttlSecs ? opts.ttlSecs * 1000 : undefined;

  if (ttlMs) {
    live.set(id, {
      dismiss: setTimeout(() => {
        ctx.ui.setWidget(widgetId, undefined);
        clearTimers(id);
      }, ttlMs),
    });
  }

  ctx.ui.setWidget(widgetId, (tui) => {
    if (ttlMs) {
      const tick = setInterval(() => {
        if (Date.now() - startedAt >= ttlMs) clearInterval(tick);
        tui.requestRender();
      }, TICK_MS);
      const entry = live.get(id);
      if (entry) entry.tick = tick;
    }
    return {
      invalidate() {},
      render(avail: number): string[] {
        const progress = ttlMs ? Math.floor(((Date.now() - startedAt) * 1000) / ttlMs) : 0;
        return renderFrame(opts, avail, progress);
      },
    };
  });
}

export function dismissCard(ctx: CardHost, id = "default"): void {
  clearTimers(id);
  ctx.ui.setWidget(`notify-card:${id}`, undefined);
}

/** extension shutdown 时收干净所有定时器，别让进程挂着不退。 */
export function disposeAllCards(ctx: CardHost): void {
  for (const id of [...live.keys()]) dismissCard(ctx, id);
}

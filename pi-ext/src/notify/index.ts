/**
 * 通知与审批卡片 extension
 *
 * 分三层：frame.ts 画框（纯渲染）、card.ts 只读通知（widget 常驻、不收键盘）、
 * ask.ts 交互审批（overlay 收键盘、await 出决策）。这里只做入口——注册演示命令、
 * 退出时收定时器。其他 extension 直接 `import { askCard } from "../notify/ask.ts"` 复用。
 */

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { askCard } from "./ask.ts";
import { disposeAllCards, showCard } from "./card.ts";

export default function (pi: ExtensionAPI) {
  // pi.registerCommand("card", {
  //   description: "演示通知卡片（可选参数：warn / error / ttl）",
  //   handler: async (args, ctx) => {
  //     const arg = (args ?? "").trim();
  //     const kind = arg === "warn" || arg === "error" ? arg : "info";
  //
  //     showCard(ctx, {
  //       id: "demo",
  //       kind,
  //       ttlSecs: 5,
  //       title: [
  //         { text: "权限审批", bold: true },
  //         { text: "  ·  ", fg: "overlay" },
  //         { text: "bash", fg: "subtext", italic: true },
  //       ],
  //       body: [
  //         [
  //           { text: "命令  " },
  //           { text: "rm -rf ./build", fg: "red", bold: true },
  //         ],
  //         [{ text: "规则  " }, { text: "rm-recursive-force", fg: "yellow" }],
  //         "",
  //         [{ text: "该操作会递归强制删除，无法撤销。", fg: "subtext" }],
  //         "",
  //         [
  //           { text: "↑↓ 选择", fg: "overlay" },
  //           {
  //             text: arg === "ttl" ? "10s 后自动取消" : "常驻",
  //             fg: "overlay",
  //             align: "center",
  //           },
  //           { text: "Enter 确认", fg: "overlay", align: "right" },
  //         ],
  //       ],
  //     });
  //   },
  // });
  // pi.registerCommand("approve", {
  //   description: "演示交互审批卡片（↑↓ 选择 · Enter 确认 · Esc 拒绝）",
  //   handler: async (_args, ctx) => {
  //     const decision = await askCard<"once" | "session" | "always" | "deny">(ctx, {
  //       kind: "warn",
  //       title: "权限审批",
  //       closeHint: "↑↓ 选择 · Enter 确认 · Esc 拒绝",
  //       // 超时按拒绝落定——审批场景里「没人应答」绝不能等于放行
  //       ttlSecs: 30,
  //       timeoutValue: "deny",
  //       body: [
  //         [{ text: "命令  " }, { text: "rm -rf ./build", fg: "red", bold: true }],
  //         [{ text: "规则  " }, { text: "rm-recursive-force", fg: "yellow" }],
  //         [{ text: "该操作会递归强制删除，无法撤销。", fg: "subtext" }],
  //       ],
  //       choices: [
  //         { value: "once", label: "允许一次", hint: "1" },
  //         { value: "session", label: "本会话都允许", hint: "2" },
  //         { value: "always", label: "永久允许", hint: "3" },
  //         { value: "deny", label: "拒绝", hint: "4", danger: true },
  //       ],
  //     });
  //     ctx.ui.notify(`决策：${decision ?? "取消"}`, decision === "deny" ? "warning" : "info");
  //   },
  // });
  // pi.on("session_shutdown", async (_event, ctx) => {
  //   disposeAllCards(ctx);
  // });
}

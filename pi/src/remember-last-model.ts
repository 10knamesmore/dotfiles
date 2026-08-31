/**
 * 记住上次使用的模型和 thinking level（effort）。
 *
 *   $XDG_STATE_HOME/pi/last-model.json，默认 ~/.local/state/pi/last-model.json
 */

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

type ThinkingLevel =
  "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max";

interface LastModelState {
  provider?: string;
  modelId?: string;
  thinkingLevel?: ThinkingLevel;
}

const STATE_DIR = join(
  process.env.XDG_STATE_HOME ?? join(homedir(), ".local", "state"),
  "pi",
);
const STATE_PATH = join(STATE_DIR, "last-model.json");

function readState(): LastModelState {
  try {
    const parsed = JSON.parse(readFileSync(STATE_PATH, "utf8"));
    return typeof parsed === "object" && parsed !== null ? parsed : {};
  } catch {
    return {};
  }
}

function writeState(state: LastModelState): void {
  try {
    mkdirSync(STATE_DIR, { recursive: true });
    writeFileSync(STATE_PATH, JSON.stringify(state, null, 2) + "\n");
  } catch (error) {
    console.error(
      `[remember-last-model] failed to write ${STATE_PATH}:`,
      error,
    );
  }
}

export default function (pi: ExtensionAPI) {
  // 用户切换模型时记录（/model、Ctrl+P、session 恢复都会触发）
  pi.on("model_select", async (event) => {
    const prev = readState();
    writeState({
      ...prev,
      provider: event.model.provider,
      modelId: event.model.id,
    });
  });

  // thinking level 变化时记录（模型切换引发的更新也会随之覆盖）
  pi.on("thinking_level_select", async (event) => {
    const prev = readState();
    writeState({ ...prev, thinkingLevel: event.level });
  });

  // 新会话启动时套用上次的模型与 effort
  pi.on("session_start", async (_event, ctx) => {
    const last = readState();
    if (!last.provider || !last.modelId) return;

    const model = ctx.modelRegistry
      .getAvailable()
      .find((m) => m.provider === last.provider && m.id === last.modelId);
    if (!model || !ctx.modelRegistry.hasConfiguredAuth(model)) return;

    await pi.setModel(model);
    if (last.thinkingLevel) {
      pi.setThinkingLevel(last.thinkingLevel);
    }
  });
}

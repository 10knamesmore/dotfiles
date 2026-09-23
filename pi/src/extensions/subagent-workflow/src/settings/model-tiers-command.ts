import type { ExtensionAPI, ExtensionCommandContext } from "@earendil-works/pi-coding-agent";
import { reportDiagnostic } from "../diagnostics.js";
import { sanitizeTerminalText } from "../ui/sanitize.js";
import { errorMessage } from "../util.js";
import {
  MODEL_TIERS,
  getPersonalConfigPath,
  readPersonalConfig,
  updatePersonalConfig,
  type ModelTiers,
} from "../../../../config/index.js";

export function registerModelTiersCommand(pi: ExtensionAPI): void {
  pi.registerCommand("model-tiers", {
    description: "查看或设置 max、high、mid 的全局模型绑定",
    handler: async (_args, ctx) => {
      if (!ctx.hasUI) return;
      const path = getPersonalConfigPath();
      for (;;) {
        let tiers: ModelTiers;
        try {
          tiers = readPersonalConfig()["model-tier"];
        } catch (error) {
          reportFailure(ctx, "read", path, error);
          return;
        }

        const tierOptions = MODEL_TIERS.map((tier) => ({
          tier,
          label: `${tier}：${sanitizeTerminalText(tiers[tier] ?? "未配置")}`,
        }));
        const tierChoice = await ctx.ui.select(
          `模型档位（选择后立即保存，取消退出）\n配置文件：${sanitizeTerminalText(path)} → model-tier`,
          tierOptions.map((option) => option.label),
        );
        const selectedTier = tierOptions.find((option) => option.label === tierChoice)?.tier;
        if (!selectedTier) return;

        let models: ReturnType<typeof ctx.modelRegistry.getAvailable>;
        try {
          models = ctx.modelRegistry.getAvailable();
        } catch (error) {
          reportFailure(ctx, "models", path, error);
          return;
        }
        if (models.length === 0) {
          ctx.ui.notify(
            "Pi 当前没有可用模型。请先配置供应商或登录，再运行 /model-tiers；已有绑定仍可清除。",
            "warning",
          );
        }
        const providers = [...new Set(models.map((model) => model.provider))].sort();
        const providerOptions = providers.map((provider) => ({
          provider,
          label: `供应商：${sanitizeTerminalText(provider)}`,
        }));
        const clearBinding = "清除绑定";
        const providerChoice = await ctx.ui.select(`${selectedTier}：选择供应商或清除绑定`, [
          ...providerOptions.map((option) => option.label),
          clearBinding,
        ]);
        if (providerChoice === undefined) return;

        let reference: string | undefined;
        if (providerChoice !== clearBinding) {
          const provider = providerOptions.find((option) => option.label === providerChoice)?.provider;
          if (provider === undefined) return;
          const modelOptions = models
            .filter((model) => model.provider === provider)
            .sort((left, right) => left.id.localeCompare(right.id))
            .map((model) => ({ model, label: sanitizeTerminalText(model.id) }));
          const modelChoice = await ctx.ui.select(
            `${selectedTier}：选择 ${sanitizeTerminalText(provider)} 的模型`,
            modelOptions.map((option) => option.label),
          );
          const selectedModel = modelOptions.find((option) => option.label === modelChoice)?.model;
          if (!selectedModel) return;
          reference = `${selectedModel.provider}/${selectedModel.id}`;
        }

        try {
          updatePersonalConfig((config) => {
            if (reference === undefined) delete config["model-tier"][selectedTier];
            else config["model-tier"][selectedTier] = reference;
          });
        } catch (error) {
          reportFailure(ctx, "write", path, error);
          return;
        }
        reportDiagnostic(
          `[subagent-workflow] model tier saved: path=${path} tier=${selectedTier} model=${reference ?? "<unconfigured>"}`,
        );
        ctx.ui.notify(
          reference === undefined
            ? `已清除 ${selectedTier} 的模型绑定。`
            : `已保存 ${selectedTier}：${sanitizeTerminalText(reference)}`,
          "info",
        );
      }
    },
  });
}

function reportFailure(
  ctx: ExtensionCommandContext,
  operation: "read" | "write" | "models",
  path: string,
  error: unknown,
): void {
  reportDiagnostic(`[subagent-workflow] model tiers ${operation} failed: path=${path} error=${errorMessage(error)}`);
  const messages = {
    read: "无法读取个人配置。请检查文件权限和 JSON 格式；language 须为非空字符串，model-tier 字段只允许 max、high、mid，值须为 provider/model-id。请手动修复后重试。",
    write: "无法保存模型档位配置。请检查配置格式和文件权限，或稍后重试。",
    models: "无法获取 Pi 的可用模型。请检查供应商配置后重试。",
  };
  ctx.ui.notify(`${messages[operation]}\n配置文件：${sanitizeTerminalText(path)}`, "error");
}

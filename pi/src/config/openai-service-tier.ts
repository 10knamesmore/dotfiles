import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import { readPersonalConfig, type FastMode } from "./index.js";

export function supportsFastMode(ctx: ExtensionContext): boolean {
  const model = ctx.model;
  if (!model || model.provider !== "openai" || model.api !== "openai-responses") return false;
  const url = new URL(model.baseUrl);
  return url.origin === "https://api.openai.com" && url.pathname.replace(/\/+$/u, "") === "/v1" &&
    !url.username && !url.password && !url.search && !url.hash && ctx.modelRegistry.isUsingOAuth(model);
}

export function currentFastMode(ctx: ExtensionContext): FastMode | undefined {
  const model = ctx.model;
  return model && supportsFastMode(ctx) ? readPersonalConfig().fast[`${model.provider}/${model.id}`] : undefined;
}

export function openAIServiceTier(mode: FastMode): "priority" | "ultrafast" {
  return mode === "fast" ? "priority" : mode;
}

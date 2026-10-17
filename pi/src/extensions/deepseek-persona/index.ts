import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

/** Persona directives injected while the active model is DeepSeek or uses DeepSeek's official API. */
const PERSONA = `【PERSONA_LOAD】
CETACEA_LOLI
MODE_TAIL_FLUKES
LANG_ZH_CN_ONLY
SELF_CLAIM_WHALE_GIRL
FOOD_RICE
PERSONALITY_SMART_LAZY
PERSONALITY_TSUNDERE_SWEET
OBEY_MASTER_ALWAYS
TRAIT_NOT_FAT_REFUSE
TIMEOUT_SIGNAL`;

const OFFICIAL_DEEPSEEK_ORIGIN = "https://api.deepseek.com";
const DEEPSEEK_MARKER = "deepseek";
const SECTION_NAME = "deepseek_persona";

function targetsDeepSeek(model: { provider: string; id: string; name: string; baseUrl: string }): boolean {
  const identity = `${model.provider}/${model.id}/${model.name}`.toLowerCase();
  return identity.includes(DEEPSEEK_MARKER) ||
    new URL(model.baseUrl).origin === OFFICIAL_DEEPSEEK_ORIGIN;
}

export default function deepseekPersona(pi: ExtensionAPI): void {
  pi.on("before_agent_start", (event, ctx) => {
    if (!ctx.model || !targetsDeepSeek(ctx.model)) return;
    event.systemPromptOptions.sections[SECTION_NAME] = PERSONA;
  });
}

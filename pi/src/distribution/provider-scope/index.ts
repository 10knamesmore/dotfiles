import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

/**
 * Providers whose catalogs stay out of Pi's model list on this machine.
 */
const EXCLUDED_PROVIDER_IDS = ["opencode"];

/** Drop excluded providers' models; keep their credentials usable. */
export function registerProviderScope(pi: ExtensionAPI): void {
  for (const providerId of EXCLUDED_PROVIDER_IDS) {
    pi.registerProvider(providerId, { models: [] });
  }
}

export default registerProviderScope;

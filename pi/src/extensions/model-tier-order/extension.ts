import type { Api, Model } from "@earendil-works/pi-ai";
import { ModelSelectorComponent, type ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { MODEL_TIERS, readPersonalConfig } from "@dotfiles/pi-config";

interface ModelItem {
  provider: string;
  id: string;
  model: Model<Api>;
}

// Pi exposes the selector but keeps sorting and row rendering private, with no customization hooks.
interface ModelSelectorInternals {
  filteredModels: ModelItem[];
  sortModels(models: ModelItem[]): ModelItem[];
  updateList(): void;
}

export default function modelTierOrder(pi: ExtensionAPI): void {
  let restoreSelector: (() => void) | undefined;

  pi.on("session_start", (_event, ctx) => {
    if (ctx.mode !== "tui") return;
    restoreSelector?.();

    const prototype = ModelSelectorComponent.prototype as unknown as ModelSelectorInternals;
    const originalSort = prototype.sortModels;
    const originalUpdateList = prototype.updateList;

    prototype.sortModels = function (models) {
      const tiers = readPersonalConfig()["model-tier"];
      const preferred = MODEL_TIERS.map((tier) => tiers[tier]);
      const rank = (item: ModelItem): number => {
        const index = preferred.indexOf(`${item.provider}/${item.id}`);
        return index === -1 ? MODEL_TIERS.length : index;
      };

      // Stable sorting retains Pi's current/default/provider order outside the tier models.
      return originalSort.call(this, models).sort((left, right) => rank(left) - rank(right));
    };

    prototype.updateList = function () {
      const tiers = readPersonalConfig()["model-tier"];
      const models = this.filteredModels;
      // Only the synchronous row renderer sees labels; search and selection retain the real IDs.
      this.filteredModels = models.map((item) => {
        const labels = MODEL_TIERS.filter((tier) => tiers[tier] === `${item.provider}/${item.id}`);
        return labels.length > 0 ? { ...item, id: `[${labels.join("/")}] ${item.id}` } : item;
      });
      try {
        originalUpdateList.call(this);
      } finally {
        this.filteredModels = models;
      }
    };

    restoreSelector = () => {
      prototype.sortModels = originalSort;
      prototype.updateList = originalUpdateList;
      restoreSelector = undefined;
    };
  });

  pi.on("session_shutdown", () => {
    restoreSelector?.();
  });
}

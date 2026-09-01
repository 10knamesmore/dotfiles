import type {
  ExtensionAPI,
  ExtensionCommandContext,
} from "@earendil-works/pi-coding-agent";
import { ThinkingSelectorComponent } from "@earendil-works/pi-coding-agent";
import type { Model, ModelThinkingLevel } from "@earendil-works/pi-ai";
import { getSupportedThinkingLevels } from "@earendil-works/pi-ai";

/** Every pi thinking level in picker order, offered until a model is loaded. */
const ALL_LEVELS: ModelThinkingLevel[] = [
  "off",
  "minimal",
  "low",
  "medium",
  "high",
  "xhigh",
  "max",
];

/** Register `/effort` as an alias for the built-in `/thinking` command. */
export function registerEffortAlias(pi: ExtensionAPI): void {
  pi.registerCommand("effort", {
    description: "Alias for /thinking: switch thinking level",
    getArgumentCompletions: (prefix) => {
      const normalized = prefix.trim().toLowerCase();
      const matches = ALL_LEVELS.filter((level) =>
        level.startsWith(normalized),
      );
      return matches.length > 0
        ? matches.map((level) => ({ value: level, label: level }))
        : null;
    },
    handler: async (rawArgs, ctx) => {
      const levels = supportedLevels(ctx.model);
      const requested = rawArgs.trim().toLowerCase();
      if (requested === "") {
        await pickLevel(pi, ctx, levels);
        return;
      }
      const level = levels.find(
        (candidate) => candidate.toLowerCase() === requested,
      );
      if (!level) {
        ctx.ui.notify(
          `Unknown thinking level "${rawArgs.trim()}". Available levels: ${levels.join(", ")}.`,
          "error",
        );
        return;
      }
      applyLevel(pi, ctx, level);
    },
  });
}

export default registerEffortAlias;

/** Levels the active model supports, mirroring the built-in `/thinking` picker. */
function supportedLevels(
  model: Model<any> | undefined,
): ModelThinkingLevel[] {
  return model ? getSupportedThinkingLevels(model) : ALL_LEVELS;
}

/** Switch the session thinking level and report it like the built-in command. */
function applyLevel(
  pi: ExtensionAPI,
  ctx: ExtensionCommandContext,
  level: ModelThinkingLevel,
): void {
  pi.setThinkingLevel(level);
  ctx.ui.notify(`Thinking level: ${level}`, "info");
}

/**
 * Open the built-in `/thinking` selector. Ctrl+S behaves like Enter because
 * the extension API cannot persist a startup default.
 */
function pickLevel(
  pi: ExtensionAPI,
  ctx: ExtensionCommandContext,
  levels: ModelThinkingLevel[],
): Promise<void> {
  return ctx.ui.custom<void>((_tui, _theme, _keybindings, done) => {
    const select = (level: ModelThinkingLevel) => {
      applyLevel(pi, ctx, level);
      done();
    };
    return new ThinkingSelectorComponent(
      pi.getThinkingLevel(),
      levels,
      select,
      () => done(),
      select,
    );
  });
}

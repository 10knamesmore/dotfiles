import type {
  CompactionResult,
  ExtensionAPI,
  ExtensionCommandContext,
  ExtensionContext,
} from "@earendil-works/pi-coding-agent";

type CompactMode = {
  label: string;
  target: "branch" | "new";
  withPrompt: boolean;
};

const COMPACT_MODES: CompactMode[] = [
  { label: "current branch：compact", target: "branch", withPrompt: false },
  { label: "current branch：compact with prompt", target: "branch", withPrompt: true },
  { label: "new session：compact", target: "new", withPrompt: false },
  { label: "new session：compact with prompt", target: "new", withPrompt: true },
];

/** Choose where the compact result lives and whether to provide compact instructions. */
export default function registerCompactNew(pi: ExtensionAPI): void {
  pi.registerCommand("compact-new", {
    description: "Choose branch/new session and compact/compact with prompt",
    handler: async (_args, ctx) => {
      if (!ctx.hasUI) return;

      const selectedLabel = await ctx.ui.select(
        "compact-new：选择 compact 方式",
        COMPACT_MODES.map((mode) => mode.label),
      );
      const mode = COMPACT_MODES.find((candidate) => candidate.label === selectedLabel);
      if (!mode) return;

      const instructions = mode.withPrompt ? await requestInstructions(ctx) : undefined;
      if (mode.withPrompt && instructions === undefined) return;

      await ctx.waitForIdle();
      if (!ctx.sessionManager.getLeafId()) {
        ctx.ui.notify("There is no conversation to compact.", "warning");
        return;
      }

      const result = await compact(ctx, instructions);
      if (!result || mode.target === "branch") return;

      await openSummarySession(ctx, result);
    },
  });
}

async function requestInstructions(ctx: ExtensionCommandContext): Promise<string | undefined> {
  const instructions = await ctx.ui.editor("compact prompt", "");
  const trimmed = instructions?.trim();
  if (!trimmed) {
    ctx.ui.notify("compact prompt cannot be empty.", "warning");
    return undefined;
  }
  return trimmed;
}

function compact(ctx: ExtensionContext, customInstructions: string | undefined): Promise<CompactionResult | undefined> {
  return new Promise((resolve) => {
    ctx.compact({
      customInstructions,
      onComplete: (result) => {
        if (ctx.hasUI) ctx.ui.notify("Compaction completed.", "info");
        resolve(result);
      },
      onError: (error) => {
        if (ctx.hasUI) ctx.ui.notify(`Compaction failed: ${error.message}`, "error");
        resolve(undefined);
      },
    });
  });
}

async function openSummarySession(ctx: ExtensionCommandContext, result: CompactionResult): Promise<void> {
  const summary = result.summary.trim();
  if (!summary) {
    ctx.ui.notify("Compaction produced an empty summary; the new session was not created.", "error");
    return;
  }

  const sessionResult = await ctx.newSession({
    parentSession: ctx.sessionManager.getSessionFile(),
    setup: async (sessionManager) => {
      sessionManager.appendMessage({
        role: "user",
        content: [{ type: "text", text: summary }],
        timestamp: Date.now(),
      });
    },
    withSession: async (replacementCtx) => {
      if (replacementCtx.hasUI) replacementCtx.ui.notify("Opened a new session with the compact summary.", "info");
    },
  });

  if (sessionResult.cancelled) return;
}

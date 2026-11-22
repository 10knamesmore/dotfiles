import { fileURLToPath } from "node:url";
import { getAgentDir, type ExtensionAPI, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { designContext, displayDesignContext } from "./context.js";
import { selectDesign } from "./selector.js";
import { DesignService, openBrowser } from "./service.js";
import type { DesignProject, DesignState } from "./types.js";

const SKILL = fileURLToPath(new URL("./skills/design/SKILL.md", import.meta.url));
const PROJECT_ENTRY = "design-project";

export default function designExtension(pi: ExtensionAPI): void {
  let service: DesignService | undefined;
  let associated: DesignProject | undefined;
  let lastSentRevision = 0;
  let lastSentProject: string | undefined;
  let newDesignPrompt = false;

  pi.on("resources_discover", () => ({ skillPaths: [SKILL] }));
  pi.registerMarkdownTransformer((markdown, context) =>
    context.messageType === "user" ? displayDesignContext(markdown) : markdown,
  );

  function remember(state: DesignState, ctx: ExtensionContext): void {
    if (!state.project) return;
    if (associated?.id !== state.project.id) {
      associated = state.project;
      pi.appendEntry(PROJECT_ENTRY, state.project);
      lastSentRevision = 0;
      lastSentProject = undefined;
    }
    ctx.ui.setStatus("design", `Design · ${state.project.name}`);
  }

  function savedProject(ctx: ExtensionContext): DesignProject | undefined {
    for (const entry of ctx.sessionManager.getBranch().slice().reverse()) {
      if (entry.type === "custom" && entry.customType === PROJECT_ENTRY) {
        return entry.data as DesignProject | undefined;
      }
    }
    return undefined;
  }

  pi.on("session_start", async (_event, ctx) => {
    process.env.PI_DESIGN_AGENT_DIR = getAgentDir();
    service = await DesignService.start(getAgentDir(), ctx.sessionManager.getSessionId());
    const saved = savedProject(ctx);
    associated = saved;
    if (saved) remember(await service.open(saved.path), ctx);
  });

  pi.on("session_shutdown", async () => {
    await service?.close();
    service = undefined;
  });

  pi.on("session_tree", async (_event, ctx) => {
    await service?.close();
    service = await DesignService.start(getAgentDir(), ctx.sessionManager.getSessionId());
    lastSentRevision = 0;
    lastSentProject = undefined;
    const saved = savedProject(ctx);
    associated = saved;
    ctx.ui.setStatus("design", undefined);
    if (saved) remember(await service.open(saved.path), ctx);
  });

  pi.registerCommand("design", {
    description: "Choose a design project or start one from a prompt",
    handler: async (_args, ctx) => {
      if (ctx.mode !== "tui") throw new Error("/design requires the interactive project selector.");
      if (!service) throw new Error("Design service is unavailable. Run dots sync and /reload.");
      try {
        const projects = await service.projects();
        const choice = await selectDesign(ctx, projects);
        if (!choice) return;
        if (choice === "new") {
          const prompt = await ctx.ui.editor("New design");
          if (!prompt?.trim()) return;
          newDesignPrompt = true;
          pi.sendUserMessage(prompt, { deliverAs: "followUp" });
          return;
        }
        const project = projects.find((item) => item.id === choice)!;
        remember(await service.open(project.path), ctx);
        await openBrowser(service.url);
      } catch (error) {
        ctx.ui.notify(error instanceof Error ? error.message : String(error), "error");
      }
    },
  });

  pi.on("input", async (event, ctx) => {
    if (newDesignPrompt && event.source === "extension") {
      newDesignPrompt = false;
      return {
        action: "transform",
        text:
          `${event.text}\n\n<pi_design_new>\n` +
          `Create a new design from the user's free-form request. Read ${SKILL} first. ` +
          "Choose an appropriate project name and directory from the request; absent a path, use ~/Documents. " +
          "Use normal file tools to create the React/Vite project and install dependencies. " +
          "Use pi_design.open(path, name=...) to register it globally and connect this session. " +
          "This is a new project, not a request to modify the previously selected design.\n</pi_design_new>",
        images: event.images,
      };
    }
    if (!service) return { action: "continue" };
    const state = await service.state();
    if (!state.project) return { action: "continue" };
    remember(state, ctx);
    const first = lastSentProject !== state.project.id;
    const changes = await service.changes(first ? 0 : lastSentRevision);
    lastSentRevision = state.revision;
    lastSentProject = state.project.id;
    return {
      action: "transform",
      text: `${event.text}\n\n${designContext(state, changes, first)}`,
      images: event.images,
    };
  });

  // SDK registration happens inside the Python worker; observe it without creating another tool.
  pi.on("tool_result", async (event, ctx) => {
    if (event.toolName !== "python_repl" || !service) return;
    const state = await service.state();
    if (!state.project) return;
    const changed = associated?.id !== state.project.id;
    remember(state, ctx);
    if (changed) {
      try {
        await openBrowser(service.url);
      } catch (error) {
        ctx.ui.notify(error instanceof Error ? error.message : String(error), "error");
      }
    }
  });
}

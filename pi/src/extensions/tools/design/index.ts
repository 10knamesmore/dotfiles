import { fileURLToPath } from "node:url";
import { getAgentDir, type ExtensionAPI, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { designContext, isNewDesignRequest, newDesignRequest } from "./context.js";
import { registerDesignStatus, selectDesign } from "./selector.js";
import { DesignService, openBrowser } from "./service.js";
import type { DesignProject, DesignState } from "./types.js";
import { registerDesignUserCells } from "./user-cell.js";

const SKILL = fileURLToPath(new URL("./skills/design/SKILL.md", import.meta.url));
const PROJECT_ENTRY = "design-project";

interface AttachedState {
  revision: number;
  opened_revision: number | null;
  selection: DesignState["selection"];
  name: string;
}

export default function designExtension(pi: ExtensionAPI): void {
  let service: DesignService | undefined;
  let associated: DesignProject | null = null;
  let observedRevision = -1;
  let lastAttached: AttachedState | undefined;
  let panelOpen = false;
  const publishStatus = registerDesignStatus(pi, manageDesign);
  const showAttachment = registerDesignUserCells(pi);

  pi.on("resources_discover", () => ({ skillPaths: [SKILL] }));

  function remember(state: DesignState, ctx: ExtensionContext): boolean {
    if (state.revision < observedRevision) return false;
    const changed = associated?.id !== state.project?.id;
    if (changed || associated?.name !== state.project?.name) {
      const saved = savedProject(ctx);
      if (saved?.id !== state.project?.id || saved?.name !== state.project?.name) {
        pi.appendEntry(PROJECT_ENTRY, state.project);
      }
    }
    observedRevision = state.revision;
    associated = state.project;
    publishStatus(ctx, associated);
    return changed;
  }

  function savedProject(ctx: ExtensionContext): DesignProject | null {
    for (const entry of ctx.sessionManager.getBranch().slice().reverse()) {
      if (entry.type === "custom" && entry.customType === PROJECT_ENTRY) {
        return (entry.data as DesignProject | null) ?? null;
      }
    }
    return null;
  }

  async function restoreConnection(ctx: ExtensionContext): Promise<void> {
    service = await DesignService.start(getAgentDir(), ctx.sessionManager.getSessionId());
    associated = null;
    observedRevision = -1;
    lastAttached = undefined;
    publishStatus(ctx, null);
    const saved = savedProject(ctx);
    if (saved) await connectProject(service, saved, ctx);
  }

  async function connectProject(service: DesignService, project: DesignProject, ctx: ExtensionContext): Promise<void> {
    try {
      remember(await service.open(project.path), ctx);
    } catch (error) {
      // The project can be selected even when Vite fails to start.
      const state = await service.state().catch(() => undefined);
      if (state) remember(state, ctx);
      throw error;
    }
  }

  pi.on("session_start", async (_event, ctx) => {
    process.env.PI_DESIGN_AGENT_DIR = getAgentDir();
    await restoreConnection(ctx);
  });

  pi.on("session_shutdown", async (_event, ctx) => {
    publishStatus(ctx, null);
    await service?.close();
    service = undefined;
  });

  pi.on("session_tree", async (_event, ctx) => {
    await service?.close();
    service = undefined;
    await restoreConnection(ctx);
  });

  async function manageDesign(ctx: ExtensionContext): Promise<void> {
    if (ctx.mode !== "tui" || panelOpen) return;
    if (!service) {
      ctx.ui.notify("Design service is unavailable. Run dots sync and /reload.", "error");
      return;
    }
    panelOpen = true;
    try {
      const projects = await service.projects();
      const connected = associated;
      const choice = await selectDesign(ctx, projects, connected);
      if (!choice) return;
      if (choice === "new") {
        const prompt = await ctx.ui.editor("New design");
        if (prompt?.trim()) pi.sendUserMessage(newDesignRequest(prompt, SKILL), { deliverAs: "followUp" });
        return;
      }
      if (choice === "disconnect" && connected) {
        remember(await service.disconnect(connected.id), ctx);
        return;
      }
      const project = choice === "workspace" ? connected : projects.find((item) => item.id === choice);
      if (!project) return;
      await connectProject(service, project, ctx);
      await openBrowser(service.url);
    } catch (error) {
      ctx.ui.notify(error instanceof Error ? error.message : String(error), "error");
    } finally {
      panelOpen = false;
    }
  }

  pi.registerCommand("design", {
    description: "Manage the connected design project",
    handler: (_args, ctx) => manageDesign(ctx),
  });

  pi.on("message_end", async ({ message }, ctx) => {
    if (!service || message.role !== "user") return;
    const content =
      typeof message.content === "string" ? [{ type: "text" as const, text: message.content }] : message.content;
    if (content.some((part) => part.type === "text" && isNewDesignRequest(part.text))) return;

    const state = await service.snapshot(lastAttached?.revision ?? 0);
    if (state.revision < observedRevision) return;
    remember(state, ctx);
    if (!state.project) return;
    const first = !lastAttached || lastAttached.opened_revision !== state.opened_revision;
    if (
      !first &&
      lastAttached?.revision === state.revision &&
      lastAttached.name === state.project.name &&
      lastAttached.selection.artboard === state.selection.artboard &&
      lastAttached.selection.element === state.selection.element
    ) {
      return;
    }

    const text = `\n\n${designContext(state, first)}`;
    lastAttached = {
      revision: state.revision,
      opened_revision: state.opened_revision,
      selection: state.selection,
      name: state.project.name,
    };
    showAttachment(text);
    return { message: { ...message, content: [...content, { type: "text", text }] } };
  });

  // SDK registration happens inside the Python worker; observe it without creating another tool.
  pi.on("tool_result", async (event, ctx) => {
    if (event.toolName !== "python_repl" || !service) return;
    const state = await service.state();
    if (remember(state, ctx) && state.project) {
      try {
        await openBrowser(service.url);
      } catch (error) {
        ctx.ui.notify(error instanceof Error ? error.message : String(error), "error");
      }
    }
  });
}

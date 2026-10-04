import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import type { ExtensionAPI, ExtensionContext, ToolDefinition } from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { PythonSession, type PythonToolDetails } from "./session.js";
import { executionText, renderPythonCall, renderPythonResult } from "./render.js";
import { describePythonEnvironment, inspectPythonEnvironment, type PythonEnvironment } from "./environment.js";
import { logPythonEvent } from "./diagnostics.js";
import { PythonObserverServer } from "./observer/server.js";
import type { ObserverStatus } from "./observer/types.js";

const Parameters = Type.Object(
  {
    code: Type.String({
      minLength: 1,
      description: "Complete Python code to execute. Use print() to display values.",
    }),
    cwd: Type.Optional(
      Type.String({
        minLength: 1,
        description:
          "Working directory for this call only. Omit to use the session directory; relative paths resolve from it.",
      }),
    ),
    timeout: Type.Optional(
      Type.Number({
        exclusiveMinimum: 0,
        maximum: 2_147_483_647 / 1000,
        description: "Execution timeout in seconds (default: 60)",
      }),
    ),
  },
  { additionalProperties: false },
);

const PYTHON_TOOL_NAME = "python_repl";

const EMPTY_ENVIRONMENT_NOTICE =
  "The previous Python environment was cleared; its variables, functions, and imports were lost. Python calls executed after that reset use a new environment. Earlier tool results are history only and do not restore lost state; reinitialize any data you still need from before the reset.";

const TERMINAL_USE_SKILL = fileURLToPath(
  new URL("./native/terminal-use/skills/terminal-use/SKILL.md", import.meta.url),
);

const COMPUTER_USE_SKILL = fileURLToPath(
  new URL("./native/computer-use/skills/computer-use/SKILL.md", import.meta.url),
);

const BROWSER_USE_SKILL = fileURLToPath(
  new URL("./native/browser-use/skills/browser-use/SKILL.md", import.meta.url),
);

const TOOL_DESCRIPTION =
  "Execute complete Python code in a persistent environment shared by this live Pi session. Variables, functions, and imports survive calls, normal exceptions, model changes, and compaction. Runs in cwd for this call, defaulting to the session directory; relative file paths and new local imports resolve there. Working directory changes are restored after each call. Host environment variables are inherited unchanged; stdin is unavailable. Text output is limited, larger output is saved to a file. display_image(image) sends an image to the model; accepts a file path, encoded image bytes, or an image object. No import needed.";

/** Register one persistent Python environment per live Pi session, including independent subagent sessions. */
export function registerPython(pi: ExtensionAPI): void {
  pi.on("resources_discover", () => ({
    skillPaths: [TERMINAL_USE_SKILL, BROWSER_USE_SKILL, ...(process.platform === "linux" ? [COMPUTER_USE_SKILL] : [])],
  }));

  let session: PythonSession | undefined;
  let currentContext: ExtensionContext | undefined;
  const observer = new PythonObserverServer(() => session);
  const inspectionController = new AbortController();

  const createSession = (ctx: ExtensionContext): PythonSession => {
    currentContext = ctx;
    return new PythonSession(
      ctx.sessionManager.getSessionId(), ctx.cwd, notifyEnvironmentCleared, updateEnvironment, updateStatus,
    );
  };

  pi.registerCommand("python", {
    description: "打开 Python / Terminal 只读 Web 观测台",
    handler: async (_args, ctx) => {
      currentContext = ctx;
      session ??= createSession(ctx);
      let url: string;
      try {
        url = await observer.open();
      } catch (error) {
        logPythonEvent({
          sessionId: ctx.sessionManager.getSessionId(), phase: "observer_open_failed",
          reason: error instanceof Error ? error.message : String(error),
        });
        ctx.ui.notify("无法启动运行观测，请查看 Python 扩展日志。", "error");
        return;
      }
      updateStatus(session.status());
      let opened = false;
      try {
        const result = await pi.exec(process.platform === "darwin" ? "open" : "xdg-open", [url], { timeout: 10_000 });
        opened = result.code === 0;
      } catch {
        logPythonEvent({ sessionId: ctx.sessionManager.getSessionId(), phase: "observer_browser_open_failed" });
      }
      ctx.ui.notify(`${opened ? "运行观测已打开" : "请在浏览器打开运行观测"}：${url}`, "info");
    },
  });

  pi.on("session_start", async (_event, ctx) => {
    session = createSession(ctx);
    if (hasPythonHistory(ctx)) notifyEnvironmentCleared();
    const sessionId = ctx.sessionManager.getSessionId();
    const started = Date.now();
    logPythonEvent({ sessionId, phase: "environment_inspection_start" });
    try {
      const environment = await inspectPythonEnvironment(pi.exec, ctx.cwd, inspectionController.signal);
      if (inspectionController.signal.aborted) return;
      updateEnvironment(environment);
      logPythonEvent({
        sessionId,
        phase: "environment_inspection_end",
        durationMs: Date.now() - started,
        interpreter: {
          executable: environment.executable,
          version: environment.version,
        },
        packageCount: environment.packages.length,
      });
    } catch (error) {
      logPythonEvent({
        sessionId,
        phase: "environment_inspection_failed",
        durationMs: Date.now() - started,
        reason: error instanceof Error ? error.message : String(error),
      });
    }
  });
  pi.on("session_shutdown", async () => {
    inspectionController.abort();
    await observer.close();
    await session?.close();
    session = undefined;
    currentContext?.ui.setStatus("python", undefined);
    currentContext = undefined;
  });
  pi.on("session_tree", async (event, ctx) => {
    if (event.newLeafId === event.oldLeafId) return;
    currentContext = ctx;
    const hadEnvironment = session?.available;
    await session?.close();
    session = createSession(ctx);
    updateStatus(session.status());
    if (hadEnvironment || hasPythonHistory(ctx)) notifyEnvironmentCleared();
  });
  pi.on("tool_result", (event) => {
    if (event.toolName !== PYTHON_TOOL_NAME) return;
    const details = event.details as PythonToolDetails | undefined;
    if (details) return { isError: details.outcome !== "completed" };
    return undefined;
  });

  const tool: ToolDefinition<typeof Parameters, PythonToolDetails | undefined> = {
    name: PYTHON_TOOL_NAME,
    label: "Python",
    description: `${TOOL_DESCRIPTION}\n\n${describePythonEnvironment()}`,
    promptSnippet: "Run Python calculations and data analysis with variables preserved across calls",
    promptGuidelines: [
      "Use python_repl for Python calculations and structured data analysis. Reuse variables from earlier python_repl calls",
      "Prefer python_repl over running Python through bash, including heredoc scripts and python -c. ",
    ],
    parameters: Parameters,
    executionMode: "sequential",
    async execute(callId, params, signal, onUpdate, ctx) {
      if (!params.code.trim()) throw new Error("code must not be blank.");
      currentContext = ctx;
      const runtime = (session ??= createSession(ctx));
      onUpdate?.({
        content: [
          {
            type: "text",
            text: runtime.available ? "Preparing Python execution…" : "Starting Python with uv…",
          },
        ],
        details: undefined,
      });
      const cwd = resolve(ctx.cwd, params.cwd ?? ".");
      const result = await runtime.execute(params.code, params.timeout ?? 60, callId, cwd, signal, () => {
        onUpdate?.({
          content: [{ type: "text", text: "Running Python…" }],
          details: undefined,
        });
      });
      return {
        content: [{ type: "text", text: executionText(result) }, ...result.images],
        details: result.details,
      };
    },
    renderCall: renderPythonCall,
    renderResult: renderPythonResult,
  };
  pi.registerTool(tool);

  function updateStatus(status: ObserverStatus): void {
    if (!currentContext?.hasUI) return;
    const labels: Record<ObserverStatus["state"], string> = {
      not_started: "未启动", starting: "启动中", running: "执行中", idle: "空闲", stopping: "停止中", exited: "已退出",
    };
    currentContext.ui.setStatus("python", `Python ${labels[status.state]} · 终端 ${status.terminalCount} · /python`);
  }

  function notifyEnvironmentCleared(): void {
    // Use the live message queue so the active tool loop and saved history receive the same notice.
    pi.sendMessage(
      {
        customType: "python-environment",
        content: EMPTY_ENVIRONMENT_NOTICE,
        display: false,
      },
      { deliverAs: "steer" },
    );
  }

  function updateEnvironment(environment: PythonEnvironment): void {
    const description = `${TOOL_DESCRIPTION}\n\n${describePythonEnvironment(environment)}`;
    if (description === tool.description) return;
    tool.description = description;
    pi.registerTool(tool);
  }
}

export default registerPython;

function hasPythonHistory(ctx: ExtensionContext): boolean {
  const { sessionManager } = ctx;
  // Walk backwards so a recent Python call can be found without building the full branch.
  let entry = sessionManager.getLeafEntry();
  while (entry) {
    if (entry.type === "message") {
      const message = entry.message;
      if (
        (message.role === "toolResult" && message.toolName === PYTHON_TOOL_NAME) ||
        (message.role === "assistant" &&
          message.content.some((part) => part.type === "toolCall" && part.name === PYTHON_TOOL_NAME))
      )
        return true;
    }
    entry = entry.parentId ? sessionManager.getEntry(entry.parentId) : undefined;
  }
  return false;
}

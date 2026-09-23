import type {
  ExtensionAPI,
  ExtensionContext,
  ToolDefinition,
} from "@earendil-works/pi-coding-agent";
import { Type } from "typebox";
import { PythonSession, type PythonToolDetails } from "./session.js";
import {
  executionText,
  renderPythonCall,
  renderPythonResult,
} from "./render.js";
import {
  describePythonEnvironment,
  inspectPythonEnvironment,
  type PythonEnvironment,
} from "./environment.js";
import { logPythonEvent } from "./diagnostics.js";

const Parameters = Type.Object(
  {
    code: Type.String({
      minLength: 1,
      description:
        "Complete Python code to execute. Use print() to display values.",
    }),
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

const EMPTY_ENVIRONMENT_NOTICE =
  "The previous Python environment was cleared; its variables, functions, and imports were lost. Python calls executed after that reset use a new environment. Earlier tool results are history only and do not restore lost state; reinitialize any data you still need from before the reset.";

const TOOL_DESCRIPTION =
  "Execute complete Python code in a persistent environment shared by this live Pi session. Variables, functions, and imports survive calls, normal exceptions, model changes, and compaction. stdin is unavailable. Output is limited, larger output is saved to a file.";

/** Register one persistent Python environment per live Pi session, including independent subagent sessions. */
export function registerPython(pi: ExtensionAPI): void {
  let session: PythonSession | undefined;
  const inspectionController = new AbortController();

  const createSession = (ctx: ExtensionContext): PythonSession =>
    new PythonSession(
      ctx.sessionManager.getSessionId(),
      notifyEnvironmentCleared,
      updateEnvironment,
    );

  pi.on("session_start", async (_event, ctx) => {
    session = createSession(ctx);
    if (hasPythonHistory(ctx)) notifyEnvironmentCleared();
    const sessionId = ctx.sessionManager.getSessionId();
    const started = Date.now();
    logPythonEvent({ sessionId, phase: "environment_inspection_start" });
    try {
      const environment = await inspectPythonEnvironment(
        pi.exec,
        ctx.cwd,
        inspectionController.signal,
      );
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
    await session?.close();
    session = undefined;
  });
  pi.on("session_tree", async (event, ctx) => {
    if (event.newLeafId === event.oldLeafId) return;
    const hadEnvironment = session?.available;
    await session?.close();
    session = createSession(ctx);
    if (hadEnvironment || hasPythonHistory(ctx)) notifyEnvironmentCleared();
  });
  pi.on("tool_result", (event) => {
    if (event.toolName !== "python") return;
    const details = event.details as PythonToolDetails | undefined;
    if (details) return { isError: details.outcome !== "completed" };
    return undefined;
  });

  const tool: ToolDefinition<typeof Parameters, PythonToolDetails | undefined> =
    {
      name: "python",
      label: "Python",
      description: `${TOOL_DESCRIPTION}\n\n${describePythonEnvironment()}`,
      promptSnippet:
        "Run Python calculations and data analysis with variables preserved across calls",
      promptGuidelines: [
        "Use python for Python calculations and structured data analysis instead of wrapping Python code in bash. Reuse variables from earlier python calls",
      ],
      parameters: Parameters,
      executionMode: "sequential",
      async execute(callId, params, signal, onUpdate, ctx) {
        if (!params.code.trim()) throw new Error("code must not be blank.");
        const runtime = (session ??= createSession(ctx));
        onUpdate?.({
          content: [
            {
              type: "text",
              text: runtime.available
                ? "Preparing Python execution…"
                : "Starting Python with uv…",
            },
          ],
          details: undefined,
        });
        const result = await runtime.execute(
          params.code,
          params.timeout ?? 60,
          callId,
          ctx.cwd,
          signal,
          () => {
            onUpdate?.({
              content: [{ type: "text", text: "Running Python…" }],
              details: undefined,
            });
          },
        );
        return {
          content: [{ type: "text", text: executionText(result) }],
          details: result.details,
        };
      },
      renderCall: renderPythonCall,
      renderResult: renderPythonResult,
    };
  pi.registerTool(tool);

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

function hasPythonHistory(ctx: ExtensionContext): boolean {
  const { sessionManager } = ctx;
  // Walk backwards so a recent Python call can be found without building the full branch.
  let entry = sessionManager.getLeafEntry();
  while (entry) {
    if (entry.type === "message") {
      const message = entry.message;
      if (
        (message.role === "toolResult" && message.toolName === "python") ||
        (message.role === "assistant" && message.content.some(
          (part) => part.type === "toolCall" && part.name === "python",
        ))
      ) return true;
    }
    entry = entry.parentId ? sessionManager.getEntry(entry.parentId) : undefined;
  }
  return false;
}

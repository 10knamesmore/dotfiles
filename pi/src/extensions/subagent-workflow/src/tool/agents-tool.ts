import { join } from "node:path";
import {
  truncateHead,
  type AgentToolResult,
  type AgentToolUpdateCallback,
  type ExtensionAPI,
  type ExtensionContext,
  type ToolDefinition,
} from "@earendil-works/pi-coding-agent";
import { Type, type Static } from "typebox";
import { subagentRunner } from "../runner/runner.js";
import { isLiveStatus } from "../store/run-projection.js";
import { listRunSummaries, readRunDetail, type ChildRow, type RunSummary } from "../store/run-reader.js";
import { readSessionMessages, type TranscriptMessage } from "../store/session-transcript.js";
import { isRecord } from "../util.js";
import { parseWorkflowScript } from "../workflow/parser.js";
import {
  renderAgentsCall,
  renderAgentsResult,
  type AgentControlRow,
  type AgentsToolRenderState,
  type AgentsToolSnapshot,
} from "../ui/agents-tool-render.js";

const Action = Type.Union([
  Type.Literal("list", { description: "List runs newest first." }),
  Type.Literal("peek", {
    description:
      "Read run progress and child IDs, or a child's recent transcript and result. Previews are bounded; full records are under artifacts.",
  }),
  Type.Literal("stop", { description: "Cancel the selected run or child." }),
  Type.Literal("send_message", { description: "Send instructions to a pending or running child." }),
], {
  description: "Inspect or control existing work. Completion is pushed automatically; do not poll.",
});
const Status = Type.Union([
  Type.Literal("pending"),
  Type.Literal("running"),
  Type.Literal("completed"),
  Type.Literal("failed"),
  Type.Literal("aborted"),
]);

const Parameters = Type.Object(
  {
    action: Action,
    runId: Type.Optional(Type.String({
      minLength: 1,
      description: "Required except for list. stop and send_message require the Pi process that owns this run.",
    })),
    childId: Type.Optional(
      Type.String({
        minLength: 1,
        description:
          "Select one child for peek/stop; required for send_message. Omit to target the entire run; stopping it also cancels future workflow work. " +
          "Stopping one child makes its agent() call return null; the workflow may continue. Finished children use subagent.followUp for a new turn.",
      }),
    ),
    message: Type.Optional(
      Type.String({
        minLength: 1,
        description:
          "send_message only: instruction to queue as steering, buffered during startup. Acceptance does not confirm consumption.",
      }),
    ),
    status: Type.Optional(
      Type.Union([Type.Literal("all"), Type.Literal("live"), Status], {
        description: "list only; default all. live includes pending and running.",
      }),
    ),
    offset: Type.Optional(
      Type.Integer({ minimum: 0, description: "Page offset for list or run-level peek; default 0." }),
    ),
    limit: Type.Optional(
      Type.Integer({ minimum: 1, maximum: 20, description: "Runs or children per page; default 20." }),
    ),
    messages: Type.Optional(
      Type.Integer({
        minimum: 0,
        maximum: 12,
        description:
          "Child-level peek only: number of recent persisted messages; default 6. Streaming text is shown separately.",
      }),
    ),
  },
  { additionalProperties: false },
);
type Input = Static<typeof Parameters>;

const RunOverview = Type.Object({
  runId: Type.String(),
  runDir: Type.String(),
  kind: Type.Union([Type.Literal("subagent"), Type.Literal("workflow")]),
  label: Type.String(),
  status: Status,
  done: Type.Number(),
  total: Type.Number(),
  canStop: Type.Boolean(),
});
const ChildOverview = Type.Object({
  childId: Type.String(),
  label: Type.String(),
  model: Type.String(),
  status: Status,
  phase: Type.Optional(Type.String()),
  activity: Type.Optional(Type.String()),
  error: Type.Optional(Type.String()),
  canControl: Type.Boolean(),
});
const MessagePreview = Type.Object({
  role: Type.String(),
  toolName: Type.Optional(Type.String()),
  text: Type.String(),
});
const Output = Type.Union([
  Type.Object({
    action: Type.Literal("list"),
    runs: Type.Array(RunOverview),
    total: Type.Number(),
    nextOffset: Type.Optional(Type.Number()),
  }),
  Type.Object({
    action: Type.Literal("peek"),
    run: RunOverview,
    children: Type.Array(ChildOverview),
    total: Type.Number(),
    nextOffset: Type.Optional(Type.Number()),
    phases: Type.Array(Type.String()),
    recentProgress: Type.Array(Type.String()),
    artifacts: Type.Array(Type.String()),
  }),
  Type.Object({
    action: Type.Literal("peek"),
    runId: Type.String(),
    child: ChildOverview,
    prompt: Type.String(),
    resultPreview: Type.Optional(Type.String()),
    messages: Type.Array(MessagePreview),
    streaming: Type.Optional(MessagePreview),
    artifacts: Type.Array(Type.String()),
  }),
  Type.Object({
    action: Type.Literal("stop"),
    runId: Type.String(),
    childId: Type.Optional(Type.String()),
    status: Status,
  }),
  Type.Object({
    action: Type.Literal("send_message"),
    runId: Type.String(),
    childId: Type.String(),
    accepted: Type.Boolean(),
  }),
]);
type Output = Static<typeof Output>;

const readOptions = {
  describeWorkflow: (script: string) => parseWorkflowScript(script).meta.name,
};

/** Only the current parent's orchestrated runs belong in its model context. */
function sessionRuns(ctx: ExtensionContext): RunSummary[] {
  const sessionId = ctx.sessionManager.getSessionId();
  return listRunSummaries(ctx.cwd, readOptions).filter(
    (run) => run.parentSessionId === sessionId && !run.directDelivery,
  );
}

function validateInput(input: Input): void {
  let allowed: Array<keyof Input>;
  switch (input.action) {
    case "list":
      allowed = ["action", "status", "offset", "limit"];
      break;
    case "peek":
      allowed = input.childId ? ["action", "runId", "childId", "messages"] : ["action", "runId", "offset", "limit"];
      break;
    case "stop":
      allowed = ["action", "runId", "childId"];
      break;
    case "send_message":
      allowed = ["action", "runId", "childId", "message"];
      if (!input.childId) throw new Error("send_message requires childId");
      if (!input.message?.trim()) throw new Error("send_message requires a non-empty message");
      if (input.message.trimStart().startsWith("/")) {
        throw new Error("send_message accepts instructions, not slash commands");
      }
      break;
  }
  for (const key of Object.keys(input) as Array<keyof Input>) {
    if (input[key] !== undefined && !allowed.includes(key)) {
      throw new Error(`${key} is not valid for ${input.action}${input.childId ? " with childId" : ""}`);
    }
  }
  if (input.action !== "list" && !input.runId) throw new Error(`${input.action} requires runId`);
}

function preview(text: string, maxBytes = 512): string {
  // Streaming decode keeps a partial final UTF-8 character out of the preview.
  const prefix = new TextDecoder().decode(Buffer.from(text).subarray(0, maxBytes), { stream: true });
  const clipped = truncateHead(prefix, { maxBytes, maxLines: 100 });
  return clipped.truncated || prefix.length < text.length ? `${clipped.content}\n[truncated; see artifacts]` : text;
}

function runOverview(run: RunSummary): Static<typeof RunOverview> {
  return {
    runId: run.runId,
    runDir: run.runDir,
    kind: run.kind,
    label: preview(run.label),
    status: run.status,
    done: run.done,
    total: run.total,
    canStop: subagentRunner.isRunActive(run.runId),
  };
}

function childOverview(runId: string, child: ChildRow): Static<typeof ChildOverview> {
  const handle = subagentRunner.get(child.id);
  return {
    childId: child.id,
    label: preview(child.label),
    model: preview(child.model),
    status: child.status,
    ...(child.phase ? { phase: preview(child.phase) } : {}),
    ...(child.activity ? { activity: preview(child.activity) } : {}),
    ...(child.error ? { error: preview(child.error) } : {}),
    canControl: handle?.runId === runId && isLiveStatus(handle.status),
  };
}

function messagePreview(message: TranscriptMessage): Static<typeof MessagePreview> {
  const parts =
    typeof message.content === "string"
      ? [message.content]
      : Array.isArray(message.content)
        ? message.content.flatMap((part: unknown) => {
            if (!isRecord(part)) return [];
            if (part.type === "text" && typeof part.text === "string") return [part.text];
            if (part.type === "toolCall") return [`${String(part.name)} ${JSON.stringify(part.arguments)}`];
            if (part.type === "image") return ["[image]"];
            return [];
          })
        : [];
  return {
    role: message.role,
    ...(message.toolName ? { toolName: message.toolName } : {}),
    text: preview(parts.join("\n"), 1_500),
  };
}

function page<T>(items: T[], offset = 0, limit = 20) {
  const end = offset + limit;
  return {
    items: items.slice(offset, end),
    total: items.length,
    ...(end < items.length ? { nextOffset: end } : {}),
  };
}

interface ActionResult {
  output: Output;
  snapshot: AgentsToolSnapshot;
}

function resultSnapshot(input: Input, output: Output, target?: string): AgentsToolSnapshot {
  const snapshot: AgentsToolSnapshot = { action: input.action, target, rows: [], expandedLines: [] };
  if (output.action === "list") {
    snapshot.rows = output.runs.map((run) => ({
      name: run.label,
      status: run.status,
      metadata: `${run.kind}  ${run.done}/${run.total}`,
    }));
    snapshot.expandedLines = output.runs.flatMap((run) => [run.label, run.runId, run.runDir]);
  } else if (output.action === "peek" && "run" in output) {
    snapshot.rows = output.children.map((child) => ({
      name: child.label,
      status: child.status,
      metadata: child.model,
    }));
    snapshot.expandedLines = [
      ...output.recentProgress,
      ...output.children.flatMap((child) => [child.childId, ...(child.activity ? [child.activity] : [])]),
      ...output.artifacts,
    ];
  } else if (output.action === "peek" && "child" in output) {
    snapshot.rows = [{ name: output.child.model, status: output.child.status }];
    snapshot.activity = output.child.activity;
    snapshot.quote = output.resultPreview || output.streaming?.text
      || [...output.messages].reverse().find((message) => message.role === "assistant" && message.text)?.text;
    snapshot.expandedLines = [
      output.prompt,
      ...output.messages.flatMap((message) => [message.toolName ?? message.role, message.text]),
      output.child.childId,
      ...output.artifacts,
    ];
  } else if (output.action === "send_message") {
    snapshot.quote = input.message!.trim();
    snapshot.expandedLines = [output.runId, output.childId];
  } else if (output.action === "stop") {
    snapshot.status = output.status;
    snapshot.expandedLines = [output.runId, ...(output.childId ? [output.childId] : [])];
  }
  if ("nextOffset" in output && output.nextOffset !== undefined) {
    snapshot.expandedLines.push(`offset ${output.nextOffset}`);
  }
  return snapshot;
}

async function executeAction(
  input: Input,
  ctx: ExtensionContext,
  onUpdate: AgentToolUpdateCallback<AgentsToolSnapshot | undefined> | undefined,
): Promise<ActionResult> {
  validateInput(input);
  const runs = sessionRuns(ctx);
  if (input.action === "list") {
    const filtered = runs.filter((run) =>
      !input.status || input.status === "all"
        ? true
        : input.status === "live"
          ? isLiveStatus(run.status)
          : run.status === input.status,
    );
    const { items, ...pagination } = page(filtered, input.offset, input.limit);
    const output: Output = { action: "list", runs: items.map(runOverview), ...pagination };
    return { output, snapshot: resultSnapshot(input, output) };
  }

  const run = runs.find((candidate) => candidate.runId === input.runId);
  if (!run) throw new Error(`No orchestrated run ${input.runId} belongs to this session and cwd; use agents list`);
  const detail = readRunDetail(run.runDir, run.runId, { ...readOptions, bypassCache: true });
  if (detail.corrupt) throw new Error(`Cannot read run ${run.runId}; inspect ${run.runDir}`);
  const child = input.childId ? detail.children.find((candidate) => candidate.id === input.childId) : undefined;
  if (input.childId && !child) throw new Error(`Child ${input.childId} does not belong to run ${run.runId}`);
  const artifacts = [join(run.runDir, "run.json"), join(run.runDir, "events.jsonl")];
  const target = child && run.kind === "workflow" ? `${run.label} / ${child.label}` : run.label;
  onUpdate?.({
    content: [],
    details: {
      action: input.action,
      target,
      rows: [],
      quote: input.action === "send_message" ? input.message!.trim() : undefined,
      expandedLines: [],
    },
  });
  const finish = (output: Output, rows?: AgentControlRow[]): ActionResult => {
    const snapshot = resultSnapshot(input, output, target);
    if (rows) snapshot.rows = rows;
    return { output, snapshot };
  };

  if (input.action === "peek") {
    if (child) {
      const live = subagentRunner.liveSession(child.id);
      const sessionFile = live?.sessionFile ?? child.sessionFile;
      const messages = sessionFile ? readSessionMessages(sessionFile) : [];
      const count = input.messages ?? 6;
      const current = live?.currentAssistant;
      return finish({
        action: "peek",
        runId: run.runId,
        child: childOverview(run.runId, child),
        prompt: preview(child.spec.prompt, 4_000),
        ...(child.result
          ? {
              resultPreview: preview(
                child.result.structured === undefined
                  ? child.result.text
                  : JSON.stringify(child.result.structured, null, 2),
                8_000,
              ),
            }
          : {}),
        messages: count === 0 ? [] : messages.slice(-count).map(messagePreview),
        ...(current ? { streaming: messagePreview(current) } : {}),
        artifacts: [...artifacts, ...(sessionFile ? [sessionFile] : [])],
      });
    }
    const { items, ...pagination } = page(detail.children, input.offset, input.limit);
    return finish({
      action: "peek",
      run: runOverview(run),
      children: items.map((row) => childOverview(run.runId, row)),
      ...pagination,
      phases: detail.phases.map((phase) => preview(phase.title)),
      recentProgress: detail.narrator.slice(-10).map((line) => preview(line.text)),
      artifacts,
    });
  }

  if (input.action === "stop" && !child) {
    if (subagentRunner.isRunActive(run.runId)) {
      await subagentRunner.stopRun(run.runId);
    } else if (isLiveStatus(run.status)) {
      throw new Error(`Run ${run.runId} is owned by another process; stop it from that Pi instance`);
    }
    const stopped = readRunDetail(run.runDir, run.runId, { ...readOptions, bypassCache: true });
    return finish({
      action: "stop",
      runId: run.runId,
      status: stopped.status,
    }, stopped.children.map((row) => ({ name: row.label, status: row.status })));
  }

  const handle = subagentRunner.get(child!.id);
  if (input.action === "stop" && !isLiveStatus(child!.status)) {
    return finish({ action: "stop", runId: run.runId, childId: child!.id, status: child!.status });
  }
  if (!handle || handle.runId !== run.runId) {
    throw new Error(`Child ${child!.id} is not active in this process; use subagent.followUp for a new turn`);
  }
  if (input.action === "stop") {
    await handle.abort();
    return finish({ action: "stop", runId: run.runId, childId: child!.id, status: handle.status });
  }
  await handle.steer(input.message!.trim());
  return finish({ action: "send_message", runId: run.runId, childId: child!.id, accepted: true });
}

export function registerAgentsTool(pi: ExtensionAPI): void {
  const tool: ToolDefinition<typeof Parameters, AgentsToolSnapshot | undefined, AgentsToolRenderState> = {
    name: "agents",
    label: "Agents",
    description:
      "Observe and control subagent and workflow runs belonging to the current parent session and cwd.",
    parameters: Parameters,
    outputSchema: Output,
    executionMode: "sequential",
    async execute(_toolCallId, input, _signal, onUpdate, ctx): Promise<AgentToolResult<AgentsToolSnapshot | undefined>> {
      const { output, snapshot } = await executeAction(input, ctx, onUpdate);
      const text = JSON.stringify(output, null, 2);
      const clipped = truncateHead(text, { maxBytes: 48_000, maxLines: 1_500 });
      return {
        content: [
          {
            type: "text",
            text: clipped.truncated
              ? `${clipped.content}\n[Output truncated; use a smaller limit or read the run artifacts.]`
              : text,
          },
        ],
        structuredContent: output,
        details: snapshot,
      };
    },
    renderCall(args, theme, context) {
      return renderAgentsCall(args.action, theme, context.state);
    },
    renderResult: renderAgentsResult,
  };
  pi.registerTool(tool);
}

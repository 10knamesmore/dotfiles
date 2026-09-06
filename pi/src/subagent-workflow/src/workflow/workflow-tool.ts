import type { AgentToolResult, ExtensionAPI, ExtensionContext, ToolDefinition } from "@earendil-works/pi-coding-agent";
import { truncateToWidth } from "@earendil-works/pi-tui";
import { Type, type Static } from "typebox";
import type { ParentContext } from "../runner/child.js";
import type { ThinkingLevel, WorkflowPhase } from "../types.js";
import { sanitizeTerminalText, sanitizeTerminalTextChunks, UNTRUSTED_FIELD_MAX } from "../ui/sanitize.js";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage, isRecord } from "../util.js";
import { linesComponent } from "../ui/component.js";
import type { ApproveLaunch, LaunchPlan, WorkflowApprovalPolicy } from "./approval.js";
import { completeWorkflowFailureInline, completeWorkflowInline, deliverWorkflowInBackground, launchWorkflow, type StartedWorkflow } from "./launch.js";
import { normalizeArgs, readAbsoluteScript, type WorkflowRunResult } from "./workflow-runner.js";
import { parseWorkflowScript } from "./parser.js";

const WorkflowToolParameters = Type.Object({
  script: Type.Optional(Type.String({
    description: "Inline workflow module source. Provide exactly one of script or scriptPath, including when resuming.",
  })),
  scriptPath: Type.Optional(Type.String({
    description: "Absolute path to a workflow module. Provide exactly one of script or scriptPath, including when resuming.",
  })),
  args: Type.Optional(Type.Unknown({
    description: "JSON-serializable deterministic input exposed as deep-frozen args. On resume, omit to reuse persisted args; an explicit value overrides them.",
  })),
  resumeRunId: Type.Optional(Type.String({
    description: "Existing workflow run id to resume with the supplied script or scriptPath; matching successful agent calls replay.",
  })),
  rerunChildIds: Type.Optional(Type.Array(Type.String({ minLength: 1 }), {
    minItems: 1,
    description: "Resume-only child ids explicitly authorized to rerun after execution-environment drift.",
  })),
}, { additionalProperties: false });

type WorkflowToolInput = Static<typeof WorkflowToolParameters>;

/** The launch-approval seam, injected so the tool stays testable. */
interface WorkflowToolServices {
  approve: ApproveLaunch;
  approvalPolicy: () => WorkflowApprovalPolicy;
  observeRun?: (run: StartedWorkflow, ctx: ExtensionContext) => void;
}

const DESCRIPTION = `Execute deterministic JavaScript orchestration over subagents. Use workflow when results feed later spawns, when you need phases, pipelines, or resumable control flow, or for more than about eight independent items; up to that many independent one-shot tasks are just that many subagent calls in one turn. Read the workflow-authoring skill before writing a non-trivial script or diagnosing a replay error; launch and runtime errors also name the exact rule violated.

The script is a module string beginning with a literal header:
export const meta = { name: 'audit-routes', description: 'Audit routes', phases: [{ title: 'Discover' }, { title: 'Audit' }] }
const result = await agent('List route files', { schema: { type: 'object', properties: { files: { type: 'array', items: { type: 'string' } } }, required: ['files'], additionalProperties: false } })
const files = result?.files.filter(Boolean) ?? []
phase('Audit')
return parallel(files.map(file => () => agent('Audit ' + file)))

Globals: agent(prompt, opts?), parallel(thunks), pipeline(items, ...stages), phase(title), log(message), and args. agent opts: model ("provider/model-id", never bare), thinkingLevel, tools, excludeTools, schema, cwd, isolation ('worktree' returns { value, patch, changed }; the patch is never applied automatically), label, phase. Every prompt must be self-contained: the child receives neither the parent conversation nor workflow variables unless interpolated. A failed agent() resolves to null - guard before dereferencing. Scripts must be deterministic: no wall-clock, randomness, or raw Promise concurrency - use parallel/pipeline and pass varying inputs through args. Resume with resumeRunId replays completed calls from the journal; drift on a completed call fails closed with an error naming the childId and the rerunChildIds recovery.

Every run is background: the call returns as soon as the workflow starts and completion arrives later as a steered parent message, so do not wait or poll - end the turn and continue when the message arrives. (In a host with no interactive UI the call instead blocks and returns the result inline.) A resumeRunId still requires exactly one of script or scriptPath.`;

/**
 * UI-side launch receipt rendered for the workflow tool row.
 *
 * The run is always background, so the row describes the workflow that started;
 * its outcome arrives as a steered message and is inspectable in /agents.
 */
export interface WorkflowToolDetails {
  status: "running";
  runId: string;
  runDir: string;
  phases: WorkflowPhase[];
}

/** Compact tool-row summary; the model reads the result content, not this. */
export function workflowSummaryLines(details: WorkflowToolDetails): string[] {
  const safe = (value: string | number): string => sanitizeTerminalText(String(value));
  const lines = [`${safe(details.runId)} - ${safe(details.status)}`];
  // Details round-trip through the session JSONL, so a resumed session can replay
  // a payload written by a different version of this file. Unlike the subagent
  // rows these lines are built eagerly in renderResult, which pi wraps in its own
  // try/catch, so a throw degrades to pi's fallback rather than killing the TUI -
  // checking the one collection is enough to keep the real summary instead.
  const phases = Array.isArray(details.phases) ? details.phases : [];
  if (phases.length > 0) lines.push(`phases: ${phases.map((phase) => safe(phase.title)).join(", ")}`);
  lines.push(`run dir: ${safe(details.runDir)}`);
  return lines;
}

/**
 * A details payload is drawable only if it carries the identity the summary
 * needs. A call this tool rejected arrives as a truthy `{}`, which must fall
 * through to the result text rather than being treated as a run.
 */
export function isWorkflowDetails(value: unknown): value is WorkflowToolDetails {
  return isRecord(value) && typeof value.runId === "string" && typeof value.runDir === "string";
}

export function registerWorkflowTool(pi: ExtensionAPI, selfPath: string, services: WorkflowToolServices): void {
  const tool: ToolDefinition<typeof WorkflowToolParameters, WorkflowToolDetails | undefined> = {
    name: "workflow",
    label: "Workflow",
    description: DESCRIPTION,
    parameters: WorkflowToolParameters,
    async execute(_toolCallId, params, signal, onUpdate, ctx): Promise<AgentToolResult<WorkflowToolDetails | undefined>> {
      const { script } = resolveScriptSource(params);
      const parsed = parseWorkflowScript(script);
      // Undefined means "reuse persisted args" on resume. Converting it to
      // null here would make the documented recovery invocation hash-miss.
      const args = normalizeWorkflowToolArgs(params.args);
      const plan: LaunchPlan = { workflow: parsed, args };
      const parent: ParentContext = {
        ctx,
        thinkingLevel: pi.getThinkingLevel() as ThinkingLevel,
        selfPath,
      };
      // Headless hosts (print/json mode) end the session when this turn ends,
      // which aborts the run - there is no later turn to steer a result into.
      // Only there does the call wait inline; only there is the turn's abort
      // signal bound, and only there do script log() lines stream to the tool
      // row (a background call has returned before any log can fire).
      const headless = !ctx.hasUI;
      const { started, execution } = await launchWorkflow(
        pi,
        parent,
        {
          plan,
          resumeRunId: params.resumeRunId,
          rerunChildIds: params.rerunChildIds,
          ...(headless ? {
            signal,
            onLog: (message: string) => onUpdate?.({ content: [{ type: "text", text: message }], details: undefined }),
          } : {}),
        },
        { approve: services.approve, ctx, deps: { policy: services.approvalPolicy() } },
      );
      try {
        services.observeRun?.(started, ctx);
      } catch (error) {
        reportDiagnostic(`[subagent-workflow] workflow observer failed: ${sanitizeTerminalText(errorMessage(error))}`);
      }
      const sessionId = ctx.sessionManager.getSessionId();
      if (headless) {
        let result: WorkflowRunResult;
        try {
          result = await execution;
        } catch (error) {
          throw completeWorkflowFailureInline(error, sessionId);
        }
        return { content: [{ type: "text", text: completeWorkflowInline(pi, result, sessionId) }], details: undefined };
      }
      // A background workflow outlives the turn: a later Esc must not abort
      // it - stop it from /agents instead.
      deliverWorkflowInBackground(pi, execution, sessionId);
      return {
        content: [{ type: "text", text: JSON.stringify({ runId: started.runId, runDir: started.runDir, phases: started.phases, status: "running" }) }],
        details: { status: "running", runId: started.runId, runDir: started.runDir, phases: started.phases },
      };
    },
    renderResult(result, _options, theme) {
      const textParts = (result.content ?? []).filter((part) => part.type === "text").map((part) => part.text);
      const lines = isWorkflowDetails(result.details)
        ? workflowSummaryLines(result.details).map((line, index) => index === 0 ? line : theme.fg("dim", line))
        : textParts.length === 0
          ? []
          : sanitizeTerminalTextChunks(textParts, UNTRUSTED_FIELD_MAX, true).split("\n");
      return linesComponent((width) => lines.map((line) => truncateToWidth(line, width)), "workflow summary");
    },
  };
  pi.registerTool(tool);
}

export function normalizeWorkflowToolArgs(args: unknown): unknown {
  return args === undefined ? undefined : normalizeArgs(args);
}

export function resolveScriptSource(
  params: WorkflowToolInput,
): { script: string } {
  const hasScript = params.script !== undefined;
  const hasPath = params.scriptPath !== undefined;
  if (hasScript === hasPath) throw new Error("Provide exactly one of workflow script or scriptPath; resumeRunId does not replace the script");
  if (hasPath) return { script: readAbsoluteScript(params.scriptPath!) };
  return { script: params.script! };
}

import { readdirSync, type Dirent } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { getAgentDir, type ExtensionAPI, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { ParentContext } from "../src/runner/child.js";
import { subagentRunner } from "../src/runner/runner.js";
import { preflightSubprocessChild } from "../src/runner/subprocess/spawn-child.js";
import {
  acknowledgeDeliveryMessage,
  claimRunDelivery,
  deliveryMarkerMatches,
  markSessionClosed,
  markSessionOpen,
  parseRunDeliveryIdentity,
  queueAcknowledgedDelivery,
  retryDeferredPublications,
  type ClaimedDeliveryTarget,
  type RunDeliveryIdentity,
} from "../src/store/delivery-marker.js";
import { runOwnerIsLive } from "../src/store/lease.js";
import { isLiveStatus, projectRunSnapshot, reconcileDeadOwnerProjection, snapshotSaysLive, type RunProjection } from "../src/store/run-projection.js";
import { encodeCwd, persistReconciledProjection } from "../src/store/run-store.js";
import { jsonObject, readRunSnapshot, type RunSnapshot } from "../src/store/run-snapshot.js";
import { readPersonalConfig } from "../../../config/index.js";
import { applyWorkflowSettings } from "../src/settings/runtime.js";
import { registerModelTiersCommand } from "../src/settings/model-tiers-command.js";
import { registerModelTierGuidance } from "../src/settings/model-tier-guidance.js";
import { fenceDirectlyDeliveredRun, registerSubagentTool, resolveFollowUpSpec } from "../src/tool/subagent-tool.js";
import { registerEntryMarkers } from "../src/ui/entry-markers.js";
import { registerNavigator, type NavigatorFollowUp, type NavigatorOpenContext } from "../src/ui/navigator/navigator.js";
import { PROMPT_EDITOR_CONFIGURE, type PromptEditorApi } from "../../editor/api.js";
import { FOLLOW_UP_PROMPT_PREFIX } from "../src/ui/navigator/transcript.js";
import { SubagentStatusWidget } from "../src/ui/status-widget.js";
import { safeDeliveryValue } from "../src/ui/delivery-safe.js";
import { SubagentUsageFooter } from "../src/ui/usage-footer.js";
import { reportDiagnostic, setTuiSession } from "../src/diagnostics.js";
import type { SubagentResult, ThinkingLevel } from "../src/types.js";
import { childLabel, errorMessage } from "../src/util.js";
import type { StartedWorkflow } from "../src/workflow/launch.js";
import { parseWorkflowScript } from "../src/workflow/parser.js";
import { registerWorkflowTool } from "../src/workflow/workflow-tool.js";

const selfPath = fileURLToPath(import.meta.url);
const CATCH_UP_RUN_CAP = 10;

type TerminalStatus = "completed" | "failed" | "aborted";

function createNavigatorFollowUp(
  pi: ExtensionAPI,
  extensionPath: string,
  widget: SubagentStatusWidget,
): NavigatorFollowUp {
  const runner = subagentRunner;

  return {
    send(runId, childId, prompt, ctx) {
      const message = prompt.trim();
      if (!message) throw new Error("Follow-up message must not be empty");
      if (message.startsWith("/")) throw new Error("Slash commands are not supported in agent follow-up messages");
      const resolved = resolveFollowUpSpec(`${runId}/${childId}`, `${FOLLOW_UP_PROMPT_PREFIX}${message}`, ctx.cwd);
      const parent: ParentContext = {
        ctx,
        thinkingLevel: pi.getThinkingLevel() as ThinkingLevel,
        selfPath: extensionPath,
      };
      const sessionId = ctx.sessionManager.getSessionId();
      const childDisplay = preflightSubprocessChild(resolved.spec, parent, { forkSessionFile: resolved.forkSessionFile });
      // directDelivery rides inside run.json, written before any child starts:
      // a crash at any later point leaves a run catch-up already knows to skip.
      const handle = runner.spawnRun(resolved, parent, { directDelivery: true });
      // spawnRun has persisted and started the child, so the run is committed
      // and nothing below may throw back to the caller: a thrown send() would
      // report a started run as a failed message and invite a duplicate spawn.
      void handle.result.then((result: SubagentResult) => {
        fenceDirectlyDeliveredRun(pi, runner, handle, result, sessionId, (degraded) => degraded);
        // The reply is model-fenced (directDelivery), so this notify is the
        // only signal a user who left the navigator gets that it arrived.
        ctx.ui.notify(`Reply from ${safeDeliveryValue(result.resolved.label)} ready - see /agents`, "info");
      }).catch((error) => {
        reportDiagnostic(`[subagent-workflow] navigator follow-up completion failed: ${errorMessage(error)}`);
      });
      try {
        widget.track(handle.runId, handle, ctx, {
          model: `${childDisplay.model.provider}/${childDisplay.model.id}`,
          thinking: childDisplay.thinking,
        });
      } catch (error) {
        reportDiagnostic(`[subagent-workflow] status widget failed: ${errorMessage(error)}`);
      }
      return { runId: handle.runId, childId: handle.id };
    },
  };
}

export interface CatchUpRun {
  runId: string;
  runDir: string;
  label: string;
  status: TerminalStatus;
  interruptedChildCount: number;
  lastActivityAt?: number;
  reason: string;
  recommendedAction: string;
  createdAt: number;
  generation: number;
}

/** Claim terminal runs using the same ownership lock as workflow resume. */
function claimCatchUpRuns(
  cwd: string,
  sessionId: string,
  runsRoot: string = join(getAgentDir(), "subagent-workflow", "runs"),
): CatchUpRun[] {
  const runRoot = join(runsRoot, encodeCwd(cwd));
  let entries: Dirent<string>[];
  try {
    entries = readdirSync(runRoot, { withFileTypes: true });
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return [];
    throw error;
  }

  const candidates: CatchUpRun[] = [];
  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const runDir = join(runRoot, entry.name);
    try {
      const snapshot = readRunSnapshot(runDir);
      if (snapshot.generationPending) continue;
      const record = jsonObject(snapshot.record);
      // Direct-delivery results were shown to the human in the navigator;
      // queueing them to the model would leak a private follow-up thread.
      if (record?.directDelivery === true) continue;
      const identity = parseRunDeliveryIdentity(record);
      if (!identity || deliveryMarkerMatches(runDir, identity)) continue;
      const parent = jsonObject(record?.parent);
      if (parent?.sessionId !== sessionId || runOwnerIsLive(runDir)) continue;
      const evaluation = evaluateCatchUpRun(snapshot, entry.name);
      if (!evaluation) continue;
      const { liveProjection, status, interruptedChildIds } = evaluation;
      const createdAt = typeof record?.createdAt === "string" ? Date.parse(record.createdAt) : NaN;
      candidates.push({
        runId: entry.name,
        runDir,
        label: persistedRunLabel(record),
        status,
        ...catchUpDetails(snapshot, liveProjection, status, interruptedChildIds),
        createdAt: Number.isFinite(createdAt) ? createdAt : 0,
        generation: identity.generation,
      });
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] catch-up scan skipped ${runDir}: ${errorMessage(error)}`);
    }
  }

  candidates.sort((left, right) => right.createdAt - left.createdAt || right.runId.localeCompare(left.runId));
  const claimed: CatchUpRun[] = [];
  for (const candidate of candidates) {
    let claim: ClaimedDeliveryTarget | "conflict" | undefined;
    try {
      if (runOwnerIsLive(candidate.runDir)) continue;
      const identity = catchUpIdentity(candidate);
      claim = claimRunDelivery(candidate.runDir, identity);
      if (claim === "conflict" || !claim) continue;
      const snapshot = readRunSnapshot(candidate.runDir);
      const record = jsonObject(snapshot.record);
      const parent = jsonObject(record?.parent);
      const currentIdentity = parseRunDeliveryIdentity(record);
      const evaluation = evaluateCatchUpRun(snapshot, candidate.runId);
      if (snapshot.generationPending
        || parent?.sessionId !== sessionId
        || !currentIdentity
        || currentIdentity.generation !== identity.generation
        || deliveryMarkerMatches(candidate.runDir, identity)
        || !evaluation) continue;
      const { liveProjection, projection, ownerWasDead, status, interruptedChildIds } = evaluation;
      if (ownerWasDead) {
        try {
          persistReconciledProjection(snapshot, projection, identity.generation, interruptedChildIds);
        } catch (error) {
          reportDiagnostic(`[subagent-workflow] catch-up reconcile failed for ${candidate.runDir}: ${errorMessage(error)}`);
          continue;
        }
      }
      claimed.push({ ...candidate, status, ...catchUpDetails(snapshot, liveProjection, status, interruptedChildIds) });
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] catch-up claim failed for ${candidate.runDir}: ${errorMessage(error)}`);
    } finally {
      if (claim !== "conflict") claim?.ownership.release();
    }
  }
  return claimed;
}

export function formatCatchUpMessage(runs: readonly CatchUpRun[]): string {
  const shown = runs.slice(0, CATCH_UP_RUN_CAP);
  const lines = shown.map((run) => [
    run.runId,
    run.label,
    `${run.status}: ${run.reason}`,
    run.lastActivityAt === undefined ? "last activity unknown" : `last activity ${new Date(run.lastActivityAt).toISOString()}`,
    `${run.recommendedAction} ${run.runDir}`,
  ].map(safeDeliveryValue).join(" | "));
  if (runs.length > shown.length) lines.push(`and ${runs.length - shown.length} more; see /agents`);
  return `Recovered background run deliveries:\n${lines.map((line) => `- ${line}`).join("\n")}`;
}

export function catchUpUndeliveredRuns(
  pi: ExtensionAPI,
  ctx: ExtensionContext,
  runsRoot?: string,
): CatchUpRun[] {
  // Settle acknowledged-but-conflicted publications first so a delivery that
  // was consumed in a previous session cannot be re-queued as undelivered.
  retryDeferredPublications();
  const sessionId = ctx.sessionManager.getSessionId();
  const runs = claimCatchUpRuns(ctx.cwd, sessionId, runsRoot);
  if (runs.length === 0) return runs;
  const message = formatCatchUpMessage(runs);
  queueAcknowledgedDelivery(pi, {
    sessionId,
    message,
    catchUp: true,
    targets: runs.map((run) => ({ runDir: run.runDir, identity: catchUpIdentity(run) })),
  });
  return runs;
}

function catchUpIdentity(run: CatchUpRun): RunDeliveryIdentity {
  return { generation: run.generation };
}

function evaluateCatchUpRun(snapshot: RunSnapshot, runId: string) {
  const liveProjection = projectRunSnapshot(snapshot, runId);
  const ownerWasDead = snapshotSaysLive(snapshot);
  const projection = ownerWasDead ? reconcileDeadOwnerProjection(liveProjection) : liveProjection;
  const status = projection.summary.corrupt ? undefined : terminalStatus(projection.summary.status);
  if (!status) return undefined;
  const interruptedChildIds = ownerWasDead ? projectionInterruptedChildIds(liveProjection, projection) : [];
  return { liveProjection, projection, ownerWasDead, status, interruptedChildIds };
}

function catchUpDetails(
  snapshot: RunSnapshot,
  liveProjection: RunProjection,
  status: TerminalStatus,
  interruptedChildIds: readonly string[],
): Pick<CatchUpRun, "interruptedChildCount" | "lastActivityAt" | "reason" | "recommendedAction"> {
  const interruptedChildCount = interruptedChildIds.length;
  const lastActivityAt = projectionLastActivityAt(snapshot, liveProjection);
  if (interruptedChildCount > 0) {
    const agent = interruptedChildCount === 1 ? "agent was" : "agents were";
    return {
      interruptedChildCount,
      ...(lastActivityAt === undefined ? {} : { lastActivityAt }),
      reason: `parent process exited while ${interruptedChildCount} ${agent} running`,
      recommendedAction: status === "completed" ? "review result from" : "restart or resume from",
    };
  }
  return {
    interruptedChildCount,
    ...(lastActivityAt === undefined ? {} : { lastActivityAt }),
    reason: "run finished before its result was delivered",
    recommendedAction: status === "completed" ? "review result from" : "inspect or resume from",
  };
}

function projectionInterruptedChildIds(liveProjection: RunProjection, projection: RunProjection): string[] {
  const reconciledStatuses = new Map(projection.detail.children.map((child) => [child.id, child.status]));
  return liveProjection.detail.children.flatMap((child) => {
    return isLiveStatus(child.status)
      && !liveProjection.terminalStatuses.has(child.id)
      && reconciledStatuses.get(child.id) === "aborted"
      ? [child.id]
      : [];
  });
}

function projectionLastActivityAt(snapshot: RunSnapshot, projection: RunProjection): number | undefined {
  const timestamps = [
    ...projection.detail.children.flatMap((child) => [child.startedAt, child.endedAt]),
    ...projection.detail.narrator.map((line) => line.timestamp),
    ...snapshot.events.flatMap((value) => {
      const event = jsonObject(value);
      if (!event || event.type === "crash_reconciled") return [];
      const timestamp = typeof event.timestamp === "string" ? Date.parse(event.timestamp) : event.timestamp;
      return [timestamp];
    }),
  ].filter((value): value is number => typeof value === "number" && Number.isFinite(value) && value > 0);
  return timestamps.length > 0 ? Math.max(...timestamps) : undefined;
}

function terminalStatus(value: unknown): TerminalStatus | undefined {
  return value === "completed" || value === "failed" || value === "aborted" ? value : undefined;
}

function persistedRunLabel(record: Record<string, unknown> | undefined): string {
  const children = Array.isArray(record?.children) ? record.children : [];
  if (record?.kind === "workflow") return "workflow";
  const child = jsonObject(children[0]);
  const resolvedLabel = jsonObject(child?.resolved)?.label;
  if (typeof resolvedLabel === "string" && resolvedLabel.trim()) return resolvedLabel.trim();
  const spec = jsonObject(child?.spec);
  if (typeof spec?.prompt === "string") {
    return childLabel({ prompt: spec.prompt, ...(typeof spec.label === "string" ? { label: spec.label } : {}) });
  }
  return "subagent";
}

function userMessageText(content: string | Array<{ type: string; text?: string }>): string {
  if (typeof content === "string") return content;
  return content.filter((part) => part.type === "text").map((part) => part.text ?? "").join("");
}

export default function subagentWorkflow(pi: ExtensionAPI): void {
  const widget = new SubagentStatusWidget(subagentRunner);
  const usageFooter = new SubagentUsageFooter(subagentRunner);
  const settings = readPersonalConfig()["subagent-workflow"];
  applyWorkflowSettings(settings, widget);
  registerEntryMarkers(pi);
  registerModelTiersCommand(pi);
  registerModelTierGuidance(pi);
  registerSubagentTool(pi, selfPath, widget);
  const observeRun = (run: StartedWorkflow, ctx: ExtensionContext) => {
    usageFooter.trackRun(run.runDir, ctx);
    widget.observeWorkflowStarted(run, ctx);
  };
  registerWorkflowTool(pi, selfPath, { approvalPolicy: settings.workflowApproval, observeRun });

  // /agents is the canonical name; /workflows stays registered as a public alias.
  const openNavigator = registerNavigator(pi, {
    followUp: createNavigatorFollowUp(pi, selfPath, widget),
    describeWorkflow: (script) => parseWorkflowScript(script).meta.name,
  });
  pi.registerShortcut("shift+down", {
    description: "Open the agent navigator (/agents)",
    handler: async (ctx) => {
      if (!ctx.hasUI) return;
      await openNavigator(ctx);
    },
  });
  pi.events.on(PROMPT_EDITOR_CONFIGURE, (requested) => {
    (requested as PromptEditorApi).useAgentNavigation({
      selectNext: () => widget.selectRun(1),
      selectPrevious: () => widget.selectRun(-1),
      hasSelection: () => widget.hasSelectedRun(),
      openSelection: () => {
        const target = widget.takeSelectedRun();
        if (!target) return false;
        const ctx = editorContext;
        if (!ctx) return false;
        void openNavigator(ctx, target).catch((error: unknown) => {
          const message = `Could not open agent view: ${errorMessage(error)}`;
          ctx.ui.notify(message, "error");
          reportDiagnostic(`[subagent-workflow] ${message}`);
        });
        return true;
      },
      clearSelection: () => widget.clearSelectedRun(),
    });
  });
  let editorContext: ExtensionContext | undefined;
  pi.on("message_start", (event, ctx) => {
    if (event.message.role !== "user") return;
    acknowledgeDeliveryMessage(ctx.sessionManager.getSessionId(), userMessageText(event.message.content));
  });
  pi.on("session_start", (_event, ctx) => {
    const sessionId = ctx.sessionManager.getSessionId();
    markSessionOpen(sessionId);
    editorContext = ctx;
    if (ctx.hasUI) setTuiSession();
    usageFooter.attach(ctx);
    try {
      catchUpUndeliveredRuns(pi, ctx);
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] startup catch-up failed: ${errorMessage(error)}`);
    }
  });
  pi.on("session_shutdown", async (_event, ctx) => {
    const sessionId = ctx.sessionManager.getSessionId();
    markSessionClosed(sessionId);
    editorContext = undefined;
    try {
      widget.dispose();
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] status widget disposal failed: ${errorMessage(error)}`);
    }
    try {
      usageFooter.dispose();
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] usage footer disposal failed: ${errorMessage(error)}`);
    }
    try {
      await subagentRunner.disposeForSession(sessionId);
    } catch (error) {
      reportDiagnostic(`[subagent-workflow] child disposal failed: ${errorMessage(error)}`);
    }
  });
}

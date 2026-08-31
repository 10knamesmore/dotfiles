import { AsyncLocalStorage } from "node:async_hooks";
import { existsSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";
import { isMainThread, parentPort, Worker, workerData } from "node:worker_threads";
import { errorMessage } from "../util.ts";
// Explicit .ts specifier: this module is a worker entry (new Worker(import.meta.url))
// loaded by plain Node type stripping when running from source, which does not remap
// .js specifiers to .ts files. rewriteRelativeImportExtensions emits .js in dist.
import { runInConstrainedContext, type SandboxHostBridge, type WorkflowSandboxInput } from "./vm-sandbox.ts";

export interface WorkflowCallScopeSegment {
  operation: number;
  branch: number;
  kind: "parallel" | "pipeline";
}

/** Stable causal identity for an agent call, independent of completion order. */
export interface WorkflowCallIdentity {
  scope: WorkflowCallScopeSegment[];
  operation: number;
}

export interface WorkflowVmApi {
  agent: (prompt: string, options: Record<string, unknown> | undefined, call: WorkflowCallIdentity) => Promise<unknown>;
  log: (message: string) => void;
  /** Host-internal diagnostics channel; must never surface in user-facing workflow narration. */
  diagnostic?: (message: string) => void;
  phase: (title: string) => void;
  args: unknown;
  /** Durable child identities for watchdog diagnostics while host calls are outstanding. */
  describeActiveAgents?: () => readonly string[];
  /** Aborting this signal preempts the workflow worker, including blocked native calls. */
  signal?: AbortSignal;
}

const DEFAULT_SYNCHRONOUS_TIMEOUT_MS = 30_000;
const WORKER_MARKER = "pi-subagent-workflow-vm-v1";

/**
 * Resolve the worker entry for this module.
 *
 * The host runs under jiti, which transpiles .ts imports in-process, but a
 * worker_threads entry is loaded by Node's native loader, which refuses to
 * type-strip files under node_modules (ERR_UNSUPPORTED_NODE_MODULES_TYPE_STRIPPING).
 * For npm installs the package always ships a fresh compiled worker, so redirect
 * to it. Outside node_modules (dev checkouts, path installs) keep the source
 * entry: dist/ may be stale there, and native type stripping is permitted.
 */
export function resolveWorkerEntryUrl(moduleUrl: string, exists: (path: string) => boolean = existsSync): URL {
  const url = new URL(moduleUrl);
  const installedSource = url.protocol === "file:"
    && url.pathname.endsWith("/src/workflow/vm.ts")
    && url.pathname.includes("/node_modules/");
  if (!installedSource) return url;

  const compiled = new URL(url.href);
  compiled.pathname = url.pathname.replace(/\/src\/workflow\/vm\.ts$/, "/dist/src/workflow/vm.js");
  compiled.search = "";
  compiled.hash = "";
  // Under node_modules the compiled worker is the only entry that can work, so a
  // missing one is fatal rather than a reason to fall back. Falling back here used
  // to re-create the exact bug this redirect exists to prevent, and reported it as
  // an opaque ERR_UNSUPPORTED_NODE_MODULES_TYPE_STRIPPING from deep inside Node.
  if (!exists(fileURLToPath(compiled))) {
    throw new Error(
      `Workflow worker is missing from the installed package: ${fileURLToPath(compiled)}. `
      + "Node cannot strip types from .ts files under node_modules, so the compiled worker must ship. "
      + "Reinstall pi-subagent-workflow.",
    );
  }
  return compiled;
}

interface WorkflowWorkerData extends WorkflowSandboxInput {
  marker: typeof WORKER_MARKER;
}

interface ExecutionScope {
  path: WorkflowCallScopeSegment[];
  nextOperation: number;
  activeBranchGroups: number;
  agentInFlight: boolean;
  wakeQuiescence?: () => void;
}

interface PendingAgentRequest {
  resolve: (resultJson: string | undefined) => void;
  reject: (error: Error) => void;
  scope: ExecutionScope;
}

type WorkerRequest =
  | { type: "heartbeat" }
  | { type: "agent"; id: number; promptJson: string; optionsJson?: string; call: WorkflowCallIdentity }
  | { type: "phase"; titleJson: string }
  | { type: "log"; message: string }
  | { type: "result"; resultJson?: string }
  | { type: "error"; name: string; message: string; stack?: string };

type WorkerResponse =
  | { type: "ping" }
  | { type: "agent-result"; id: number; resultJson?: string }
  | { type: "agent-error"; id: number; message: string };

/**
 * Execute workflow-authored JavaScript in a disposable worker.
 *
 * A node:vm context constrains capabilities, while the worker is the
 * preemption boundary. The worker sends heartbeats from its own event loop, so
 * a long awaited agent call remains healthy but a JavaScript loop or blocking
 * native call can be terminated without wedging the host process.
 */
export async function executeWorkflowBody(
  body: string,
  name: string,
  api: WorkflowVmApi,
  synchronousTimeoutMs = DEFAULT_SYNCHRONOUS_TIMEOUT_MS,
): Promise<unknown> {
  if (!Number.isSafeInteger(synchronousTimeoutMs) || synchronousTimeoutMs <= 0) {
    throw new TypeError("Workflow VM synchronousTimeoutMs must be a positive integer");
  }

  const argsJson = JSON.stringify(api.args);
  if (argsJson === undefined) throw new TypeError("Workflow args must be JSON-serializable");
  if (api.signal?.aborted) throw new Error("Workflow stopped");

  // Yield once before constructing the worker. Run-level controllers are
  // registered immediately before this function is called, so an immediate
  // session shutdown can cancel cleanly without racing a worker that has not
  // reached its online state yet.
  await Promise.resolve();
  if (api.signal?.aborted) throw new Error("Workflow stopped");

  const heartbeatIntervalMs = Math.max(10, Math.min(1_000, Math.floor(synchronousTimeoutMs / 4)));
  const data: WorkflowWorkerData = {
    marker: WORKER_MARKER,
    body,
    name,
    argsJson,
    synchronousTimeoutMs,
  };

  return new Promise<unknown>((resolve, reject) => {
    const worker = new Worker(resolveWorkerEntryUrl(import.meta.url), {
      workerData: data,
      resourceLimits: {
        maxOldGenerationSizeMb: 128,
        maxYoungGenerationSizeMb: 32,
        codeRangeSizeMb: 16,
        stackSizeMb: 4,
      },
    });
    let settled = false;
    const workerStartedAt = performance.now();
    const startupTimeoutMs = Math.max(5_000, synchronousTimeoutMs);
    let receivedFirstHeartbeat = false;
    let lastHeartbeatAt = workerStartedAt;
    let livenessChallenge: { sentAt: number; heartbeatAt: number } | undefined;
    const outstandingAgentRequests = new Map<number, { startedAt: number; warned: boolean }>();
    const challengeWindowMs = Math.max(25, heartbeatIntervalMs);

    const cleanup = (): void => {
      clearInterval(watchdog);
      api.signal?.removeEventListener("abort", onAbort);
      worker.removeAllListeners();
    };
    const finish = (outcome: { value: unknown } | { error: unknown }): void => {
      if (settled) return;
      settled = true;
      cleanup();
      // In particular, wait for termination when abort races worker startup.
      // Resolving first can let a not-yet-online Bun worker start afterward
      // and keep the process alive while blocked inside node:vm.
      void worker.terminate().then(
        () => {
          if ("error" in outcome) reject(outcome.error);
          else resolve(outcome.value);
        },
        (terminationError) => reject(terminationError),
      );
    };
    const fail = (error: unknown): void => finish({ error: toError(error) });
    const post = (message: WorkerResponse): void => {
      if (settled) return;
      try {
        worker.postMessage(message);
      } catch (error) {
        fail(error);
      }
    };
    const onAbort = (): void => fail(new Error("Workflow stopped"));

    const watchdog = setInterval(() => {
      const now = performance.now();
      const deadlineMs = receivedFirstHeartbeat ? synchronousTimeoutMs : startupTimeoutMs;
      const progressAt = receivedFirstHeartbeat ? lastHeartbeatAt : workerStartedAt;
      if (receivedFirstHeartbeat && now - progressAt < deadlineMs) {
        const overdue = [...outstandingAgentRequests.entries()]
          .filter(([, request]) => !request.warned && now - request.startedAt >= synchronousTimeoutMs);
        if (overdue.length > 0) {
          for (const [, request] of overdue) request.warned = true;
          let activeAgents: readonly string[] = [];
          try {
            activeAgents = api.describeActiveAgents?.() ?? [];
          } catch {
            // Diagnostics must never interfere with the child they observe.
          }
          const context = activeAgents.length > 0
            ? ` Active children: ${activeAgents.join(", ")}.`
            : ` Outstanding host agent request ids: ${overdue.map(([id]) => id).join(", ")}.`;
          try {
            api.diagnostic?.(`Child/tool work is still running after ${synchronousTimeoutMs}ms while the workflow worker remains responsive.${context} Waiting for child completion or the configured agent timeout.`);
          } catch {
            // Diagnostics must never interfere with the child they observe.
          }
        }
      }
      if (now - progressAt >= deadlineMs) {
        // A blocked host event loop can delay both this timer and already-queued
        // worker heartbeats. The first stale observation therefore challenges
        // the worker directly; only a later turn that still has no answer may
        // convict authored synchronous execution.
        if (!livenessChallenge) {
          livenessChallenge = { sentAt: now, heartbeatAt: lastHeartbeatAt };
          post({ type: "ping" });
          return;
        }
        if (lastHeartbeatAt > livenessChallenge.heartbeatAt) {
          livenessChallenge = undefined;
          return;
        }
        if (now - livenessChallenge.sentAt < challengeWindowMs) return;
        let activeAgents: readonly string[] = [];
        try {
          activeAgents = api.describeActiveAgents?.() ?? [];
        } catch {
          // Diagnostics must never mask the watchdog failure they describe.
        }
        const context = activeAgents.length > 0
          ? ` Active children: ${activeAgents.join(", ")}.`
          : outstandingAgentRequests.size > 0
            ? ` Outstanding host agent request ids: ${[...outstandingAgentRequests.keys()].sort((a, b) => a - b).join(", ")}.`
            : "";
        const message = receivedFirstHeartbeat
          ? `Workflow worker failed a direct liveness challenge after ${synchronousTimeoutMs}ms of uninterrupted synchronous execution.${context}`
          : `Workflow worker failed its startup liveness challenge after ${startupTimeoutMs}ms.${context}`;
        fail(new Error(message));
        return;
      }
      post({ type: "ping" });
    }, heartbeatIntervalMs);

    worker.on("message", (message: WorkerRequest) => {
      if (settled) return;
      switch (message.type) {
        case "heartbeat":
          receivedFirstHeartbeat = true;
          lastHeartbeatAt = performance.now();
          livenessChallenge = undefined;
          return;
        case "phase":
          try {
            api.phase(JSON.parse(message.titleJson) as string);
          } catch (error) {
            fail(error);
          }
          return;
        case "log":
          try {
            api.log(message.message);
          } catch (error) {
            fail(error);
          }
          return;
        case "agent":
          outstandingAgentRequests.set(message.id, { startedAt: performance.now(), warned: false });
          void answerAgentRequest(worker, message, api, post, fail)
            .finally(() => outstandingAgentRequests.delete(message.id));
          return;
        case "result":
          try {
            finish({ value: message.resultJson === undefined ? undefined : JSON.parse(message.resultJson) });
          } catch (error) {
            fail(error);
          }
          return;
        case "error": {
          const error = new Error(message.message);
          error.name = message.name;
          if (message.stack) error.stack = message.stack;
          fail(error);
        }
      }
    });
    worker.once("error", fail);
    worker.once("messageerror", fail);
    worker.once("exit", (code) => {
      if (!settled) fail(new Error(`Workflow worker exited before returning a result (code ${code})`));
    });
    api.signal?.addEventListener("abort", onAbort, { once: true });
    if (api.signal?.aborted) onAbort();
  });
}

async function answerAgentRequest(
  worker: Worker,
  request: Extract<WorkerRequest, { type: "agent" }>,
  api: WorkflowVmApi,
  post: (message: WorkerResponse) => void,
  fail: (error: unknown) => void,
): Promise<void> {
  try {
    const prompt = JSON.parse(request.promptJson) as string;
    const agentOptions = request.optionsJson === undefined
      ? undefined
      : JSON.parse(request.optionsJson) as Record<string, unknown>;
    const result = await api.agent(prompt, agentOptions, request.call);
    const resultJson = JSON.stringify(result);
    post({ type: "agent-result", id: request.id, resultJson });
  } catch (error) {
    try {
      post({
        type: "agent-error",
        id: request.id,
        message: errorMessage(error),
      });
    } catch (postError) {
      // Keep worker lifecycle failures on the main settlement path.
      worker.removeAllListeners("message");
      fail(postError);
    }
  }
}

function toError(value: unknown): Error {
  return value instanceof Error ? value : new Error(String(value));
}

function isWorkflowWorkerData(value: unknown): value is WorkflowWorkerData {
  return !!value && typeof value === "object" && (value as { marker?: unknown }).marker === WORKER_MARKER;
}

async function runWorkflowWorker(data: WorkflowWorkerData): Promise<void> {
  const port = parentPort;
  if (!port) throw new Error("Workflow worker started without a parent message port");

  // Bun's node:vm stack formatter reads the worker realm's Error controls
  // rather than the context's controls. The worker is disposable, so seal
  // both layers before any authored Error can capture host-specific frames.
  Object.defineProperty(Error, "stackTraceLimit", {
    value: 0,
    writable: false,
    configurable: false,
  });
  Object.defineProperty(Error, "prepareStackTrace", {
    value: () => undefined,
    writable: false,
    configurable: false,
  });

  const pendingAgents = new Map<number, PendingAgentRequest>();
  const bufferedAgentResponses = new Map<number, Extract<WorkerResponse, { type: "agent-result" | "agent-error" }>>();
  const pendingWaiters = new Set<() => void>();
  let nextAgentId = 0;
  let nextAgentResponseId = 0;
  const signalPendingProgress = (): void => {
    for (const wake of pendingWaiters) wake();
    pendingWaiters.clear();
  };
  const scopes = new AsyncLocalStorage<ExecutionScope>();
  port.postMessage({ type: "heartbeat" } satisfies WorkerRequest);

  const deliverBufferedAgentResponses = (): void => {
    while (true) {
      const message = bufferedAgentResponses.get(nextAgentResponseId);
      if (!message) return;
      bufferedAgentResponses.delete(nextAgentResponseId);
      nextAgentResponseId += 1;
      const pending = pendingAgents.get(message.id);
      if (!pending) continue;
      pendingAgents.delete(message.id);
      signalPendingProgress();
      pending.scope.agentInFlight = false;
      signalScopeProgress(pending.scope);
      if (message.type === "agent-error") pending.reject(new Error(message.message));
      else pending.resolve(message.resultJson);
    }
  };

  port.on("message", (message: WorkerResponse) => {
    if (message.type === "ping") {
      port.postMessage({ type: "heartbeat" } satisfies WorkerRequest);
      return;
    }
    if (message.type !== "agent-result" && message.type !== "agent-error") return;
    bufferedAgentResponses.set(message.id, message);
    deliverBufferedAgentResponses();
  });

  const bridge: SandboxHostBridge = {
    agent: (prompt, options) => {
      const scope = requireExecutionScope(scopes);
      if (scope.agentInFlight || scope.activeBranchGroups > 0) {
        throw new Error("Overlapping agent() calls or branch groups in the same workflow scope are unavailable. Await the current operation and use parallel() or pipeline() for concurrency");
      }
      scope.agentInFlight = true;
      try {
        const promptJson = serializeVmValue(prompt, "agent prompt");
        const optionsJson = options === undefined ? undefined : serializeVmValue(options, "agent options");
        const call = { scope: scope.path, operation: scope.nextOperation++ } satisfies WorkflowCallIdentity;
        const id = nextAgentId++;
        return new Promise<string | undefined>((resolve, reject) => {
          pendingAgents.set(id, { resolve, reject, scope });
          try {
            port.postMessage({ type: "agent", id, promptJson, optionsJson, call } satisfies WorkerRequest);
          } catch (error) {
            bufferedAgentResponses.set(id, {
              type: "agent-error",
              id,
              message: toError(error).message,
            });
            deliverBufferedAgentResponses();
          }
        });
      } catch (error) {
        scope.agentInFlight = false;
        throw error;
      }
    },
    phase: (title) => {
      const scope = requireExecutionScope(scopes);
      if (scope.path.length > 0 || !scopeIsQuiescent(scope)) {
        throw new Error("phase() is only available at an idle workflow root. Use agent opts.phase inside parallel() or pipeline() branches");
      }
      port.postMessage({ type: "phase", titleJson: serializeVmValue(title, "phase title") } satisfies WorkerRequest);
    },
    log: (message) => {
      port.postMessage({ type: "log", message: String(message) } satisfies WorkerRequest);
    },
    beginBranchGroup: () => {
      const scope = requireExecutionScope(scopes);
      if (!scopeIsQuiescent(scope)) {
        throw new Error("Overlapping parallel() or pipeline() groups and agent() calls in the same workflow scope are unavailable. Await the current operation first");
      }
      scope.activeBranchGroups += 1;
      return scope.nextOperation++;
    },
    endBranchGroup: () => {
      const scope = requireExecutionScope(scopes);
      scope.activeBranchGroups = Math.max(0, scope.activeBranchGroups - 1);
      signalScopeProgress(scope);
    },
    runBranch: (kind, operation, branch, thunk) => {
      const parent = requireExecutionScope(scopes);
      const child: ExecutionScope = {
        path: [...parent.path, { kind, operation, branch }],
        nextOperation: 0,
        activeBranchGroups: 0,
        agentInFlight: false,
      };
      return scopes.run(child, () => runScopeToQuiescence(child, thunk));
    },
  };

  let result: unknown;
  let workflowFailed = false;
  let workflowError: unknown;
  const rootScope: ExecutionScope = {
    path: [],
    nextOperation: 0,
    activeBranchGroups: 0,
    agentInFlight: false,
  };
  try {
    result = await scopes.run(
      rootScope,
      () => runInConstrainedContext(data, bridge),
    );
  } catch (error) {
    workflowFailed = true;
    workflowError = error;
  }

  try {
    await drainExecutionScope(rootScope);
    await drainPendingAgentRequests(pendingAgents, pendingWaiters);
    if (workflowFailed) throw workflowError;
    const resultJson = result === undefined ? undefined : serializeVmValue(result, "workflow return value");
    port.postMessage({ type: "result", resultJson } satisfies WorkerRequest);
  } catch (error) {
    const normalized = toError(error);
    port.postMessage({
      type: "error",
      name: normalized.name,
      message: normalized.message,
      stack: normalized.stack,
    } satisfies WorkerRequest);
  } finally {
    port.close();
  }
}

async function drainPendingAgentRequests(
  pendingAgents: Map<number, PendingAgentRequest>,
  pendingWaiters: Set<() => void>,
): Promise<void> {
  while (true) {
    while (pendingAgents.size > 0) {
      await new Promise<void>((resolve) => pendingWaiters.add(resolve));
    }
    await new Promise<void>((resolve) => setImmediate(resolve));
    if (pendingAgents.size === 0) return;
  }
}

async function runScopeToQuiescence(scope: ExecutionScope, run: () => unknown): Promise<unknown> {
  let result: unknown;
  let failed = false;
  let failure: unknown;
  try {
    result = await run();
  } catch (error) {
    failed = true;
    failure = error;
  }
  await drainExecutionScope(scope);
  if (failed) throw failure;
  return result;
}

async function drainExecutionScope(scope: ExecutionScope): Promise<void> {
  while (true) {
    while (!scopeIsQuiescent(scope)) {
      await new Promise<void>((resolve) => {
        scope.wakeQuiescence = resolve;
      });
    }
    // Promise continuations can start another unawaited call after resolving
    // the request that emptied the scope. Cross one event-loop turn so all such
    // microtasks run before deciding the workflow is finished.
    await new Promise<void>((resolve) => setImmediate(resolve));
    if (scopeIsQuiescent(scope)) return;
  }
}

function scopeIsQuiescent(scope: ExecutionScope): boolean {
  return !scope.agentInFlight && scope.activeBranchGroups === 0;
}

function signalScopeProgress(scope: ExecutionScope): void {
  const wake = scope.wakeQuiescence;
  scope.wakeQuiescence = undefined;
  wake?.();
}

function requireExecutionScope(scopes: AsyncLocalStorage<ExecutionScope>): ExecutionScope {
  const scope = scopes.getStore();
  if (!scope) throw new Error("Workflow VM lost its deterministic execution scope");
  return scope;
}

function serializeVmValue(value: unknown, label: string): string {
  try {
    const json = JSON.stringify(value);
    if (json === undefined) throw new TypeError(`${label} is not JSON-serializable`);
    return json;
  } catch (error) {
    if (error instanceof TypeError && error.message === `${label} is not JSON-serializable`) throw error;
    throw new TypeError(`${label} is not JSON-serializable: ${errorMessage(error)}`);
  }
}

if (!isMainThread && isWorkflowWorkerData(workerData)) {
  void runWorkflowWorker(workerData);
}

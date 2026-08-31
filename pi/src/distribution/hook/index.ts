/**
 * Applies the shared Rust PreToolUse rules to Pi tool calls.
 *
 * The Rust adapter owns policy evaluation. This module only transports Pi's
 * event shape, presents `ask` decisions, and translates a refusal into Pi's
 * pre-execution block result.
 */

import { spawn } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";
import type {
  ExtensionAPI,
  ExtensionContext,
  ToolCallEvent,
} from "@earendil-works/pi-coding-agent";

const AGENT_HOOK_PATH = join(homedir(), ".local", "bin", "agent-hook");
const ADAPTER_TIMEOUT_MS = 2_000;
const MAX_ADAPTER_OUTPUT_BYTES = 64 * 1024;
const SUMMARY_MAX_CHARS = 600;

type AdapterDecision =
  | { decision: "allow" }
  | { decision: "deny" | "ask"; reason: string };

interface AdapterProcessResult {
  /** Structured adapter output written to stdout. */
  stdout: string;

  /** Fail-open diagnostics written to stderr. */
  stderr: string;
}

type DiagnosticKind = "adapter-process" | "adapter-notice" | "adapter-output";

const reportedDiagnostics = new Set<DiagnosticKind>();

/** Register the Pi pre-execution bridge to the shared `agent-hook` policy. */
export function registerHook(pi: ExtensionAPI): void {
  pi.on("tool_call", async (event, ctx) => {
    let processResult: AdapterProcessResult;
    try {
      processResult = await runAdapter(event, ctx.cwd);
    } catch (error: unknown) {
      reportOnce(
        ctx,
        "adapter-process",
        `工具守卫不可用，当前调用已放行：${errorMessage(error)}`,
      );
      return undefined;
    }

    if (processResult.stderr.trim()) {
      reportOnce(ctx, "adapter-notice", processResult.stderr);
    }

    let verdict: AdapterDecision;
    try {
      verdict = parseDecision(processResult.stdout);
    } catch (error: unknown) {
      reportOnce(
        ctx,
        "adapter-output",
        `工具守卫返回无效结果，当前调用已放行：${errorMessage(error)}`,
      );
      return undefined;
    }

    if (verdict.decision === "allow") return undefined;
    if (verdict.decision === "deny") {
      return { block: true, reason: verdict.reason };
    }

    if (!ctx.hasUI) {
      return {
        block: true,
        reason: `当前 Pi mode 没有可回答的 approval UI，已拒绝需要确认的调用。原规则：${verdict.reason}`,
      };
    }

    let approved: boolean;
    try {
      approved = await ctx.ui.confirm(
        "确认工具调用",
        `${sanitizeDisplay(verdict.reason)}\n\n${event.toolName}: ${summarizeInput(event)}`,
      );
    } catch (error: unknown) {
      return {
        block: true,
        reason: `无法完成工具调用确认，已拒绝执行。原规则：${verdict.reason}。UI 错误：${errorMessage(error)}`,
      };
    }
    return approved
      ? undefined
      : { block: true, reason: `用户拒绝工具调用。原规则：${verdict.reason}` };
  });
}

export default registerHook;

/** Execute one bounded adapter process without a shell. */
function runAdapter(
  event: ToolCallEvent,
  cwd: string,
): Promise<AdapterProcessResult> {
  const stdin = JSON.stringify({
    tool_name: event.toolName,
    tool_input: event.input,
  });
  return new Promise((resolve, reject) => {
    const child = spawn(AGENT_HOOK_PATH, ["pi-pretool"], {
      cwd,
      stdio: ["pipe", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    let stdoutBytes = 0;
    let stderrBytes = 0;
    let settled = false;

    const finish = (result: AdapterProcessResult | Error): void => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      if (result instanceof Error) reject(result);
      else resolve(result);
    };
    const collect = (stream: "stdout" | "stderr", chunk: Buffer): void => {
      if (stream === "stdout") {
        stdoutBytes += chunk.byteLength;
        if (stdoutBytes > MAX_ADAPTER_OUTPUT_BYTES) {
          child.kill();
          finish(new Error("stdout 超过 64 KiB 上限"));
          return;
        }
        stdout += chunk.toString("utf8");
        return;
      }
      stderrBytes += chunk.byteLength;
      if (stderrBytes > MAX_ADAPTER_OUTPUT_BYTES) {
        child.kill();
        finish(new Error("stderr 超过 64 KiB 上限"));
        return;
      }
      stderr += chunk.toString("utf8");
    };

    const timeout = setTimeout(() => {
      child.kill();
      finish(new Error(`超过 ${ADAPTER_TIMEOUT_MS}ms 未完成`));
    }, ADAPTER_TIMEOUT_MS);
    child.stdout.on("data", (chunk: Buffer) => collect("stdout", chunk));
    child.stderr.on("data", (chunk: Buffer) => collect("stderr", chunk));
    child.on("error", (error: Error) => finish(error));
    child.on("close", (code, signal) => {
      if (code === 0) {
        finish({ stdout, stderr });
        return;
      }
      finish(
        new Error(
          `进程异常退出：code=${String(code)} signal=${String(signal)}`,
        ),
      );
    });
    child.stdin.on("error", (error: Error) => finish(error));
    child.stdin.end(stdin);
  });
}

/** Validate the adapter's discriminated JSON response. */
function parseDecision(stdout: string): AdapterDecision {
  const text = stdout.trim();
  if (!text) throw new Error("stdout 为空");
  const parsed: unknown = JSON.parse(text);
  if (!isRecord(parsed)) throw new Error("顶层结果不是 object");
  if (parsed.decision === "allow") return { decision: "allow" };
  if (
    (parsed.decision === "deny" || parsed.decision === "ask") &&
    typeof parsed.reason === "string"
  ) {
    return { decision: parsed.decision, reason: parsed.reason };
  }
  throw new Error("decision/reason 不符合协议");
}

/** Show each infrastructure failure class at most once per extension lifetime. */
function reportOnce(
  ctx: ExtensionContext,
  kind: DiagnosticKind,
  message: string,
): void {
  if (reportedDiagnostics.has(kind)) return;
  reportedDiagnostics.add(kind);
  const diagnostic = `[pi-distribution] ${sanitizeDisplay(message)}`;
  console.error(diagnostic);
  if (ctx.hasUI) ctx.ui.notify(diagnostic, "warning");
}

/** Render the important input without dumping a write body into the dialog. */
function summarizeInput(event: ToolCallEvent): string {
  const input: unknown = event.input;
  if (isRecord(input)) {
    const command = input.command;
    if (typeof command === "string") return truncateDisplay(command);
    const path = input.path ?? input.file_path;
    if (typeof path === "string") return truncateDisplay(path);
  }
  let serialized: string;
  try {
    serialized = JSON.stringify(event.input) ?? String(event.input);
  } catch {
    serialized = "[无法序列化的输入]";
  }
  return truncateDisplay(serialized);
}

/** Remove terminal controls and keep confirmation dialogs bounded. */
function truncateDisplay(value: string): string {
  const clean = sanitizeDisplay(value);
  return clean.length <= SUMMARY_MAX_CHARS
    ? clean
    : `${clean.slice(0, SUMMARY_MAX_CHARS - 1)}…`;
}

/** Remove control characters from diagnostics and confirmation text. */
function sanitizeDisplay(value: string): string {
  return value.replace(/[\u0000-\u001f\u007f-\u009f]/g, " ").trim();
}

/** Narrow JSON and tool inputs to ordinary non-array objects. */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Convert unknown thrown values into one-line diagnostics. */
function errorMessage(error: unknown): string {
  return sanitizeDisplay(
    error instanceof Error ? error.message : String(error),
  );
}

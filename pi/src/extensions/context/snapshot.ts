import { createHash } from "node:crypto";
import { resolve } from "node:path";
import { getCurrentSystemMessage, getCurrentTools, type Usage } from "@earendil-works/pi-ai";
import {
  convertToLlm,
  type BuildSystemPromptOptions,
  type ContextWithSystemEvent,
} from "@earendil-works/pi-coding-agent";
import { change, field, heading, note, type ReportRow } from "./report.js";

export interface ContextEntry {
  key: string;
  label: string;
  body: string;
  format: "text" | "json";
  /** UTF-8 size of raw text or compact JSON, before JSON is pretty-printed for display. */
  bytes: number;
  fingerprint: string;
}

/** Two request observations are retained in memory; session changes discard both. */
export interface ContextSnapshot {
  number: number;
  capturedAt: number;
  selectedModel: string;
  runtimePrompt: ContextEntry;
  prompt: ContextEntry[];
  tools: ContextEntry[];
  messages: ContextEntry[];
  sources: ReportRow[];
  response?: {
    model: string;
    responseModel?: string;
    stopReason: string;
    usage: Usage;
  };
}

/** Provider hooks expose no request identity; these observations are independent of context snapshots. */
export interface PayloadSnapshot {
  number: number;
  capturedAt: number;
  json: string;
  entries: ContextEntry[];
  bytes: number;
}

export function contextEntry(key: string, label: string, body: string): ContextEntry {
  return {
    key,
    label,
    body,
    format: "text",
    bytes: Buffer.byteLength(body, "utf8"),
    fingerprint: createHash("sha256").update(body).digest("hex"),
  };
}

function jsonEntry(key: string, label: string, value: unknown): ContextEntry {
  const json = JSON.stringify(value);
  return {
    ...contextEntry(key, label, json),
    body: JSON.stringify(value, null, 2),
    format: "json",
  };
}

export function captureContext(
  event: ContextWithSystemEvent,
  options: BuildSystemPromptOptions,
  number: number,
  selectedModel: string,
  runtimePrompt: string,
): ContextSnapshot {
  const system = getCurrentSystemMessage(event.messages);
  const prompt: ContextEntry[] = [];
  if (system) {
    const content =
      typeof system.content === "string" ? system.content : system.content.map((block) => block.text).join("\n");
    if (content) prompt.push(contextEntry("content", "Base and appended instructions", content));
    for (const [name, text] of Object.entries(system.sections ?? {})) {
      if (text !== null) prompt.push(contextEntry(`section:${name}`, name, text));
    }
  }

  const calls = new Map<string, { name: string; path?: string }>();
  for (const message of event.messages) {
    if (message.role !== "assistant") continue;
    for (const block of message.content) {
      if (block.type !== "toolCall") continue;
      calls.set(block.id, {
        name: block.name,
        ...(typeof block.arguments.path === "string" ? { path: block.arguments.path } : {}),
      });
    }
  }

  const messages: ContextEntry[] = [];
  const visibleReadPaths = new Set<string>();
  for (const original of event.messages) {
    const converted = convertToLlm([original]);
    for (const message of converted) {
      let label: string = original.role;
      if (message.role === "toolResult") {
        const call = calls.get(message.toolCallId);
        label = `${message.toolName}${call?.path ? ` · ${call.path}` : ""}${message.isError ? " · error" : ""}`;
        if (call?.name === "read" && call.path && !message.isError) {
          visibleReadPaths.add(resolve(options.cwd, call.path));
        }
      }
      // Response usage, timestamps and tool display details are not request content.
      const content =
        message.role === "system"
          ? {
              role: message.role,
              content: message.content,
              sections: message.sections,
              toolsAdded: message.toolsAdded,
              toolsRemoved: message.toolsRemoved,
            }
          : message.role === "toolResult"
            ? {
                role: message.role,
                toolName: message.toolName,
                toolCallId: message.toolCallId,
                content: message.content,
                isError: message.isError,
              }
            : { role: message.role, content: message.content };
      const entry = jsonEntry(String(messages.length + 1), `#${messages.length + 1} · ${label}`, content);
      messages.push(entry);
    }
  }

  const sources: ReportRow[] = [
    heading("Context files"),
    note("before_agent_start configuration; text lookup in the runtime prompt."),
    ...(options.contextFiles ?? []).map((file) =>
      field(
        file.path,
        file.content.length === 0
          ? "Empty configured text"
          : runtimePrompt.includes(file.content)
            ? "Text present in runtime prompt"
            : "Text not found in runtime prompt",
        undefined,
        file.content.length > 0 && runtimePrompt.includes(file.content) ? "success" : "warning",
      ),
    ),
    heading("Configured skills"),
    ...(options.skills ?? []).map((skill) => {
      const read = visibleReadPaths.has(resolve(options.cwd, skill.filePath));
      return field(
        skill.name,
        read ? "Read result retained · may be partial" : "No retained successful read matched",
        skill.filePath,
        read ? "success" : "warning",
      );
    }),
    heading("Evidence limits"),
    note("Skill evidence matches read(path) calls and their retained results only."),
    note("Other tools, omitted calls and compacted history cannot establish full skill loading here."),
  ];

  return {
    number,
    capturedAt: Date.now(),
    selectedModel,
    runtimePrompt: contextEntry("runtime-prompt", "Runtime prompt · session configuration", runtimePrompt),
    prompt,
    tools: getCurrentTools(event.messages).map((tool) => jsonEntry(tool.name, tool.name, tool)),
    messages,
    sources,
  };
}

export function capturePayload(payload: unknown, number: number): PayloadSnapshot {
  const serialized = JSON.stringify(payload);
  const fields: unknown = JSON.parse(serialized);
  const entries =
    fields !== null && typeof fields === "object" && !Array.isArray(fields)
      ? Object.entries(fields).map(([key, value]) => jsonEntry(key, key, value))
      : [jsonEntry("payload", "Provider payload", fields)];
  return {
    number,
    capturedAt: Date.now(),
    json: JSON.stringify(fields, null, 2),
    entries,
    bytes: Buffer.byteLength(serialized, "utf8"),
  };
}

export function formatBytes(bytes: number): string {
  return `${bytes.toLocaleString("en-US")} B`;
}

export function compareEntries(previous: ContextEntry[], current: ContextEntry[]): ReportRow[] {
  const before = new Map(previous.map((entry) => [entry.key, entry]));
  const after = new Map(current.map((entry) => [entry.key, entry]));
  return [
    ...current.flatMap((entry) => {
      const old = before.get(entry.key);
      if (!old) return [change("added", `${entry.label} · ${formatBytes(entry.bytes)}`)];
      if (old.fingerprint === entry.fingerprint) return [];
      return [change("changed", `${entry.label} · ${formatBytes(old.bytes)} → ${formatBytes(entry.bytes)}`)];
    }),
    ...previous.filter((entry) => !after.has(entry.key)).map((entry) => change("removed", entry.label)),
  ];
}

/** Match message content as a multiset so compaction does not mark every shifted index as edited. */
function unmatchedMessages(left: ContextEntry[], right: ContextEntry[]): ContextEntry[] {
  const remaining = new Map<string, number>();
  for (const entry of right) remaining.set(entry.fingerprint, (remaining.get(entry.fingerprint) ?? 0) + 1);
  return left.filter((entry) => {
    const count = remaining.get(entry.fingerprint) ?? 0;
    if (count === 0) return true;
    remaining.set(entry.fingerprint, count - 1);
    return false;
  });
}

export function changesReport(previous: ContextSnapshot | undefined, current: ContextSnapshot): ReportRow[] {
  if (!previous) return [note("No previous request captured in this session branch.")];
  const added = unmatchedMessages(current.messages, previous.messages);
  const removed = unmatchedMessages(previous.messages, current.messages);
  const section = (title: string, changes: ReportRow[]) => [
    heading(title),
    ...(changes.length ? changes : [note("Unchanged")]),
  ];
  return [
    heading(`Requests #${previous.number} → #${current.number}`),
    field(
      "Selected model",
      current.selectedModel,
      previous.selectedModel === current.selectedModel ? "unchanged" : `was ${previous.selectedModel}`,
    ),
    ...section("Runtime prompt", compareEntries([previous.runtimePrompt], [current.runtimePrompt])),
    ...section("Transcript prompt · before projections", compareEntries(previous.prompt, current.prompt)),
    ...section("Transcript tools · before hidden-tool filtering", compareEntries(previous.tools, current.tools)),
    heading("Transcript messages"),
    field("Added / removed", `+${added.length} / −${removed.length}`, undefined, "number"),
    ...added.map((entry) => change("added", `${entry.label} · ${formatBytes(entry.bytes)}`)),
    ...removed.map((entry) => change("removed", `${entry.label} · ${formatBytes(entry.bytes)}`)),
    note("Content matching ignores positions; edits appear as removal + addition."),
  ];
}

export function payloadReport(
  current: PayloadSnapshot | undefined,
  previous: PayloadSnapshot | undefined,
): ReportRow[] {
  if (!current) return [note("No provider payload observed.")];
  const changes = previous ? compareEntries(previous.entries, current.entries) : [];
  return [
    heading(`Provider observation #${current.number}`),
    field("Captured", new Date(current.capturedAt).toLocaleString()),
    field("Compact JSON", formatBytes(current.bytes), undefined, "number"),
    heading("Changes"),
    ...(previous
      ? [
          field("Compared with", `#${previous.number}`, new Date(previous.capturedAt).toLocaleString()),
          ...(changes.length ? changes : [note("Unchanged")]),
        ]
      : [note("No previous provider observation.")]),
    heading("Observation scope"),
    note("This hook includes main requests, retries and cache warming."),
    note("Pi supplies no request identity here. Payload observations are not attributed to a context snapshot."),
  ];
}

export function overviewReport(snapshot: ContextSnapshot, payload: PayloadSnapshot | undefined): ReportRow[] {
  const sum = (entries: ContextEntry[]) => entries.reduce((total, entry) => total + entry.bytes, 0);
  const usage = snapshot.response?.usage;
  return [
    heading(`Request #${snapshot.number}`),
    field("Captured", new Date(snapshot.capturedAt).toLocaleString()),
    field("Selected model", snapshot.selectedModel),
    heading("Context composition · UTF-8 bytes"),
    field("Runtime prompt", formatBytes(snapshot.runtimePrompt.bytes), "session configuration", "number"),
    field(
      "Transcript prompt",
      formatBytes(sum(snapshot.prompt)),
      `${snapshot.prompt.length} sections · summed`,
      "number",
    ),
    field("Tool declarations", formatBytes(sum(snapshot.tools)), `${snapshot.tools.length} tools · summed`, "number"),
    field("Messages", formatBytes(sum(snapshot.messages)), `${snapshot.messages.length} entries · summed`, "number"),
    field(
      "Provider payload",
      payload ? formatBytes(payload.bytes) : "not observed",
      payload ? `observation #${payload.number} · ${new Date(payload.capturedAt).toLocaleTimeString()}` : undefined,
      "number",
    ),
    heading("Assistant response · tokens"),
    ...(usage
      ? [
          field(
            "Status",
            snapshot.response!.stopReason,
            undefined,
            snapshot.response!.stopReason === "error"
              ? "error"
              : snapshot.response!.stopReason === "aborted"
                ? "warning"
                : "success",
          ),
          field("Request model", snapshot.response!.model),
          field("Response model", snapshot.response!.responseModel ?? "not supplied"),
          ...(usage.input + usage.output + usage.cacheRead + usage.cacheWrite > 0
            ? [
                field(
                  "Input / output",
                  `${usage.input.toLocaleString("en-US")} / ${usage.output.toLocaleString("en-US")}`,
                  undefined,
                  "number",
                ),
                field(
                  "Cache read / write",
                  `${usage.cacheRead.toLocaleString("en-US")} / ${usage.cacheWrite.toLocaleString("en-US")}`,
                  undefined,
                  "number",
                ),
                field("Total tokens", usage.totalTokens.toLocaleString("en-US"), undefined, "number"),
              ]
            : [note("Pi usage counters are all zero; no nonzero provider usage observed.")]),
        ]
      : [note("No completed assistant response captured for this request.")]),
    heading("Largest transcript messages"),
    ...[...snapshot.messages]
      .sort((a, b) => b.bytes - a.bytes)
      .slice(0, 10)
      .map((entry) => field(entry.label, formatBytes(entry.bytes), undefined, "number")),
    heading("Measurement"),
    note("UTF-8 bytes: raw prompt text, compact JSON for declarations/messages/payload."),
    note("Entry sums exclude separators. Prompt/tools also occur in system messages; totals overlap."),
    note("Bytes are not tokens. Images and opaque signatures remain in JSON sizes."),
    note("Token usage is normalized by Pi; no per-section attribution is available."),
    heading("Observation points"),
    field("Runtime prompt", "ctx.getSystemPrompt()", "includes configured forced-prompt overrides"),
    field(
      "Transcript",
      "context_with_system",
      "before forced-prompt projection, hidden-tool filtering and provider conversion",
    ),
    field("Provider payload", "before_provider_request", "independent observations; includes cache warming"),
    note("Later handlers and transport processing can still change the observed payload."),
    note("Snapshots are memory-only. Reload, session switch and tree navigation clear them."),
  ];
}

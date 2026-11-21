import { resolve } from "node:path";
import { getCurrentSystemMessage, getCurrentTools } from "@earendil-works/pi-ai";
import { convertToLlm } from "@earendil-works/pi-coding-agent";
import { numberCell, type ReportSection } from "./report.js";
import type { ContextSnapshot, PayloadSnapshot } from "./snapshot.js";
import type { SourceDocument } from "./source-layout.js";

export interface ContextEntry extends SourceDocument {
  label: string;
  /** UTF-8 size of raw text or compact JSON, before JSON is pretty-printed for display. */
  bytes: number;
}

interface ContextMessage extends ContextEntry {
  number: number;
  path?: string;
}

/** Entries and byte counts prepared once when an inspector opens. JSON bodies stay compact until selected. */
export interface ContextInspection {
  number: number;
  capturedAt: number;
  selectedModel: string;
  runtimePrompt: ContextEntry;
  prompt: ContextEntry[];
  tools: ContextEntry[];
  messages: ContextMessage[];
  sources: ReportSection[];
  response?: ContextSnapshot["response"];
}

export interface PayloadInspection extends PayloadSnapshot {
  entries: ContextEntry[];
  bytes: number;
}

function contextEntry(label: string, body: string): ContextEntry {
  return {
    label,
    body,
    format: "text",
    bytes: Buffer.byteLength(body, "utf8"),
  };
}

function jsonEntry(label: string, value: unknown): ContextEntry {
  const json = JSON.stringify(value);
  return {
    ...contextEntry(label, json),
    format: "json",
  };
}

export function prepareContext(snapshot: ContextSnapshot): ContextInspection {
  const { options, runtimePrompt } = snapshot;
  const system = getCurrentSystemMessage(snapshot.messages);
  const prompt: ContextEntry[] = [];
  if (system) {
    const content =
      typeof system.content === "string" ? system.content : system.content.map((block) => block.text).join("\n");
    if (content) prompt.push(contextEntry("Base and appended instructions", content));
    for (const [name, text] of Object.entries(system.sections ?? {})) {
      if (text !== null) prompt.push(contextEntry(name, text));
    }
  }

  const calls = new Map<string, { name: string; path?: string }>();
  for (const message of snapshot.messages) {
    if (message.role !== "assistant") continue;
    for (const block of message.content) {
      if (block.type !== "toolCall") continue;
      calls.set(block.id, {
        name: block.name,
        ...(typeof block.arguments.path === "string" ? { path: block.arguments.path } : {}),
      });
    }
  }

  const messages: ContextMessage[] = [];
  const visibleReadPaths = new Set<string>();
  for (const original of snapshot.messages) {
    const converted = convertToLlm([original]);
    for (const message of converted) {
      let label: string = original.role;
      let path: string | undefined;
      if (message.role === "toolResult") {
        const call = calls.get(message.toolCallId);
        label = message.isError ? `${message.toolName} (error)` : message.toolName;
        path = call?.path;
        if (call?.name === "read" && path && !message.isError) {
          visibleReadPaths.add(resolve(options.cwd, path));
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
      messages.push({ ...jsonEntry(label, content), number: messages.length + 1, path });
    }
  }

  const sources: ReportSection[] = [
    {
      title: "Context files",
      columns: [{ heading: "File", weight: 1 }],
      rows: (options.contextFilePaths ?? []).map((path) => [path]),
      note: options.contextFilePaths?.length ? undefined : "No context files configured.",
    },
    {
      title: "Configured skills",
      columns: [
        { heading: "Skill", weight: 2 },
        { heading: "File", weight: 4 },
        { heading: "Read", weight: 3 },
      ],
      rows: (options.skills ?? []).map((skill) => {
        const read = visibleReadPaths.has(resolve(options.cwd, skill.filePath));
        return [skill.name, skill.filePath, { text: read ? "Read" : "Unread", color: read ? "success" : "muted" }];
      }),
      note: options.skills?.length
        ? "Only successful read calls and their results still in the request count. A read may cover only part of a file."
        : "No skills configured.",
    },
  ];

  return {
    number: snapshot.number,
    capturedAt: snapshot.capturedAt,
    selectedModel: snapshot.selectedModel,
    runtimePrompt: contextEntry("Runtime prompt", runtimePrompt),
    prompt,
    tools: getCurrentTools(snapshot.messages).map((tool) => jsonEntry(tool.name, tool)),
    messages,
    sources,
    response: snapshot.response,
  };
}

export function preparePayload(snapshot: PayloadSnapshot): PayloadInspection {
  const fields: unknown = JSON.parse(snapshot.json);
  const entries =
    fields !== null && typeof fields === "object" && !Array.isArray(fields)
      ? Object.entries(fields).map(([key, value]) => jsonEntry(key, value))
      : [jsonEntry("Provider payload", fields)];
  return {
    ...snapshot,
    entries,
    bytes: Buffer.byteLength(snapshot.json, "utf8"),
  };
}

export function formatBytes(bytes: number): string {
  return `${bytes.toLocaleString("en-US")} B`;
}

const propertyColumns: ReportSection["columns"] = [
  { heading: "Field", weight: 2 },
  { heading: "Value", weight: 3 },
];

export function overviewReport(snapshot: ContextInspection): ReportSection[] {
  const sum = (entries: ContextEntry[]) => entries.reduce((total, entry) => total + entry.bytes, 0);
  const response = snapshot.response;
  return [
    {
      title: "Request",
      columns: propertyColumns,
      rows: [
        ["Captured", new Date(snapshot.capturedAt).toLocaleString()],
        ["Selected model", snapshot.selectedModel],
      ],
    },
    {
      title: "Context composition",
      columns: [
        { heading: "Component", weight: 3 },
        { heading: "Entries", weight: 2, align: "right" },
        { heading: "Size", weight: 2, align: "right" },
      ],
      rows: [
        ["Runtime prompt", "—", numberCell(formatBytes(snapshot.runtimePrompt.bytes))],
        [
          "Transcript prompt",
          numberCell(String(snapshot.prompt.length)),
          numberCell(formatBytes(sum(snapshot.prompt))),
        ],
        ["Tool declarations", numberCell(String(snapshot.tools.length)), numberCell(formatBytes(sum(snapshot.tools)))],
        ["Messages", numberCell(String(snapshot.messages.length)), numberCell(formatBytes(sum(snapshot.messages)))],
      ],
    },
    {
      title: "Assistant response",
      columns: propertyColumns,
      rows: response
        ? [
            [
              "Status",
              {
                text: response.stopReason,
                color:
                  response.stopReason === "error" ? "error" : response.stopReason === "aborted" ? "warning" : "success",
              },
            ],
            ["Request model", response.model],
            ["Response model", response.responseModel ?? "Not supplied"],
          ]
        : [],
      note: response ? undefined : "No completed assistant response captured.",
    },
    ...snapshot.sources,
    {
      title: "Largest transcript messages",
      columns: [
        { heading: "#", weight: 1, align: "right" },
        { heading: "Name", weight: 3 },
        { heading: "Path", weight: 5 },
        { heading: "Size", weight: 2, align: "right" },
      ],
      rows: [...snapshot.messages]
        .sort((a, b) => b.bytes - a.bytes)
        .slice(0, 10)
        .map((entry) => [
          numberCell(`#${entry.number}`),
          entry.label,
          entry.path ?? "",
          numberCell(formatBytes(entry.bytes)),
        ]),
      note: snapshot.messages.length ? undefined : "No messages captured.",
    },
  ];
}

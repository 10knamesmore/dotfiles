import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { contentText, getCurrentSystemMessage } from "@earendil-works/pi-ai";
import { convertToLlm } from "@earendil-works/pi-coding-agent";
import type { ContextSnapshot } from "./snapshot.js";
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
  runtimePrompt: ContextEntry;
  promptSections: ContextEntry[];
  tools: ContextEntry[];
  messages: ContextMessage[];
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

function expandHome(path: string): string {
  if (path === "~") return homedir();
  return path.startsWith("~/") ? join(homedir(), path.slice(2)) : path;
}

const home = resolve(homedir());
const homePrefix = join(home, "/");

function displayPath(path: string): string {
  if (path === home) return "~/";
  return path.startsWith(homePrefix) ? `~/${path.slice(homePrefix.length)}` : path;
}

export function prepareContext(snapshot: ContextSnapshot): ContextInspection {
  const system = getCurrentSystemMessage(snapshot.messages);
  const promptSections: ContextEntry[] = [];
  if (system) {
    const instructions = contentText(system.content);
    if (instructions) promptSections.push(contextEntry("Base and appended instructions", instructions));
    for (const [name, text] of Object.entries(system.sections ?? {})) {
      if (text !== null) promptSections.push(contextEntry(name, text));
    }
  }

  const toolPaths = new Map<string, string>();
  for (const message of snapshot.messages) {
    if (message.role !== "assistant") continue;
    for (const block of message.content) {
      if (block.type === "toolCall" && typeof block.arguments.path === "string") {
        toolPaths.set(block.id, displayPath(expandHome(block.arguments.path)));
      }
    }
  }

  const messages: ContextMessage[] = [];
  for (const original of snapshot.messages) {
    const converted = convertToLlm([original]);
    for (const message of converted) {
      let label: string = original.role;
      let path: string | undefined;
      if (message.role === "toolResult") {
        label = message.isError ? `${message.toolName} (error)` : message.toolName;
        path = toolPaths.get(message.toolCallId);
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

  return {
    runtimePrompt: contextEntry("Full system prompt", snapshot.runtimePrompt),
    promptSections,
    tools: (system?.toolsAdded ?? []).map((tool) => jsonEntry(tool.name, tool)),
    messages,
  };
}

const sizeFormat = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

export function formatBytes(bytes: number): string {
  if (bytes >= 1_000_000) return `${sizeFormat.format(bytes / 1_000_000)} MB`;
  if (bytes >= 1_000) return `${sizeFormat.format(bytes / 1_000)} KB`;
  return `${bytes} B`;
}

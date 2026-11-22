import { truncateHead } from "@earendil-works/pi-coding-agent";
import type { UsageSummary } from "./types.js";

export const DELIVERY_MESSAGE_TYPE = "subagent-workflow:delivery";

/** Result sections for humans, separate from the model-facing delivery envelope. */
export interface DeliveryRunResult {
  kind: "subagent" | "workflow" | "run";
  runId?: string;
  label: string;
  status: "completed" | "completed-with-failures" | "failed" | "aborted";
  body?: {
    format: "markdown" | "json";
    text: string;
    truncated: boolean;
  };
  failures?: readonly string[];
  recovery?: readonly string[];
  warnings?: readonly string[];
  model?: string;
  thinkingLevel?: string;
  usage?: UsageSummary;
  durationMs?: number;
  activity?: {
    tools: Readonly<Record<string, number>>;
    complete: boolean;
  };
  artifacts?: readonly { label: string; path: string }[];
}

/** Persisted display data also identifies the message consumed by the parent. */
export interface DeliveryMessageDetails {
  deliveryId: string;
  catchUp: boolean;
  runs: readonly DeliveryRunResult[];
}

/** Bound transcript storage; callers include the full result's location in artifacts. */
export function createDeliveryBody(
  value: unknown,
  format: "markdown" | "json",
): NonNullable<DeliveryRunResult["body"]> {
  const text = format === "markdown" ? String(value) : JSON.stringify(value, null, 2);
  const preview = truncateHead(text, { maxBytes: 16_000, maxLines: 500 });
  return { format, text: preview.content, truncated: preview.truncated };
}

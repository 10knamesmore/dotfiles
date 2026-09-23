import type { AgentToolResult, ToolDefinition } from "@earendil-works/pi-coding-agent";
import { Type, type TSchema } from "typebox";

export interface SchemaCapture {
  called: boolean;
  value?: unknown;
  terminate?: () => void;
}

export function createReportResultTool(schema: Record<string, unknown>, capture: SchemaCapture): ToolDefinition {
  return {
    name: "report_result",
    label: "Report Result",
    description: "Call this exactly once with the final structured answer. Do not continue after it succeeds.",
    parameters: Type.Unsafe(schema) as TSchema,
    async execute(_id, params): Promise<AgentToolResult<unknown>> {
      if (capture.called) throw new Error("report_result may only be called once");
      capture.called = true;
      capture.value = params;
      capture.terminate?.();
      return { content: [{ type: "text", text: "Structured result accepted." }], details: undefined };
    },
  };
}

export const STRUCTURED_REPAIR_PROMPT =
  "You did not call report_result. Call report_result exactly once now with your final answer matching its schema.";

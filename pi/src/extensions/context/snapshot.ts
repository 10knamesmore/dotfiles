import type { Usage } from "@earendil-works/pi-ai";
import type { BuildSystemPromptOptions, ContextWithSystemEvent } from "@earendil-works/pi-coding-agent";

/** Owned request data captured before later handlers can mutate it; replaced on each observation. */
export interface ContextSnapshot {
  readonly number: number;
  readonly capturedAt: number;
  readonly selectedModel: string;
  readonly runtimePrompt: string;
  readonly messages: ContextWithSystemEvent["messages"];
  readonly options: Pick<BuildSystemPromptOptions, "cwd" | "contextFiles" | "skills">;
  readonly response?: {
    model: string;
    responseModel?: string;
    stopReason: string;
    usage: Usage;
  };
}

/** Provider observations have their own timeline. One compact JSON string owns the captured payload. */
export interface PayloadSnapshot {
  readonly number: number;
  readonly capturedAt: number;
  readonly json: string;
}

export function captureContext(
  event: ContextWithSystemEvent,
  options: ContextSnapshot["options"],
  number: number,
  selectedModel: string,
  runtimePrompt: string,
): ContextSnapshot {
  return {
    number,
    capturedAt: Date.now(),
    selectedModel,
    runtimePrompt,
    ...structuredClone({ messages: event.messages, options }),
  };
}

export function capturePayload(payload: unknown, number: number): PayloadSnapshot {
  return {
    number,
    capturedAt: Date.now(),
    json: JSON.stringify(payload),
  };
}

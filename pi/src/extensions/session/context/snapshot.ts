import type { ContextWithSystemEvent } from "@earendil-works/pi-coding-agent";

/** Owned request data captured before later handlers can mutate it; replaced on each observation. */
export interface ContextSnapshot {
  readonly runtimePrompt: string;
  readonly messages: ContextWithSystemEvent["messages"];
}

/** Provider observations have their own timeline. One compact JSON string owns the captured payload. */
export interface PayloadSnapshot {
  readonly number: number;
  readonly capturedAt: number;
  readonly json: string;
}

export function captureContext(event: ContextWithSystemEvent, runtimePrompt: string): ContextSnapshot {
  return {
    runtimePrompt,
    messages: structuredClone(event.messages),
  };
}

export function capturePayload(payload: unknown, number: number): PayloadSnapshot {
  return {
    number,
    capturedAt: Date.now(),
    json: JSON.stringify(payload),
  };
}

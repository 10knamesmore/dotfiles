import { safeDeliveryValue } from "../ui/delivery-safe.js";
import type { RunSnapshot } from "./run-snapshot.js";

export interface ChildActivityFold {
  label: string;
  tools: Record<string, number>;
}

/** Incremental per-run activity state, keyed by the runner's persisted child id. */
export interface RunActivityFold {
  children: Map<string, ChildActivityFold>;
  complete: boolean;
}

export type ActivityFoldEvent =
  | { type: "child"; id: string; label: string }
  | { type: "activity"; id: string; description: string };

export function createActivityFold(complete = true): RunActivityFold {
  return { children: new Map(), complete };
}

/** Fold one child registration or activity event into a live or batch projection. */
export function foldActivity(state: RunActivityFold, event: ActivityFoldEvent): void {
  if (event.type === "child") {
    const current = state.children.get(event.id);
    if (current) current.label = event.label;
    else state.children.set(event.id, { label: event.label, tools: {} });
    return;
  }
  const child = state.children.get(event.id);
  if (!child) return;
  const tool = safeDeliveryValue(event.description.split(" ", 1)[0] || "unknown");
  child.tools[tool] = (child.tools[tool] ?? 0) + 1;
}

export function activityFoldFromSnapshot(snapshot: RunSnapshot): RunActivityFold {
  const fold = createActivityFold(snapshot.diagnostics.length === 0);
  const record = snapshot.record as {
    children?: Array<{ id?: string; spec?: { label?: string }; resolved?: { label?: string } }>;
  } | undefined;
  for (const child of record?.children ?? []) {
    if (typeof child.id !== "string") continue;
    foldActivity(fold, { type: "child", id: child.id, label: child.resolved?.label ?? child.spec?.label ?? child.id });
  }
  for (const value of snapshot.events) {
    const event = value as { type?: unknown; id?: unknown; description?: unknown };
    if (event.type !== "activity" || typeof event.id !== "string" || typeof event.description !== "string") continue;
    foldActivity(fold, { type: "activity", id: event.id, description: event.description });
  }
  return fold;
}

export function cloneActivityFold(state: RunActivityFold): RunActivityFold {
  return {
    complete: state.complete,
    children: new Map([...state.children].map(([id, child]) => [id, { label: child.label, tools: { ...child.tools } }])),
  };
}

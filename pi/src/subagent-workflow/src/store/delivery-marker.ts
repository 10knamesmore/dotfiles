import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage, isRecord } from "../util.js";
import { replaceAtomicFile } from "./atomic-file.js";
import { acquireRunOwnership, runOwnerIsLive, RunOwnershipConflictError, type RunOwnership } from "./lease.js";

export const DELIVERED_FILE = "delivered.json";
export const DELIVERY_PROTOCOL_VERSION = 1 as const;

export interface RunDeliveryIdentity {
  protocol: typeof DELIVERY_PROTOCOL_VERSION;
  generation: number;
}

export interface DeliveryMarker {
  v: 1;
  sessionId: string;
  catchUp: boolean;
  generation: number;
}

export interface DeliveryTarget {
  runDir: string;
  identity: RunDeliveryIdentity;
}

export interface ClaimedDeliveryTarget extends DeliveryTarget {
  ownership: RunOwnership;
}

interface PendingDelivery {
  sessionId: string;
  message: string;
  catchUp: boolean;
  targets: DeliveryTarget[];
}

const pendingDeliveries = new Map<string, PendingDelivery[]>();
const closedSessions = new Set<string>();
/** Acknowledged publications whose run was temporarily owned by another process. */
const deferredPublications: Array<{ target: DeliveryTarget; sessionId: string; catchUp: boolean }> = [];

export function parseRunDeliveryIdentity(record: unknown): RunDeliveryIdentity | undefined {
  if (!isRecord(record) || record.v !== 3 || !isRecord(record.delivery)) return undefined;
  const { protocol, generation } = record.delivery;
  return protocol === DELIVERY_PROTOCOL_VERSION && Number.isSafeInteger(generation) && (generation as number) >= 0
    ? { protocol, generation: generation as number }
    : undefined;
}

export function readRunDeliveryIdentity(runDir: string): RunDeliveryIdentity | undefined {
  const record: unknown = JSON.parse(readFileSync(join(runDir, "run.json"), "utf8"));
  return parseRunDeliveryIdentity(record);
}

export function readDeliveryMarker(runDir: string): DeliveryMarker | undefined {
  try {
    const value: unknown = JSON.parse(readFileSync(join(runDir, DELIVERED_FILE), "utf8"));
    if (!isRecord(value)
      || value.v !== 1
      || typeof value.sessionId !== "string"
      || typeof value.catchUp !== "boolean"
      || !Number.isSafeInteger(value.generation)
      || (value.generation as number) < 0) return undefined;
    return value as unknown as DeliveryMarker;
  } catch {
    return undefined;
  }
}

export function deliveryMarkerMatches(runDir: string, identity: RunDeliveryIdentity): boolean {
  return readDeliveryMarker(runDir)?.generation === identity.generation;
}

/**
 * Acquire the run's process-death-safe ownership lock for delivery work.
 * "conflict" means another live process owns the run right now; undefined
 * means the run's identity no longer matches (the claim is stale for good).
 */
export function claimRunDelivery(
  runDir: string,
  expected?: RunDeliveryIdentity,
): ClaimedDeliveryTarget | "conflict" | undefined {
  let ownership: RunOwnership;
  try {
    ownership = acquireRunOwnership(runDir);
  } catch (error) {
    if (error instanceof RunOwnershipConflictError) return "conflict";
    throw error;
  }
  try {
    const identity = readRunDeliveryIdentity(runDir);
    if (!identity || (expected && !sameIdentity(identity, expected))) {
      ownership.release();
      return undefined;
    }
    return { runDir, identity, ownership };
  } catch (error) {
    ownership.release();
    throw error;
  }
}

/** Publish only while ownership is held for the claimed generation. */
export function publishClaimedDelivery(claim: ClaimedDeliveryTarget, sessionId: string, catchUp: boolean): boolean {
  const existing = readDeliveryMarker(claim.runDir);
  if (existing?.generation === claim.identity.generation) return true;
  if (existing && existing.generation > claim.identity.generation) return false;

  replaceAtomicFile(join(claim.runDir, DELIVERED_FILE), markerText({
    v: 1,
    sessionId,
    catchUp,
    generation: claim.identity.generation,
  }), {
    mode: 0o600,
    fsync: true,
    syncParentDirectory: true,
  });
  return true;
}

/** Inline tool results are already accepted by the current tool call. */
export function writeDeliveryMarker(runDir: string, sessionId: string, identity: RunDeliveryIdentity): boolean {
  const claim = claimRunDelivery(runDir, identity);
  if (claim === "conflict" || !claim) return false;
  try {
    return publishClaimedDelivery(claim, sessionId, false);
  } finally {
    claim.ownership.release();
  }
}

/**
 * Publish one acknowledged target, deferring on a temporary ownership
 * conflict so a resume that later fails cannot cause a duplicate redelivery.
 * The deferral is in-memory only: if this process exits first, the worst
 * outcome is one repeated catch-up notice in a later session.
 */
function publishAcknowledgedTarget(target: DeliveryTarget, sessionId: string, catchUp: boolean): void {
  let claim: ClaimedDeliveryTarget | "conflict" | undefined;
  try {
    claim = claimRunDelivery(target.runDir, target.identity);
    if (claim === "conflict") {
      deferPublication(target, sessionId, catchUp);
      return;
    }
    if (!claim) return;
    publishClaimedDelivery(claim, sessionId, catchUp);
  } catch (error) {
    // A transient failure (ENOSPC, damaged lock database) must not drop an
    // acknowledged delivery: without a marker the next catch-up would
    // redeliver a message the model already consumed.
    deferPublication(target, sessionId, catchUp);
    reportDiagnostic(`[subagent-workflow] delivery acknowledgement failed for ${target.runDir}: ${errorMessage(error)}`);
  } finally {
    if (claim !== "conflict") claim?.ownership.release();
  }
}

function deferPublication(target: DeliveryTarget, sessionId: string, catchUp: boolean): void {
  const duplicate = deferredPublications.some((deferred) =>
    deferred.target.runDir === target.runDir
    && deferred.target.identity.generation === target.identity.generation);
  if (!duplicate) deferredPublications.push({ target, sessionId, catchUp });
}

/** Retry acknowledged publications that hit a temporary ownership conflict. */
export function retryDeferredPublications(): void {
  for (const deferred of deferredPublications.splice(0)) {
    try {
      // A cheap liveness probe keeps still-owned entries parked without paying
      // the lock-acquisition wait on every user message.
      if (runOwnerIsLive(deferred.target.runDir)) {
        deferPublication(deferred.target, deferred.sessionId, deferred.catchUp);
        continue;
      }
      publishAcknowledgedTarget(deferred.target, deferred.sessionId, deferred.catchUp);
    } catch (error) {
      // publishAcknowledgedTarget re-parks its own failures; this guards the
      // probe so one damaged run cannot drop the rest of the batch.
      deferPublication(deferred.target, deferred.sessionId, deferred.catchUp);
      reportDiagnostic(`[subagent-workflow] deferred delivery retry failed for ${deferred.target.runDir}: ${errorMessage(error)}`);
    }
  }
}

/** Queue a background message and wait for its matching user message_start before publication. */
export function queueAcknowledgedDelivery(
  pi: Pick<ExtensionAPI, "sendUserMessage">,
  request: {
    sessionId: string;
    message: string;
    targets: readonly DeliveryTarget[];
    catchUp?: boolean;
  },
): void {
  if (closedSessions.has(request.sessionId)) return;
  const pending: PendingDelivery = {
    sessionId: request.sessionId,
    message: request.message,
    catchUp: request.catchUp ?? false,
    targets: [...request.targets],
  };

  const sessionPending = pendingDeliveries.get(request.sessionId) ?? [];
  sessionPending.push(pending);
  pendingDeliveries.set(request.sessionId, sessionPending);
  try {
    pi.sendUserMessage(request.message, { deliverAs: "steer" });
  } catch (error) {
    removePending(pending);
    throw error;
  }
}

/** Accept exactly one queued delivery whose text appears in the consumed user message. */
export function acknowledgeDeliveryMessage(sessionId: string, message: string): boolean {
  retryDeferredPublications();
  const sessionPending = pendingDeliveries.get(sessionId);
  if (!sessionPending) return false;
  const index = sessionPending.findIndex((pending) => message.includes(pending.message));
  if (index < 0) return false;
  const pending = sessionPending.splice(index, 1)[0];
  if (!pending) return false;
  if (sessionPending.length === 0) pendingDeliveries.delete(sessionId);

  for (const target of pending.targets) {
    publishAcknowledgedTarget(target, sessionId, pending.catchUp);
  }
  return true;
}

/** Drop queued deliveries without publishing them. */
export function releasePendingDeliveries(sessionId: string): void {
  pendingDeliveries.delete(sessionId);
}

export function markSessionClosed(sessionId: string): void {
  closedSessions.add(sessionId);
  releasePendingDeliveries(sessionId);
}

export function markSessionOpen(sessionId: string): void {
  closedSessions.delete(sessionId);
}

function sameIdentity(left: RunDeliveryIdentity, right: RunDeliveryIdentity): boolean {
  return left.protocol === right.protocol && left.generation === right.generation;
}

function removePending(pending: PendingDelivery): void {
  const sessionPending = pendingDeliveries.get(pending.sessionId);
  if (!sessionPending) return;
  const index = sessionPending.indexOf(pending);
  if (index >= 0) sessionPending.splice(index, 1);
  if (sessionPending.length === 0) pendingDeliveries.delete(pending.sessionId);
}

function markerText(marker: DeliveryMarker): string {
  return `${JSON.stringify(marker)}\n`;
}

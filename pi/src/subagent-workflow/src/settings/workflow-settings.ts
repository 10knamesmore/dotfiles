/**
 * User-global workflow settings.
 *
 * The file is intentionally strict and versioned. A malformed or partially
 * understood file never changes runtime behavior: the store reports the
 * problem and exposes safe defaults until the user fixes or resets it.
 */

import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { replaceAtomicFile } from "../store/atomic-file.js";
import { SqliteLockBusyError, withSqliteMutex } from "../store/sqlite-lock.js";
import { reportDiagnostic } from "../diagnostics.js";
import { errorMessage, isRecord } from "../util.js";

export type ConcurrentAgentLimit = "auto" | number;
export type WorkflowApprovalMode = "always-prompt" | "remember" | "auto";
export type NumericSettingId = "maxConcurrentAgents" | "agentTimeoutMinutes";

export interface NumericSettingRule {
  readonly min: number;
  readonly max: number;
  readonly sentinel: 0 | "auto";
}

export const NUMERIC_SETTING_RULES = Object.freeze({
  maxConcurrentAgents: Object.freeze({ min: 1, max: 64, sentinel: "auto" }),
  agentTimeoutMinutes: Object.freeze({ min: 1, max: 240, sentinel: 0 }),
} as const) satisfies Readonly<Record<NumericSettingId, NumericSettingRule>>;

export interface WorkflowSettings {
  maxConcurrentAgents: ConcurrentAgentLimit;
  workflowApproval: WorkflowApprovalMode;
  agentTimeoutMinutes: number;
  showStatusWidget: boolean;
}

export const DEFAULT_WORKFLOW_SETTINGS: Readonly<WorkflowSettings> = Object.freeze({
  maxConcurrentAgents: "auto",
  workflowApproval: "remember",
  agentTimeoutMinutes: 0,
  showStatusWidget: true,
});

const SETTINGS_VERSION = 4;
const SETTING_KEYS = [
  "agentTimeoutMinutes",
  "maxConcurrentAgents",
  "showStatusWidget",
  "version",
  "workflowApproval",
] as const;
/**
 * Keys of the strictly-valid older schemas this store migrates in place.
 * v3 carried childExtensionExclusions (an in-process compensation deleted
 * with the subprocess-child cutover); v2 additionally carried three knobs
 * removed earlier. Migration drops the dead keys and keeps the live ones.
 */
const V3_SETTING_KEYS = Object.freeze([
  "agentTimeoutMinutes",
  "childExtensionExclusions",
  "maxConcurrentAgents",
  "showStatusWidget",
  "workflowApproval",
] as const);
const V2_SETTING_KEYS = Object.freeze([
  "agentTimeoutMinutes",
  "childExtensionExclusions",
  "maxAgentsPerWorkflow",
  "maxConcurrentAgents",
  "showStatusWidget",
  "statusWidgetMaxRows",
  "warmSessionLimit",
  "workflowApproval",
] as const);
const SETTINGS_LOCK_WAIT_MS = 250;

class SettingsVersionError extends Error {
  constructor(message: string, readonly fileVersion: string) {
    super(message);
    this.name = "SettingsVersionError";
  }
}

interface WorkflowSettingsStoreOptions {
  path?: string;
  warn?: (message: string) => void;
}

/** In-process settings authority. Mutations are synchronous and atomic on disk. */
export class WorkflowSettingsStore {
  readonly path: string;
  private settings: WorkflowSettings = copyDefaults();
  private warning: string | undefined;
  private readonly warn: (message: string) => void;
  private readonly listeners = new Set<(settings: Readonly<WorkflowSettings>) => void>();

  constructor(options: WorkflowSettingsStoreOptions = {}) {
    this.path = options.path ?? join(getAgentDir(), "subagent-workflow", "settings.json");
    this.warn = options.warn ?? ((message) => reportDiagnostic(`[subagent-workflow] ${message}`));
    this.load();
  }

  get(): Readonly<WorkflowSettings> {
    return { ...this.settings };
  }

  getWarning(): string | undefined {
    return this.warning;
  }

  set<K extends keyof WorkflowSettings>(key: K, value: WorkflowSettings[K]): Readonly<WorkflowSettings> {
    return this.mutate((latest) => ({ ...latest, [key]: value }));
  }

  reset(): Readonly<WorkflowSettings> {
    return this.mutate(() => copyDefaults(), false);
  }

  /** Re-read a manually edited file and notify runtime consumers immediately. */
  reload(): Readonly<WorkflowSettings> {
    this.load();
    return this.get();
  }

  subscribe(listener: (settings: Readonly<WorkflowSettings>) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private mutate(
    transform: (latest: WorkflowSettings) => WorkflowSettings,
    rebaseFromDisk = true,
  ): Readonly<WorkflowSettings> {
    let committed: WorkflowSettings;
    try {
      committed = withSettingsFileLock(this.path, () => {
        const latest = rebaseFromDisk && existsSync(this.path)
          ? parseSettingsForMutation(readFileSync(this.path, "utf8"))
          : copyDefaults();
        const next = transform(latest);
        validateSettings(next);
        writeSettingsFile(this.path, next);
        return { ...next };
      });
    } catch (error) {
      throw new Error(`Cannot save workflow settings at ${this.path}: ${errorMessage(error)}`);
    }
    this.settings = committed;
    this.warning = undefined;
    this.emit();
    return this.get();
  }

  private load(): void {
    let next = copyDefaults();
    this.warning = undefined;
    if (existsSync(this.path)) {
      try {
        next = parseSettings(readFileSync(this.path, "utf8"));
      } catch (error) {
        if (error instanceof SettingsVersionError) {
          try {
            const migration = withSettingsFileLock(this.path, () => migrateLegacySettingsFile(this.path));
            next = migration.settings;
            if (migration.migratedFrom !== undefined) {
              this.reportWarning(`Workflow settings at ${this.path} were migrated from v${migration.migratedFrom} to v${SETTINGS_VERSION}.`);
            }
          } catch {
            this.warning = `Workflow settings at ${this.path} were reset to defaults because the schema changed to v${SETTINGS_VERSION} (found file version ${error.fileVersion}): ${error.message}. The file was left unchanged.`;
            this.reportWarning(this.warning);
          }
        } else {
          this.warning = `Cannot load workflow settings at ${this.path}: ${errorMessage(error)}. Using safe defaults.`;
          this.reportWarning(this.warning);
        }
      }
    }
    this.settings = next;
    this.emit();
  }

  private emit(): void {
    for (const listener of this.listeners) {
      try {
        listener({ ...this.settings });
      } catch (error) {
        this.reportWarning(`Workflow settings listener failed: ${errorMessage(error)}`);
      }
    }
  }

  private reportWarning(warning: string): void {
    try {
      this.warn(warning);
    } catch {
      // Diagnostics must never make safe-default loading or runtime updates fail.
    }
  }
}

export function parseSettings(text: string): WorkflowSettings {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (error) {
    throw new Error(`invalid JSON (${errorMessage(error)})`);
  }
  if (!isRecord(value)) throw new Error("settings must be a JSON object");
  if (value.version !== SETTINGS_VERSION) {
    const fileVersion = "version" in value ? JSON.stringify(value.version) ?? String(value.version) : "missing";
    throw new SettingsVersionError(`unsupported settings version ${String(value.version)} (expected ${SETTINGS_VERSION})`, fileVersion);
  }
  const keys = Object.keys(value).sort();
  if (keys.length !== SETTING_KEYS.length || keys.some((key, index) => key !== SETTING_KEYS[index])) {
    throw new Error(`settings must contain exactly: ${SETTING_KEYS.join(", ")}`);
  }
  const settings = liveFieldsFrom(value);
  validateSettings(settings);
  return settings;
}

function liveFieldsFrom(value: Record<string, unknown>): WorkflowSettings {
  return {
    maxConcurrentAgents: value.maxConcurrentAgents as ConcurrentAgentLimit,
    workflowApproval: value.workflowApproval as WorkflowApprovalMode,
    agentTimeoutMinutes: value.agentTimeoutMinutes as number,
    showStatusWidget: value.showStatusWidget as boolean,
  };
}

function validateSettings(settings: WorkflowSettings): void {
  const concurrentRule = NUMERIC_SETTING_RULES.maxConcurrentAgents;
  const concurrent = settings.maxConcurrentAgents;
  if (concurrent !== concurrentRule.sentinel && !integerInRange(concurrent, concurrentRule.min, concurrentRule.max)) {
    throw new Error(`maxConcurrentAgents must be \"auto\" or an integer from ${concurrentRule.min} to ${concurrentRule.max}`);
  }
  if (!["always-prompt", "remember", "auto"].includes(settings.workflowApproval)) {
    throw new Error("workflowApproval must be \"always-prompt\", \"remember\", or \"auto\"");
  }
  const timeoutRule = NUMERIC_SETTING_RULES.agentTimeoutMinutes;
  if (settings.agentTimeoutMinutes !== timeoutRule.sentinel
    && !integerInRange(settings.agentTimeoutMinutes, timeoutRule.min, timeoutRule.max)) {
    throw new Error(`agentTimeoutMinutes must be an integer from ${timeoutRule.min} to ${timeoutRule.max}`);
  }
  if (typeof settings.showStatusWidget !== "boolean") throw new Error("showStatusWidget must be a boolean");
}

function writeSettingsFile(path: string, settings: WorkflowSettings): void {
  const directory = dirname(path);
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  replaceAtomicFile(path, `${JSON.stringify({ version: SETTINGS_VERSION, ...settings }, null, 2)}\n`, {
    mode: 0o600,
    fsync: true,
    exactMode: true,
    syncParentDirectory: true,
  });
}

function migrateLegacySettingsFile(path: string): { settings: WorkflowSettings; migratedFrom?: 2 | 3 } {
  const text = readFileSync(path, "utf8");
  try {
    return { settings: parseSettings(text) };
  } catch (error) {
    if (!(error instanceof SettingsVersionError)) throw error;
  }
  const { settings, from } = parseLegacySettings(text);
  writeSettingsFile(path, settings);
  return { settings, migratedFrom: from };
}

function parseLegacySettings(text: string): { settings: WorkflowSettings; from: 2 | 3 } {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (error) {
    throw new Error(`invalid JSON (${errorMessage(error)})`);
  }
  if (!isRecord(value) || (value.version !== 2 && value.version !== 3)) throw new Error("settings are not version 2 or 3");
  const from = value.version as 2 | 3;
  const expected = from === 2 ? V2_SETTING_KEYS : V3_SETTING_KEYS;
  const keys = Object.keys(value).filter((key) => key !== "version").sort();
  if (keys.length !== expected.length || keys.some((key, index) => key !== expected[index])) {
    throw new Error(`version ${from} settings must contain exactly: ${expected.join(", ")}`);
  }
  // Migration only accepts STRICTLY valid legacy files, deleted keys
  // included; anything else must fail closed and leave the file untouched
  // rather than be rewritten as valid v4. Historical rule for the one
  // structured deleted key:
  const exclusions = value.childExtensionExclusions;
  if (!Array.isArray(exclusions) || exclusions.length > 32
    || !exclusions.every((entry) => typeof entry === "string" && entry.trim().length > 0 && entry.length <= 200)) {
    throw new Error("childExtensionExclusions must be up to 32 non-empty strings of at most 200 characters");
  }
  const settings = liveFieldsFrom(value);
  validateSettings(settings);
  return { settings, from };
}

/** Serialize read-modify-write so independent pi processes cannot lose fields. */
function withSettingsFileLock<T>(path: string, action: () => T): T {
  const directory = dirname(path);
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  try {
    return withSqliteMutex(`${path}.lock.sqlite`, action, SETTINGS_LOCK_WAIT_MS);
  } catch (error) {
    if (error instanceof SqliteLockBusyError) {
      throw new Error("another pi process is editing them; retry the change");
    }
    throw error;
  }
}

function copyDefaults(): WorkflowSettings {
  return { ...DEFAULT_WORKFLOW_SETTINGS };
}

function parseSettingsForMutation(text: string): WorkflowSettings {
  try {
    return parseSettings(text);
  } catch (error) {
    // A deliberate version reset is replaced on the next explicit change. Other
    // unreadable or invalid files retain the existing fail-closed save behavior.
    if (error instanceof SettingsVersionError) return copyDefaults();
    throw error;
  }
}

function integerInRange(value: unknown, min: number, max: number): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= min && value <= max;
}

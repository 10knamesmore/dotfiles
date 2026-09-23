import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { getAgentDir } from "@earendil-works/pi-coding-agent";
import { parsePersonalConfig, type PersonalConfig } from "./schema.js";

export {
  MODEL_TIERS,
  isModelTier,
  type ModelTier,
  type ModelTiers,
  type PersonalConfig,
  type WorkflowSettings,
} from "./schema.js";

/** One user-global file for personal extensions; Pi's native settings stay in settings.json. */
export function getPersonalConfigPath(): string {
  return join(getAgentDir(), "config.json");
}

/** Read current disk contents. A missing file uses defaults; invalid configuration throws. */
export function readPersonalConfig(): PersonalConfig {
  const path = getPersonalConfigPath();
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return parsePersonalConfig({}, path);
    throw error;
  }
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new Error(`Invalid JSON in personal configuration at ${path}`);
  }
  return parsePersonalConfig(value, path);
}

/** Read current values, change the caller's fields, and write the config back. */
export function updatePersonalConfig(update: (config: PersonalConfig) => void): void {
  const path = getPersonalConfigPath();
  const config = readPersonalConfig();
  update(config);
  const validated = parsePersonalConfig(config, path);
  mkdirSync(dirname(path), { recursive: true, mode: 0o700 });
  writeFileSync(path, `${JSON.stringify(validated, null, 2)}\n`, { mode: 0o600 });
}

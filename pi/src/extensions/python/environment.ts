import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Type, type Static } from "typebox";
import { Value } from "typebox/value";

const PROJECT_DIRECTORY = dirname(fileURLToPath(import.meta.url));
export const STARTUP_TIMEOUT_MS = 120_000;

const Environment = Type.Object({
  executable: Type.String(),
  version: Type.String(),
  packages: Type.Array(Type.Object({ name: Type.String(), version: Type.String() })),
});

/** Interpreter and installed distributions reported by the uv project, rather than inferred from its lockfile. */
export type PythonEnvironment = Static<typeof Environment>;

/** Use the same uv project and interpreter selection for inspection and persistent execution. */
export function pythonWorkerArguments(...args: string[]): string[] {
  return [
    "run",
    "--project",
    PROJECT_DIRECTORY,
    "--locked",
    "--",
    "python",
    "-u",
    join(PROJECT_DIRECTORY, "worker.py"),
    ...args,
  ];
}

export function isPythonEnvironment(value: unknown): value is PythonEnvironment {
  return Value.Check(Environment, value);
}

/** Prepare the locked uv environment and inspect it without opening a persistent namespace. */
export async function inspectPythonEnvironment(
  exec: ExtensionAPI["exec"],
  cwd: string,
  signal: AbortSignal,
): Promise<PythonEnvironment> {
  const result = await exec("uv", pythonWorkerArguments("--describe-environment"), {
    cwd,
    signal,
    timeout: STARTUP_TIMEOUT_MS,
  });
  if (result.killed || result.code !== 0) {
    throw new Error(result.stderr.trim() || "Python environment inspection did not complete.");
  }
  const environment: unknown = JSON.parse(result.stdout);
  if (!isPythonEnvironment(environment)) throw new Error("Invalid Python environment information.");
  return environment;
}

export function describePythonEnvironment(environment?: PythonEnvironment): string {
  if (!environment) return "Python version and installed third-party package versions: unavailable.";
  const packages = environment.packages.map((pkg) => `${pkg.name}==${pkg.version}`).join(", ");
  return `Python version: ${environment.version}\nInstalled third-party packages (distribution names): ${packages || "none"}`;
}

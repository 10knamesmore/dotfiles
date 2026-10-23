import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Type, type Static } from "typebox";
import { Value } from "typebox/value";

const PROJECT_DIRECTORY = dirname(fileURLToPath(import.meta.url));
export const STARTUP_TIMEOUT_MS = 120_000;

const Environment = Type.Object({
  executable: Type.String(),
  version: Type.String(),
  packages: Type.Array(Type.Object({ name: Type.String(), version: Type.String() })),
});

/** Installed interpreter and packages captured by dots sync. */
export type PythonEnvironment = Static<typeof Environment>;

/** dots sync owns dependency installation; tool execution only starts the prepared environment. */
export function pythonWorkerArguments(): string[] {
  return [
    "run",
    "--project",
    PROJECT_DIRECTORY,
    "--no-sync",
    "--",
    "python",
    "-u",
    join(PROJECT_DIRECTORY, "worker.py"),
  ];
}

/** Read the machine-local inventory without launching uv or Python. */
export async function readPythonEnvironment(): Promise<PythonEnvironment> {
  const manifest = new URL("../../../../.gen/pi/python-environment.json", import.meta.url);
  const environment: unknown = JSON.parse(await readFile(manifest, "utf8"));
  if (!Value.Check(Environment, environment)) {
    throw new Error("Invalid Python environment manifest. Run dots sync to regenerate it.");
  }
  return environment;
}

export function describePythonEnvironment(environment: PythonEnvironment): string {
  const packages = environment.packages.map((pkg) => `${pkg.name}==${pkg.version}`).join(", ");
  return `Python version: ${environment.version}\nInstalled third-party packages (distribution names): ${packages || "none"}`;
}

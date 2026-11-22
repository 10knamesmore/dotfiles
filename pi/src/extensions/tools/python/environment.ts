import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { PythonEnvironment } from "@dotfiles/pi-config/python-environment";

const PROJECT_DIRECTORY = dirname(fileURLToPath(import.meta.url));
export const STARTUP_TIMEOUT_MS = 120_000;

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

export function describePythonEnvironment(environment: PythonEnvironment): string {
  const packages = environment.packages.map((pkg) => `${pkg.name}==${pkg.version}`).join(", ");
  return `Python version: ${environment.version}\nInstalled third-party packages (distribution names): ${packages || "none"}`;
}

import { readFile } from "node:fs/promises";
import { Type, type Static } from "typebox";
import { Value } from "typebox/value";

const Environment = Type.Object({
  executable: Type.String(),
  version: Type.String(),
  packages: Type.Array(Type.Object({ name: Type.String(), version: Type.String() })),
});

export type PythonEnvironment = Static<typeof Environment>;

/** Read the interpreter and installed packages recorded by dots sync. */
export async function readPythonEnvironment(): Promise<PythonEnvironment> {
  const manifest = new URL("../../../.gen/pi/python-environment.json", import.meta.url);
  const environment: unknown = JSON.parse(await readFile(manifest, "utf8"));
  if (!Value.Check(Environment, environment)) {
    throw new Error("Invalid Python environment manifest. Run dots sync to regenerate it.");
  }
  return environment;
}

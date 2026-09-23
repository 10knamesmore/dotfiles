/**
 * The child-shim extension, loaded (via -e) into every child pi process the
 * subprocess backend spawns. It carries the two construction-time injections
 * that cannot cross the pi CLI boundary as flags:
 *
 * - report_result: registered from the spec's JSON Schema so structured
 *   output gets pi's own TypeBox validation inside the child.
 * - the tool report: the child's active tool names, published to a file the
 *   parent reads before the first prompt. It serves the missing-tool and
 *   recursion-guard checks only; workflow replay fingerprints deliberately
 *   use the parent's prospective scan instead.
 *
 * Without the spec environment variable the shim is inert, so loading it
 * into an ordinary pi session by accident does nothing.
 */

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { createReportResultTool, type SchemaCapture } from "../src/runner/schema-tool.js";
import { readShimSpec, SHIM_SPEC_ENV, writeToolReport } from "../src/runner/subprocess/shim-contract.js";

export default function childShim(pi: ExtensionAPI): void {
  const specPath = process.env[SHIM_SPEC_ENV];
  if (!specPath) return;
  const spec = readShimSpec(specPath);
  if (spec.schema) {
    // The capture is child-local and only enforces the call-once contract;
    // the parent observes the call through tool events and owns termination.
    const capture: SchemaCapture = { called: false };
    pi.registerTool(createReportResultTool(spec.schema, capture));
  }
  // resources_discover fires after EVERY extension's session_start handler
  // has completed (agent-session emits it immediately after the session_start
  // pass), so the report reflects tool mutations later extensions made during
  // startup - a session_start-time report from this shim (loaded first, as a
  // CLI extension) would be stale.
  pi.on("resources_discover", () => {
    writeToolReport(spec.toolReportPath, { activeTools: pi.getActiveTools() });
    return undefined;
  });
}

import { randomUUID } from "node:crypto";
import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";
import { executionText } from "./render.js";
import type { PythonSession } from "./session.js";

interface NeovimInstance {
  instance_id: string;
  pid: number;
  cwd: string;
  current_file: string;
  connection: { session_id: string } | null;
}

/** Use the tool's worker so the selected connection survives into subsequent Python calls. */
export function registerNeovimCommand(
  pi: ExtensionAPI,
  getSession: (ctx: ExtensionContext) => PythonSession,
): void {
  pi.registerCommand("nvim", {
    description: "Choose and connect a Neovim instance",
    handler: async (_args, ctx) => {
      if (!ctx.hasUI) throw new Error("/nvim requires an interactive instance selector.");

      const execute = async (expression: string): Promise<string> => {
        await ctx.waitForIdle();
        const runtime = getSession(ctx);
        const result = await runtime.execute(
          `print(__import__("json").dumps(${expression}))`,
          30,
          `nvim-command-${randomUUID()}`,
          ctx.cwd,
        );
        if (result.details.outcome !== "completed") throw new Error(executionText(result));
        return result.output;
      };

      const appendResult = (content: string): void => {
        pi.sendMessage(
          { customType: "nvim-connection", content, display: true },
          { triggerTurn: false, deliverAs: "steer" },
        );
      };

      try {
        const instances = JSON.parse(await execute('__import__("nvim_use").discover()')) as NeovimInstance[];
        if (instances.length === 0) {
          appendResult("Neovim: no running instances found.");
          return;
        }
        const labels = instances.map((instance) => {
          const connection = instance.connection;
          const status = !connection
            ? "available"
            : connection.session_id === ctx.sessionManager.getSessionId()
              ? "this session"
              : `in use: ${connection.session_id.slice(0, 8)}`;
          return `${instance.cwd} · PID ${instance.pid} · ${status} · ${instance.current_file || "[No Name]"}`;
        });
        const choice = await ctx.ui.select("Connect Neovim", labels);
        if (choice === undefined) return;
        const instance = instances[labels.indexOf(choice)]!;
        const instanceId = JSON.parse(
          await execute(`__import__("nvim_use").select(instance_id=${JSON.stringify(instance.instance_id)}).instance_id`),
        ) as string;
        appendResult(
          `Neovim connected: ${instance.cwd} (PID ${instance.pid}).\n` +
            `Instance: ${instanceId}\n` +
            'Selected connection is available in python_repl as: import nvim_use as nvim; editor = nvim.current()\n' +
            "Read the nvim-use skill before editor operations or showcast. This selection lasts until disconnect or Python environment reset.",
        );
      } catch (error) {
        appendResult(`Neovim connection failed:\n${error instanceof Error ? error.message : String(error)}`);
      }
    },
  });
}

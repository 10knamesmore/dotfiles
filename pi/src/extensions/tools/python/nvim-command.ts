import { randomUUID } from "node:crypto";
import { homedir } from "node:os";
import { resolve } from "node:path";
import { keyHint, type ExtensionAPI, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { replaceControlCharacters, truncateToWidth, visibleWidth } from "@earendil-works/pi-tui";
import { executionText } from "./render.js";
import type { PythonSession } from "./session.js";

interface NeovimInstance {
  instance_id: string;
  pid: number;
  cwd: string;
  current_file: string;
  connection: { session_id: string } | null;
}

function displayPath(path: string): string {
  const home = resolve(homedir());
  if (path === home) return "~";
  const prefix = home.endsWith("/") ? home : `${home}/`;
  return replaceControlCharacters(path.startsWith(prefix) ? `~/${path.slice(prefix.length)}` : path);
}

const divider = " │ ";

function fitColumnWidths(preferredWidths: number[], width: number): number[] {
  const count = preferredWidths.length;
  let remaining = Math.max(0, width - (count - 1) * visibleWidth(divider));
  const widths = [...preferredWidths];
  const shortestFirst = widths.map((_, i) => i).sort((a, b) => widths[a]! - widths[b]!);
  shortestFirst.forEach((column, index) => {
    const share = Math.floor(remaining / (count - index));
    widths[column] = Math.min(preferredWidths[column]!, share);
    remaining -= widths[column]!;
  });
  return widths;
}

function renderInstanceRow(cells: string[], widths: number[]): string {
  return cells
    .map((cell, i) => {
      const width = widths[i]!;
      const text = truncateToWidth(cell, width, "…");
      const padding = width - visibleWidth(text);
      const left = Math.floor(padding / 2);
      return " ".repeat(left) + text + " ".repeat(padding - left);
    })
    .join(divider);
}

async function chooseInstance(ctx: ExtensionContext, instances: NeovimInstance[]): Promise<NeovimInstance | undefined> {
  const title = "Connect Neovim";
  const sessionId = ctx.sessionManager.getSessionId();
  const rows = instances.map(({ pid, connection, cwd, current_file }) => [
    String(pid),
    !connection
      ? "available"
      : connection.session_id === sessionId
        ? "this session"
        : `in use: ${connection.session_id.slice(0, 8)}`,
    displayPath(cwd),
    displayPath(current_file) || "[No Name]",
  ]);
  const preferredWidths = rows[0]!.map((_, i) => Math.max(...rows.map((row) => visibleWidth(row[i]!))));

  if (ctx.mode === "rpc") {
    const labels = rows.map((row) => renderInstanceRow(row, preferredWidths));
    const choice = await ctx.ui.select(title, labels);
    return choice === undefined ? undefined : instances[labels.indexOf(choice)];
  }

  return ctx.ui.custom<NeovimInstance | undefined>(
    (tui, theme, keys, done) => {
      let selected = 0;
      return {
        render(width) {
          const widths = fitColumnWidths(preferredWidths, width - 2);
          const count = Math.max(1, Math.min(10, tui.terminal.rows - 2));
          const start = Math.max(0, Math.min(selected - Math.floor(count / 2), rows.length - count));
          const position = rows.length > count ? ` (${selected + 1}/${rows.length})` : "";
          const visible = rows.slice(start, start + count).map((row, i) => {
            const line = renderInstanceRow(row, widths);
            return start + i === selected ? theme.fg("accent", `→ ${line}`) : `  ${line}`;
          });
          return [
            ...visible,
            theme.fg("dim", "↑↓ navigate  ") +
              keyHint("tui.select.confirm", "connect") +
              "  " +
              keyHint("tui.select.cancel", "cancel") +
              theme.fg("dim", position),
          ].map((line) => truncateToWidth(line, width, ""));
        },
        invalidate() {},
        handleInput(data) {
          if (keys.matches(data, "tui.select.up") || data === "k") selected = Math.max(0, selected - 1);
          else if (keys.matches(data, "tui.select.down") || data === "j")
            selected = Math.min(rows.length - 1, selected + 1);
          else if (keys.matches(data, "tui.select.confirm") || data === "\n") return done(instances[selected]);
          else if (keys.matches(data, "tui.select.cancel")) return done(undefined);
          else return;
          tui.requestRender();
        },
      };
    },
    { blocked: { kind: "question", message: title } },
  );
}

/** Use the tool's worker so the selected connection survives into subsequent Python calls. */
export function registerNeovimCommand(pi: ExtensionAPI, getSession: (ctx: ExtensionContext) => PythonSession): void {
  pi.registerCommand("nvim", {
    description: "Choose and connect a Neovim instance",
    handler: async (_args, ctx) => {
      if (!ctx.hasUI) throw new Error("/nvim requires an interactive instance selector.");

      const execute = async (expression: string): Promise<string> => {
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
          { triggerTurn: !ctx.isIdle(), deliverAs: "steer" },
        );
      };

      try {
        const instances = JSON.parse(await execute('__import__("nvim_use").discover()')) as NeovimInstance[];
        if (instances.length === 0) {
          appendResult("Neovim: no running instances found.");
          return;
        }
        const instance = await chooseInstance(ctx, instances);
        if (!instance) return;
        const instanceId = JSON.parse(
          await execute(
            `__import__("nvim_use").select(instance_id=${JSON.stringify(instance.instance_id)}).instance_id`,
          ),
        ) as string;
        appendResult(
          `Neovim connected: ${displayPath(instance.cwd)} (PID ${instance.pid}).\n` +
            `Instance: ${instanceId}\n` +
            "Selected connection is available in python_repl as: import nvim_use as nvim; editor = nvim.current()\n" +
            "Read the nvim-use skill before editor operations. This selection lasts until disconnect or Python environment reset.",
        );
      } catch (error) {
        appendResult(`Neovim connection failed:\n${error instanceof Error ? error.message : String(error)}`);
      }
    },
  });
}

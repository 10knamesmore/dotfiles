/** Read-only process picker and live output browser for background bash commands. */
import { readFileSync, watch, type FSWatcher } from "node:fs";
import { stripVTControlCharacters } from "node:util";
import type { ExtensionContext, Theme, ThemeColor } from "@earendil-works/pi-coding-agent";
import { matchesKey, parseKey, truncateToWidth, type Component, type TUI } from "@earendil-works/pi-tui";
import { formatDuration, sanitizeFooterText } from "../footer/format.js";
import { runningProcesses, subscribeToProcesses, type BackgroundProcess, type BackgroundSession } from "./model.js";

type BrowserContext = Pick<ExtensionContext, "mode" | "hasUI" | "ui">;

interface BrowserServices {
  stop: (pid: number) => void;
}

/** A live log read on filesystem changes. No subprocess input or PTY is involved. */
class LogOutput {
  private watcher: FSWatcher | undefined;
  private raw = "";
  private lines: string[] = [];

  constructor(
    path: string,
    private readonly changed: () => void,
  ) {
    this.refresh(path);
    this.watcher = watch(path, () => {
      this.refresh(path);
      this.changed();
    });
  }

  snapshot(): string[] {
    return this.lines;
  }

  dispose(): void {
    this.watcher?.close();
    this.watcher = undefined;
  }

  private refresh(path: string): void {
    const raw = readFileSync(path, "utf8");
    if (raw === this.raw) return;
    this.raw = raw;
    this.lines = terminalLines(raw);
  }
}

/**
 * Preserve SGR colors while discarding cursor movement and other terminal controls.
 * Read-only log replay needs line integrity, not an interactive terminal emulator.
 */
function terminalLines(raw: string): string[] {
  const styles: string[] = [];
  const protectedStyles = raw.replace(/\x1b\[[0-?]*m/g, (style) => {
    const marker = `\uE000${styles.length.toString(36)}\uE001`;
    styles.push(style);
    return marker;
  });
  const visible = stripVTControlCharacters(protectedStyles)
    .replace(/\r\n?/g, "\n")
    .replace(/\t/g, "   ")
    .replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, " ");
  return visible
    .replace(/\uE000([0-9a-z]+)\uE001/g, (_marker, index: string) => styles[Number.parseInt(index, 36)] ?? "")
    .split("\n")
    .map((line) => `${line}\x1b[0m`);
}

function processStatus(process: BackgroundProcess): { glyph: string; text: string; color: ThemeColor } {
  if (!process.exit) return { glyph: "●", text: "running", color: "success" };
  if (process.exit.timedOut) return { glyph: "■", text: "timed out", color: "warning" };
  if (process.exit.signal !== null) return { glyph: "■", text: process.exit.signal, color: "warning" };
  if (process.exit.code === 0) return { glyph: "✓", text: "exit 0", color: "success" };
  return { glyph: "✗", text: `exit ${process.exit.code ?? "unknown"}`, color: "error" };
}

function elapsed(process: BackgroundProcess): string {
  return formatDuration((process.exit?.endedAt ?? Date.now()) - process.startedAt);
}

function commandLine(process: BackgroundProcess): string {
  return sanitizeFooterText(process.command);
}

function keyId(data: string): string | undefined {
  if (matchesKey(data, "shift+g")) return "shift+g";
  if (matchesKey(data, "shift+j")) return "shift+j";
  if (matchesKey(data, "shift+k")) return "shift+k";
  return (parseKey(data) ?? "").toLowerCase() || undefined;
}

function panelLines(
  title: string,
  content: readonly string[],
  footer: string,
  inner: number,
  totalRows: number,
  theme: Theme,
): string[] {
  const border = (text: string) => theme.fg("muted", text);
  const pad = (line: string) => `${border("│")} ${truncateToWidth(line, inner, "", true)} ${border("│")}`;
  const heading = truncateToWidth(` ${title} `, inner + 1, "…", true);
  const top = border(`╭─${heading}${"─".repeat(Math.max(0, inner - heading.length))}╮`);
  const bodyRows = Math.max(1, totalRows - 3);
  const rows = content.slice(0, bodyRows).map(pad);
  while (rows.length < bodyRows) rows.push(pad(""));
  rows.push(pad(footer));
  return [top, ...rows, border(`╰${"─".repeat(inner + 2)}╯`)];
}

export async function openBackgroundBrowser(
  ctx: BrowserContext,
  session: BackgroundSession,
  services: BrowserServices,
): Promise<void> {
  const running = runningProcesses(session);
  const tasks = [...session.processes.values()].sort(
    (left, right) =>
      Number(left.exit !== undefined) - Number(right.exit !== undefined) || left.startedAt - right.startedAt,
  );
  if (tasks.length === 0) {
    if (ctx.hasUI) ctx.ui.notify("No background bash commands in this session", "info");
    return;
  }
  if (ctx.mode !== "tui") return;

  await ctx.ui.custom<void>((tui: TUI, theme: Theme, _keybindings, done: (result: void) => void) => {
    let page: "picker" | "output" = running.length === 1 ? "output" : "picker";
    let returnToPicker = running.length !== 1;
    let selectedPid = running[0]?.pid ?? tasks[0]!.pid;
    let outputProcess = running.length === 1 ? running[0] : undefined;
    let output: LogOutput | undefined;
    let scroll = 0;
    let follow = true;
    let viewport = 1;
    let stopArmedPid: number | undefined;
    let disposed = false;

    const render = () => {
      if (!disposed) tui.requestRender();
    };
    const openOutput = (process: BackgroundProcess) => {
      returnToPicker = page === "picker";
      output?.dispose();
      outputProcess = process;
      output = new LogOutput(process.logPath, render);
      page = "output";
      scroll = 0;
      follow = true;
      stopArmedPid = undefined;
      render();
    };
    if (outputProcess) openOutput(outputProcess);
    const unsubscribe = subscribeToProcesses(session, render);
    const timer = setInterval(render, 1_000);
    const cleanup = () => {
      disposed = true;
      clearInterval(timer);
      unsubscribe();
      output?.dispose();
      output = undefined;
    };
    const finish = () => {
      cleanup();
      done();
    };

    const currentTask = () => tasks.find((task) => task.pid === selectedPid) ?? tasks[0]!;
    const currentOutput = () => outputProcess!;
    const scrollBy = (amount: number) => {
      const max = Math.max(0, (output?.snapshot().length ?? 0) - viewport);
      const origin = follow ? max : scroll;
      scroll = Math.max(0, Math.min(max, origin + amount));
      follow = scroll >= max;
    };
    const stop = (process: BackgroundProcess) => {
      if (process.exit) return;
      if (stopArmedPid !== process.pid) {
        stopArmedPid = process.pid;
      } else {
        stopArmedPid = undefined;
        services.stop(process.pid);
      }
      render();
    };

    const component: Component & { dispose(): void } = {
      render(width: number) {
        const totalRows = Math.max(8, Math.floor((tui.terminal?.rows ?? 24) * 0.9));
        const bodyRows = Math.max(1, totalRows - 3);
        const inner = Math.max(20, width - 4);
        if (page === "picker") {
          const runningCount = runningProcesses(session).length;
          const lines = tasks.map((process) => {
            const status = processStatus(process);
            const cursor = process.pid === selectedPid ? theme.fg("accent", "❯") : " ";
            const command = truncateToWidth(commandLine(process), Math.max(12, inner - 44), "…", true);
            return `${cursor} ${theme.fg(status.color, status.glyph)} ${command}  ${theme.fg(status.color, status.text)}  ${theme.fg("dim", `${elapsed(process)} · pid ${process.pid}`)}`;
          });
          const footer =
            stopArmedPid === selectedPid
              ? "x again to STOP · esc close"
              : "↑↓/j/k select · enter output · x stop · esc/q close";
          return panelLines(
            `backgrounds · ${runningCount} running`,
            lines,
            theme.fg("dim", footer),
            inner,
            totalRows,
            theme,
          );
        }

        const process = currentOutput();
        const status = processStatus(process);
        const title = `${commandLine(process)} · ${status.text} · pid ${process.pid} · ${elapsed(process)}`;
        const lines = output?.snapshot() ?? [];
        viewport = Math.max(1, bodyRows - 2);
        const max = Math.max(0, lines.length - viewport);
        if (follow) scroll = max;
        else scroll = Math.min(scroll, max);
        const visible = lines.slice(scroll, scroll + viewport);
        while (visible.length < viewport) visible.push("");
        const content = [
          `${theme.fg(status.color, status.glyph)} ${theme.bold(title)}`,
          theme.fg("dim", "─".repeat(inner)),
          ...visible,
        ];
        const back = returnToPicker ? "back" : "close";
        const position = follow ? (process.exit ? "End of output" : "Following latest") : "Reading history";
        const stopHint = process.exit ? "" : stopArmedPid === process.pid ? " · x again to STOP" : " · x stop";
        return panelLines(
          "background output",
          content,
          theme.fg("dim", `${position} · ↑↓/j/k line · PgUp/PgDn page · g/G ends · esc ${back}${stopHint} · q close`),
          inner,
          totalRows,
          theme,
        );
      },
      handleInput(data: string) {
        const key = keyId(data);
        if (key !== "x") stopArmedPid = undefined;
        if (key === "q") {
          finish();
          return;
        }
        if (page === "picker") {
          const index = tasks.findIndex((task) => task.pid === selectedPid);
          if (key === "down" || key === "j") selectedPid = tasks[Math.min(tasks.length - 1, index + 1)]!.pid;
          else if (key === "up" || key === "k") selectedPid = tasks[Math.max(0, index - 1)]!.pid;
          else if (key === "enter" || key === "return" || key === "l") {
            openOutput(currentTask());
            return;
          } else if (key === "escape" || key === "esc" || key === "h") {
            finish();
            return;
          } else if (key === "x") {
            stop(currentTask());
            return;
          } else return;
          render();
          return;
        }

        if (key === "escape" || key === "esc" || key === "h") {
          if (returnToPicker) {
            output?.dispose();
            output = undefined;
            outputProcess = undefined;
            page = "picker";
            render();
          } else finish();
        } else if (key === "up" || key === "k" || key === "ctrl+u") scrollBy(-1);
        else if (key === "down" || key === "j" || key === "ctrl+d") scrollBy(1);
        else if (key === "pageup" || key === "ctrl+b") scrollBy(-viewport);
        else if (key === "pagedown" || key === "ctrl+f") scrollBy(viewport);
        else if (key === "g") {
          scroll = 0;
          follow = false;
        } else if (key === "shift+g") {
          follow = true;
        } else if (key === "x") {
          stop(currentOutput());
          return;
        } else return;
        render();
      },
      invalidate() {
        // Every render reads current process state and the latest log snapshot.
      },
      dispose: cleanup,
    };
    return component;
  });
}

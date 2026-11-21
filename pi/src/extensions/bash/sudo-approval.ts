import { highlightCode, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { Key, matchesKey, wrapTextWithAnsi } from "@earendil-works/pi-tui";

/** Show terminal controls and bidi overrides as text rather than interpreting them in a root command preview. */
export function visibleCommand(command: string): string {
  return command.replace(/[\p{Cc}\p{Bidi_Control}]/gu, (char) => {
    if (char === "\n") return char;
    if (char === "\t") return "    ";
    return `⟨U+${char.codePointAt(0)!.toString(16).toUpperCase().padStart(4, "0")}⟩`;
  });
}

export function approveRootCommand(
  ctx: ExtensionContext,
  command: string,
  cwd: string,
  signal: AbortSignal | undefined,
): Promise<boolean> {
  return ctx.ui.custom<boolean>(
    (tui, theme, _keys, done) => {
      const cancel = () => done(false);
      if (signal?.aborted) cancel();
      else signal?.addEventListener("abort", cancel, { once: true });
      let confirming = false;
      let selected: "deny" | "allow" = "deny";
      const choice = (action: typeof selected, label: string) =>
        action === selected
          ? theme.style(` ${label} `, { fg: "text", bg: "selectedBg", bold: true })
          : theme.fg("muted", ` ${label} `);
      let offset = 0;
      let pageSize = 1;
      let cachedWidth = 0;
      let lines: string[] = [];
      return {
        render(width) {
          if (width !== cachedWidth) {
            cachedWidth = width;
            lines = wrapTextWithAnsi(highlightCode(visibleCommand(command), "bash").join("\n"), Math.max(1, width));
          }
          pageSize = Math.max(1, Math.min(12, Math.floor(tui.terminal.rows / 3)));
          offset = Math.min(Math.max(0, lines.length - pageSize), Math.max(0, offset));
          const end = Math.min(offset + pageSize, lines.length);
          const scroll = lines.length > pageSize ? `   ↑↓ / PgUp PgDn  ${offset + 1}–${end} / ${lines.length}` : "";
          const border = theme.fg("border", "─".repeat(width));
          const root = theme.fg("warning", "root");
          const prompt = confirming ? `Confirm: run this command as ${root}?` : `Run once as ${root}?`;
          return [
            border,
            ...wrapTextWithAnsi(`${prompt} · ${visibleCommand(cwd)}`, width),
            "",
            ...lines.slice(offset, end),
            "",
            ...wrapTextWithAnsi(
              `${choice("deny", "Deny")}  ${choice("allow", confirming ? "Confirm" : "Allow")}`,
              width,
            ),
            ...wrapTextWithAnsi(theme.fg("dim", `← → Choose   Enter Select   Esc Exit${scroll}`), width),
            border,
          ];
        },
        handleInput(data) {
          if (matchesKey(data, Key.escape) || matchesKey(data, Key.ctrl("c"))) return cancel();
          if (matchesKey(data, Key.left)) selected = "deny";
          else if (matchesKey(data, Key.right)) selected = "allow";
          else if (matchesKey(data, Key.enter)) {
            if (selected === "deny") return cancel();
            if (confirming) return done(true);
            confirming = true;
            selected = "deny";
          } else if (matchesKey(data, Key.up)) offset--;
          else if (matchesKey(data, Key.down)) offset++;
          else if (matchesKey(data, Key.pageUp)) offset -= pageSize;
          else if (matchesKey(data, Key.pageDown)) offset += pageSize;
          else return;
          tui.requestRender();
        },
        invalidate() {
          cachedWidth = 0;
        },
        dispose() {
          signal?.removeEventListener("abort", cancel);
        },
      };
    },
    { blocked: { kind: "permission", message: "Approve this Bash command to run once as root?" } },
  );
}

import { CustomEditor, type ExtensionContext, type KeybindingsManager } from "@earendil-works/pi-coding-agent";
import { stripTerminalSequences, truncateToWidth, visibleWidth, type EditorTheme, type TUI } from "@earendil-works/pi-tui";
import { formatDuration, formatTokensPerSecond, formatTokenLatency, sanitizeFooterText } from "../footer/format.js";
import { palette } from "../footer/palette.js";
import type { EditorActivity, EditorStatus } from "./api.js";
import type { AgentInputNavigation } from "./navigation.js";

/** Pi's input editor, with session information on its borders and optional agent navigation. */
export class PromptEditor extends CustomEditor {
  constructor(
    tui: TUI,
    theme: EditorTheme,
    keybindings: KeybindingsManager,
    private readonly navigation: () => AgentInputNavigation | undefined,
    private readonly status: () => EditorStatus | undefined,
    private readonly context: () => ExtensionContext | undefined,
    private readonly todoStatus: () => string | undefined = () => undefined,
    private readonly workflowUsage: () => string | undefined = () => undefined,
  ) {
    super(tui, theme, keybindings);
    this.inputKeybindings = keybindings;
  }

  private readonly inputKeybindings: KeybindingsManager;

  override handleInput(data: string): void {
    const navigation = this.navigation();
    if (!navigation || this.isShowingAutocomplete()) {
      super.handleInput(data);
      return;
    }

    const selected = navigation.hasSelection();
    if (this.inputKeybindings.matches(data, "tui.editor.cursorDown")
      && (selected || this.isOnLastInputLine())
      && navigation.selectNext()) {
      return;
    }
    if (selected && this.inputKeybindings.matches(data, "tui.editor.cursorUp")) {
      navigation.selectPrevious();
      return;
    }
    if (selected) {
      if (this.inputKeybindings.matches(data, "tui.input.submit") && navigation.openSelection()) return;
      if (this.inputKeybindings.matches(data, "app.interrupt")) {
        navigation.clearSelection();
        return;
      }
      navigation.clearSelection();
    }
    super.handleInput(data);
  }

  override render(width: number): string[] {
    if (width < 6) return super.render(width);
    // Reserve one column on each side before the base Editor lays out text,
    // its hardware cursor, scroll indicators, and autocomplete suggestions.
    const lines = super.render(width - 2);
    let bottom = -1;
    for (let index = lines.length - 1; index > 0; index -= 1) {
      if (!/^(?:─+|─── ↓.*|─*\.{1,3})$/u.test(stripTerminalSequences(lines[index]!))) continue;
      bottom = index;
      break;
    }
    if (bottom < 0) return super.render(width);

    const status = this.status();
    const top = status ? this.activityLabel(status.activity) + this.timingLabel(status) : palette.readyBadge(" READY ");
    const ctx = this.context();
    const model = ctx?.model;
    const modelLabel = model
      ? `${palette.overlay2(`${sanitizeFooterText(model.provider)}/`)}${palette.sky(sanitizeFooterText(model.name || model.id))}${model.reasoning ? `${palette.overlay2(" · ")}${palette.mauve(ctx.thinkingLevel ?? "off")}` : ""}`
      : palette.sky("no-model");
    const sides = [
      ...lines.slice(1, bottom),
      ...lines.slice(bottom + 1),
    ].map((line) => `${this.borderColor("│")}${line}${this.borderColor("│")}`);
    return [
      this.frameBorder(lines[0] ?? "", top, width, "╭", "╮", this.todoStatus()),
      ...sides,
      this.frameBorder(lines[bottom]!, modelLabel, width, "╰", "╯", this.workflowUsage()),
    ];
  }

  private activityLabel(activity: EditorActivity): string {
    switch (activity.kind) {
      case "waiting":
      case "thinking":
      case "writing":
      case "compacting":
        return palette.modelBadge(` ${activity.spinner} ${activity.kind.toUpperCase()} `);
      case "tool": return palette.toolBadge(activity.toolCount > 1
        ? ` ${activity.spinner} TOOLS ${activity.toolCount} `
        : ` ${activity.spinner} TOOL · ${sanitizeFooterText(activity.toolName)} `);
      case "ready": return palette.readyBadge(" READY ");
    }
  }

  private timingLabel(status: EditorStatus): string {
    const showLatency = status.activity.kind !== "tool" && status.activity.kind !== "compacting";
    const latency = status.timeToFirstTokenMilliseconds === undefined
      ? status.activity.kind === "waiting" ? "…" : undefined
      : formatTokenLatency(status.timeToFirstTokenMilliseconds);
    const ttft = showLatency && latency !== undefined
      ? `${palette.overlay2("ttft")} ${palette.sky(latency)}${palette.overlay2(" · ")}`
      : "";
    const idle = status.idleMilliseconds === undefined
      ? ""
      : `${palette.overlay2("idle")} ${palette.lavender(formatDuration(status.idleMilliseconds))}${palette.overlay2(" · ")}`;
    const session = palette.lavender(formatDuration(status.sessionMilliseconds));
    const api = palette.sky(formatDuration(status.apiMilliseconds));
    const tps = status.tokensPerSecond === undefined
      ? ""
      : `${palette.overlay2(" · ")}${palette.sky(formatTokensPerSecond(status.tokensPerSecond))} ${palette.overlay2("t/s")}`;
    return ` ${ttft}${idle}${palette.overlay2("session")} ${session}${palette.overlay2(" · api ")}${api}${tps}`;
  }

  private frameBorder(
    original: string,
    text: string,
    width: number,
    left: "╭" | "╰",
    right: "╮" | "╯",
    rightStatus?: string,
  ): string {
    const prefix = left === "╭" ? "╭─" : "╰─ ";
    const rightLabel = rightStatus && width >= 32
      ? truncateToWidth(
        palette.lavender(sanitizeFooterText(rightStatus)),
        Math.floor((width - 6) * 0.48),
        palette.overlay2("…"),
      )
      : "";
    const rightWidth = visibleWidth(rightLabel);
    const budget = Math.max(0, width - visibleWidth(prefix) - (rightLabel ? rightWidth + 6 : 3));
    const scroll = /^─── ([↑↓]) (\d+) more /u.exec(stripTerminalSequences(original));
    const hint = scroll ? `${scroll[1]}${scroll[2]}` : "";
    const textWidth = hint && visibleWidth(hint) + 1 < budget ? budget - visibleWidth(hint) - 1 : budget;
    const label = truncateToWidth(text, textWidth, palette.overlay2("…"))
      + (textWidth < budget ? ` ${hint}` : "");
    const remaining = width - visibleWidth(prefix) - visibleWidth(label) - 1;
    if (rightLabel) {
      const fill = "─".repeat(remaining - (label ? 1 : 0) - rightWidth - 3);
      return this.borderColor(prefix) + label
        + this.borderColor(`${label ? " " : ""}${fill} `) + rightLabel + this.borderColor(` ─${right}`);
    }
    return this.borderColor(prefix) + label
      + this.borderColor(`${label ? " " : ""}${"─".repeat(remaining - (label ? 1 : 0))}${right}`);
  }

  private isOnLastInputLine(): boolean {
    const cursor = this.getCursor();
    return cursor.line >= this.getLines().length - 1;
  }
}

import type { AssistantMessage } from "@earendil-works/pi-ai";
import { keyText, SettingsManager, type KeybindingsManager, type Theme } from "@earendil-works/pi-coding-agent";
import { Input, ScrollView, truncateToWidth, type TUI, type TuiMouseEvent } from "@earendil-works/pi-tui";
import type { TranscriptMessage } from "../../store/session-transcript.js";
import type { SubagentResult, SubagentStatus, UsageSummary } from "../../types.js";
import { formatTokenUsage, modelEffort, statusGlyph } from "../format.js";
import { sanitizeTerminalText } from "../sanitize.js";
import { AgentTranscript } from "./agent-transcript.js";
import { keyToAction, scrollDistance } from "./controls.js";

interface AgentViewDeps {
  tui: TUI;
  theme: Theme;
  keybindings: KeybindingsManager;
  settingsCwd: string;
  cwd: () => string;
  header: () => { label: string; model: string; thinking?: string; status: SubagentStatus; usage: UsageSummary };
  messages: () => { history: readonly TranscriptMessage[]; current?: AssistantMessage };
  result: () => Pick<SubagentResult, "status" | "text" | "structured"> | undefined;
  live: () => boolean;
  onSteer: (text: string) => void;
  canMessage: () => boolean;
  onMessage?: (text: string) => boolean;
  onSteerUnavailable: () => void;
  onStop: () => void;
  subscribe: (listener: () => void) => () => void;
}

export class AgentView {
  private readonly transcript: AgentTranscript;
  private readonly scroll: ScrollView;
  private readonly unsubscribe: () => void;
  private readonly requestRender = () => this.deps.tui.requestRender();
  private stopArmed = false;
  private composer: Input | undefined;
  private error: string | undefined;
  private hasFocus = false;
  private contentHeight = 0;

  constructor(private readonly deps: AgentViewDeps) {
    const settings = SettingsManager.create(deps.settingsCwd);
    this.transcript = new AgentTranscript(deps.tui, deps.cwd, settings);
    this.scroll = new ScrollView(this.transcript, { follow: "end" });
    this.unsubscribe = deps.subscribe(this.requestRender);
  }

  set focused(value: boolean) {
    this.hasFocus = value;
    if (this.composer) this.composer.focused = value;
  }

  dispose(): void {
    this.unsubscribe();
  }

  invalidate(): void {
    this.transcript.invalidate();
  }

  showError(message: string): void {
    this.error = message;
    this.requestRender();
  }

  handleInput(data: string, keyId: string | undefined): boolean {
    this.error = undefined;
    if (this.composer) {
      this.composer.handleInput(data);
      this.requestRender();
      return true;
    }
    const confirmStop = this.stopArmed;
    this.stopArmed = false;
    const keys = this.deps.keybindings;
    if (keys.matches(data, "app.tools.expand")) this.transcript.toggleTools();
    else if (keys.matches(data, "app.thinking.toggle")) this.transcript.toggleThinking();
    else if (keys.matches(data, "tui.altScreen.top")) this.scroll.scrollToStart();
    else if (keys.matches(data, "tui.altScreen.bottom")) this.scroll.scrollToEnd();
    else if (keys.matches(data, "tui.altScreen.pageUp")) this.scroll.scrollBy(-this.scroll.viewportHeight);
    else if (keys.matches(data, "tui.altScreen.pageDown")) this.scroll.scrollBy(this.scroll.viewportHeight);
    else if (keys.matches(data, "tui.select.confirm")) {
      if (this.deps.live() || this.deps.canMessage()) this.openComposer();
    } else if (keyId === "x") {
      if (this.deps.live()) {
        if (confirmStop) this.deps.onStop();
        else this.stopArmed = true;
      }
    } else {
      const action = keyToAction(keyId, "agent");
      if (action.type === "scroll") this.scroll.scrollBy(scrollDistance(action.amount, this.scroll.viewportHeight));
      else if (action.type === "move") this.scroll.scrollBy(action.delta);
      else return false;
    }
    this.requestRender();
    return true;
  }

  handleMouse(event: TuiMouseEvent) {
    const y = event.y - 1;
    if (y < 0 || y >= this.scroll.viewportHeight) return undefined;
    if (event.type === "wheel" && event.wheelDelta) {
      this.scroll.scrollBy(event.wheelDelta);
      return { handled: true };
    }
    return this.transcript.handleMouse({ ...event, y: y + this.scroll.scrollTop, height: this.contentHeight });
  }

  private openComposer(): void {
    const input = new Input();
    input.focused = this.hasFocus;
    input.onSubmit = (value) => {
      const text = value.trim();
      if (this.deps.live()) {
        this.composer = undefined;
        if (text) this.deps.onSteer(text);
      } else if (this.deps.canMessage()) {
        if (this.deps.onMessage?.(text)) this.composer = undefined;
      } else this.deps.onSteerUnavailable();
    };
    input.onEscape = () => {
      this.composer = undefined;
    };
    this.composer = input;
  }

  render(width: number, bodyRows: number) {
    const { header, messages, result, live, theme } = this.deps;
    const canSteer = live();
    const { history, current } = messages();
    this.transcript.update(history, current, canSteer, result());
    const lines = this.scroll.render(width);
    const viewport = Math.max(1, bodyRows - 1 - (this.composer ? 1 : 0));
    this.contentHeight = lines.length;
    // Custom overlays render into a row budget supplied by the navigator.
    this.scroll.updateLayout(lines.length, viewport, this.requestRender);
    const top = this.scroll.scrollTop;
    const visible = lines.length ? lines.slice(top, top + viewport) : [theme.fg("muted", "No transcript yet.")];
    const { label, model, thinking, status, usage } = header();
    const glyph = statusGlyph(status, theme, Date.now(), status === "running");
    const displayModel = sanitizeTerminalText(model ? modelEffort(model, thinking, 28) : "?");
    const title = `${glyph} ${theme.bold(sanitizeTerminalText(label))} ${theme.fg("dim", `· ${displayModel} · ${status} · ${formatTokenUsage(usage)}`)}`;

    const hints = this.composer ? [canSteer ? "Steer" : "Message", "esc cancel"] : ["esc back"];
    if (!this.composer) {
      if (canSteer) hints.push("enter steer", this.stopArmed ? "x again to stop" : "x stop");
      else if (this.deps.canMessage()) hints.push("enter message");
      const tools = keyText("app.tools.expand");
      const thinking = keyText("app.thinking.toggle");
      if (tools) hints.push(`${tools} tools`);
      if (thinking) hints.push(`${thinking} thinking`);
    }
    return {
      lines: [
        truncateToWidth(title, width),
        ...visible,
        ...Array<string>(viewport - visible.length).fill(""),
        ...(this.composer?.render(width) ?? []),
      ],
      footer: this.error ? theme.fg("error", this.error) : theme.fg("dim", hints.join(" · ")),
      hasSpinner: status === "running",
    };
  }
}

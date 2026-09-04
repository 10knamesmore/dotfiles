/**
 * Agent-detail view (navigator level 3): a scrollable full transcript with an
 * inline steering composer.
 *
 * Live vs dead-parent switching happens in the deps handed to this view, not
 * here. The shell always supplies messages parsed from the persisted session
 * file. While the runner still owns the child it also supplies a subscription
 * that re-renders on session events plus steer/stop callbacks.
 */

import { Input, truncateToWidth, type TUI } from "@earendil-works/pi-tui";
import type { SubagentStatus, UsageSummary } from "../../types.js";
import { formatTokenUsage, modelEffort, PLAIN, statusGlyph, type ThemeLike } from "../format.js";
import { sanitizeTerminalText } from "../sanitize.js";
import { messagesToLines, type TranscriptMessage } from "./transcript.js";

interface AgentHeader {
  label: string;
  model: string;
  /** Reasoning effort the child resolved to, shown beside the model. */
  thinking?: string;
  status: SubagentStatus;
  usage: UsageSummary;
}

const MODEL_MAX = 28;

type ComposerKind = "steer" | "message";

interface AgentViewDeps {
  tui: TUI;
  header: () => AgentHeader;
  messages: () => TranscriptMessage[];
  /** True while the child is live and steerable/stoppable this session. */
  live: () => boolean;
  onSteer?: (text: string) => void;
  canMessage?: () => boolean;
  /** Return true only after the new follow-up run was spawned successfully. */
  onMessage?: (text: string) => boolean;
  onSteerUnavailable?: () => void;
  onStop?: () => void;
  /** Subscribe to live session events; returns unsubscribe. Absent for static views. */
  subscribe?: (listener: () => void) => () => void;
}

export class AgentView {
  private scrollOffset = 0;
  private autoScroll = true;
  private stopArmed = false;
  private composer: { input: Input; kind: ComposerKind } | undefined;
  private unsubscribe: (() => void) | undefined;
  private lastWidth = 0;
  private lastViewport = 0;
  private transcriptCache: {
    messages: TranscriptMessage[];
    theme: ThemeLike;
    width: number;
    lines: string[];
  } | undefined;

  constructor(private readonly deps: AgentViewDeps) {
    this.unsubscribe = deps.subscribe?.(() => {
      this.transcriptCache = undefined;
      deps.tui.requestRender();
    });
  }

  dispose(): void {
    this.unsubscribe?.();
    this.unsubscribe = undefined;
  }

  get canSteer(): boolean {
    return !!this.deps.onSteer && this.deps.live() && this.isActive();
  }

  get canMessage(): boolean {
    return !!this.deps.onMessage && !this.deps.live() && !!this.deps.canMessage?.();
  }

  /** Whether a composer currently owns keyboard input, including tab. */
  get composerOpen(): boolean {
    return this.composer !== undefined;
  }

  get canStop(): boolean {
    return this.isStoppable();
  }

  get isStopArmed(): boolean {
    return this.stopArmed;
  }

  private isActive(): boolean {
    const status = this.deps.header().status;
    return status === "running" || status === "pending";
  }

  private isStoppable(): boolean {
    return !!this.deps.onStop && this.deps.live() && this.isActive();
  }

  /**
   * Handle a key. Returns true when consumed (scroll, composer, steer-open,
   * stop); false for keys the navigator should process (such as esc back).
   */
  handleInput(data: string, keyId: string | undefined): boolean {
    if (this.composer) {
      this.composer.input.handleInput(data);
      this.deps.tui.requestRender();
      return true;
    }
    if (keyId === "enter" || keyId === "return") {
      const kind = this.canSteer ? "steer" : this.canMessage ? "message" : undefined;
      if (kind) {
        this.stopArmed = false;
        this.openComposer(kind);
        return true;
      }
    }
    if (keyId === "x") {
      if (this.isStoppable()) {
        if (this.stopArmed) {
          this.stopArmed = false;
          this.deps.onStop?.();
        } else {
          this.stopArmed = true;
        }
        this.deps.tui.requestRender();
        return true;
      }
      return false;
    }
    if (this.stopArmed) this.stopArmed = false;
    const scrolled = this.scroll(keyId);
    if (scrolled) this.deps.tui.requestRender();
    return scrolled;
  }

  private scroll(keyId: string | undefined): boolean {
    const width = Math.max(20, this.lastWidth);
    const messages = this.deps.messages();
    const total = this.transcriptCache?.messages === messages && this.transcriptCache.width === width
      ? this.transcriptCache.lines.length
      : this.transcriptLines(messages, PLAIN, width).length;
    const max = Math.max(0, total - this.lastViewport);
    switch (keyId) {
      case "up":
      case "k":
        this.scrollOffset = Math.max(0, this.scrollOffset - 1);
        this.autoScroll = false;
        return true;
      case "down":
      case "j":
        this.scrollOffset = Math.min(max, this.scrollOffset + 1);
        this.autoScroll = this.scrollOffset >= max;
        return true;
      case "pageup":
      case "shift+up":
      case "shift+k":
        this.scrollOffset = Math.max(0, this.scrollOffset - this.lastViewport);
        this.autoScroll = false;
        return true;
      case "pagedown":
      case "shift+down":
      case "shift+j":
        this.scrollOffset = Math.min(max, this.scrollOffset + this.lastViewport);
        this.autoScroll = this.scrollOffset >= max;
        return true;
      case "home":
        this.scrollOffset = 0;
        this.autoScroll = false;
        return true;
      case "end":
        this.scrollOffset = max;
        this.autoScroll = true;
        return true;
      default:
        return false;
    }
  }

  private openComposer(kind: ComposerKind): void {
    const input = new Input();
    input.focused = true;
    input.onSubmit = (value: string) => {
      const message = value.trim();
      const submitKind = this.canSteer ? "steer" : this.canMessage ? "message" : undefined;
      if (submitKind === "steer") {
        this.composer = undefined;
        if (message) this.deps.onSteer?.(message);
      } else if (submitKind === "message" || kind === "message") {
        // Display eligibility may have lapsed since the composer opened; the
        // send path revalidates authoritatively and surfaces its own error,
        // so a lapsed composer never dead-ends silently.
        if (this.deps.onMessage?.(message)) this.composer = undefined;
      } else {
        this.deps.onSteerUnavailable?.();
      }
      this.deps.tui.requestRender();
    };
    input.onEscape = () => {
      this.composer = undefined;
      this.deps.tui.requestRender();
    };
    this.composer = { input, kind };
    this.deps.tui.requestRender();
  }

  private transcriptLines(messages: TranscriptMessage[], theme: ThemeLike, width: number): string[] {
    if (
      this.transcriptCache?.messages === messages
      && this.transcriptCache.theme === theme
      && this.transcriptCache.width === width
    ) {
      return this.transcriptCache.lines;
    }
    const lines = messagesToLines(messages, theme, width);
    this.transcriptCache = { messages, theme, width, lines };
    return lines;
  }

  /** Render header + transcript window (+ composer). `bodyRows` is the transcript viewport height. */
  render(width: number, bodyRows: number, theme: ThemeLike): string[] {
    const cap = Math.max(20, width);
    this.lastWidth = cap;
    const composerRows = this.composer ? 1 : 0;
    const viewport = Math.max(1, bodyRows - composerRows);
    this.lastViewport = viewport;

    const head = this.deps.header();
    const glyph = statusGlyph(head.status, theme, Date.now(), head.status === "running");
    const label = sanitizeTerminalText(head.label);
    // The tight "·" binds effort to its model; the spaced " · " separates fields.
    const model = sanitizeTerminalText(head.model ? modelEffort(head.model, head.thinking, MODEL_MAX) : "?");
    const title = `${glyph} ${theme.bold(label)} ${theme.fg("dim", `· ${model} · ${head.status} · ${formatTokenUsage(head.usage)}`)}`;
    const lines: string[] = [truncateToWidth(title, cap), truncateToWidth(theme.fg("dim", "─".repeat(cap)), cap)];

    const messages = this.deps.messages();
    const content = this.transcriptLines(messages, theme, cap);
    const max = Math.max(0, content.length - viewport);
    if (this.autoScroll) this.scrollOffset = max;
    const start = Math.min(this.scrollOffset, max);
    for (let i = 0; i < viewport; i += 1) lines.push(content[start + i] ?? "");

    if (this.composer) {
      const kind = this.canSteer ? "steer" : this.canMessage ? "message" : this.composer.kind;
      const label = `✎ ${kind}: `;
      lines.push(truncateToWidth(theme.fg("accent", label) + (this.composer.input.render(Math.max(1, cap - label.length))[0] ?? ""), cap));
    }
    return lines;
  }
}

/** Child conversation browser: typed blocks, folding, stable scrolling, and an inline message composer. */
import { getMarkdownTheme } from "@earendil-works/pi-coding-agent";
import { Input, truncateToWidth, type TUI } from "@earendil-works/pi-tui";
import type { SubagentStatus, UsageSummary } from "../../types.js";
import { formatTokenUsage, modelEffort, PLAIN, statusGlyph, type ThemeLike } from "../format.js";
import { sanitizeTerminalText } from "../sanitize.js";
import { blockCanCollapse, blockIsExpanded, buildTranscriptBlocks, type TranscriptBlock, type TranscriptResult } from "./transcript-blocks.js";
import { renderTranscript, type TranscriptLayout } from "./transcript-render.js";
import type { TranscriptMessage } from "./transcript.js";

interface AgentHeader {
  label: string;
  model: string;
  thinking?: string;
  status: SubagentStatus;
  usage: UsageSummary;
}

type ComposerKind = "steer" | "message";

interface AgentViewDeps {
  tui: TUI;
  header: () => AgentHeader;
  messages: () => TranscriptMessage[];
  /** Persisted terminal output, also available after the child process has exited. */
  result: () => TranscriptResult | undefined;
  live: () => boolean;
  onSteer?: (text: string) => void;
  canMessage?: () => boolean;
  /** True only after a new follow-up run has started successfully. */
  onMessage?: (text: string) => boolean;
  onSteerUnavailable?: () => void;
  onStop?: () => void;
  subscribe?: (listener: () => void) => () => void;
}

interface TranscriptCache {
  messages: TranscriptMessage[];
  resultKey: string;
  live: boolean;
  theme: ThemeLike;
  width: number;
  blocks: TranscriptBlock[];
  layout: TranscriptLayout;
}

export class AgentView {
  private scrollOffset = 0;
  private autoScroll = true;
  private stopArmed = false;
  private composer: { input: Input; kind: ComposerKind } | undefined;
  private unsubscribe: (() => void) | undefined;
  private lastWidth = 20;
  private lastViewport = 1;
  private lastTheme: ThemeLike = PLAIN;
  private transcriptCache: TranscriptCache | undefined;
  private lastLayout: TranscriptLayout | undefined;
  private expanded = new Map<string, boolean>();
  private selectedId: string | undefined;
  private outputOnly = false;
  private fullContentPosition: { blockId?: string; offset: number; scroll: number; follow: boolean; selectedId?: string } | undefined;

  constructor(private readonly deps: AgentViewDeps) {
    this.unsubscribe = deps.subscribe?.(() => {
      this.invalidate();
      deps.tui.requestRender();
    });
  }

  dispose(): void { this.unsubscribe?.(); this.unsubscribe = undefined; }

  /** Keep fold choices and scroll anchors when a new event or theme invalidates the rendered rows. */
  invalidate(): void { this.transcriptCache = undefined; }

  get canSteer(): boolean { return !!this.deps.onSteer && this.deps.live() && this.isActive(); }
  get canMessage(): boolean { return !!this.deps.onMessage && !this.deps.live() && !!this.deps.canMessage?.(); }
  get composerOpen(): boolean { return this.composer !== undefined; }
  get canStop(): boolean { return !!this.deps.onStop && this.deps.live() && this.isActive(); }
  get isStopArmed(): boolean { return this.stopArmed; }

  private isActive(): boolean {
    const status = this.deps.header().status;
    return status === "running" || status === "pending";
  }

  /** The composer owns all input; browser shortcuts never intercept text being typed. */
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
      if (!this.canStop) return false;
      if (this.stopArmed) { this.stopArmed = false; this.deps.onStop?.(); }
      else this.stopArmed = true;
      this.deps.tui.requestRender();
      return true;
    }
    this.stopArmed = false;
    if (keyId === "[" || keyId === "]") {
      this.selectBlock(keyId === "]" ? 1 : -1);
    } else if (keyId === "space" || data === " ") {
      this.toggleSelected();
    } else if (keyId === "t" || keyId === "o") {
      this.toggleKind(keyId === "t" ? "thinking" : "tool");
    } else if (keyId === "f") {
      this.toggleOutputFilter();
    } else if (!this.scroll(keyId)) return false;
    this.deps.tui.requestRender();
    return true;
  }

  private currentTranscript(): TranscriptCache {
    const messages = this.deps.messages();
    const result = this.deps.result();
    const resultKey = result ? JSON.stringify(result) : "";
    const live = this.deps.live();
    const cached = this.transcriptCache;
    if (cached && cached.messages === messages && cached.resultKey === resultKey && cached.live === live
      && cached.theme === this.lastTheme && cached.width === this.lastWidth) return cached;

    const previous = this.lastLayout;
    const anchor = !this.autoScroll ? previous?.blocks.find((block) => block.end > this.scrollOffset) : undefined;
    const offsetInBlock = anchor ? this.scrollOffset - anchor.start : 0;
    const blocks = buildTranscriptBlocks(messages, result, live);
    const present = new Set(blocks.map((block) => block.id));
    for (const id of this.expanded.keys()) if (!present.has(id)) this.expanded.delete(id);
    if (this.selectedId && !present.has(this.selectedId)) this.selectedId = undefined;
    const layout = renderTranscript(blocks, { expanded: this.expanded, selectedId: this.selectedId, outputOnly: this.outputOnly }, this.lastWidth, this.lastTheme, getMarkdownTheme());
    if (anchor) {
      const next = layout.blocks.find((block) => block.id === anchor.id);
      if (next) this.scrollOffset = Math.max(0, next.start + Math.min(offsetInBlock, next.end - next.start - 1));
    }
    this.lastLayout = layout;
    return this.transcriptCache = { messages, resultKey, live, theme: this.lastTheme, width: this.lastWidth, blocks, layout };
  }

  private selectBlock(delta: 1 | -1): void {
    const { layout } = this.currentTranscript();
    if (layout.blocks.length === 0) return;
    let index = layout.blocks.findIndex((block) => block.id === this.selectedId);
    if (index < 0) {
      const visible = layout.blocks.filter((block) => block.end > this.scrollOffset && block.start < this.scrollOffset + this.lastViewport);
      const target = delta > 0 ? visible[0] : visible.at(-1);
      index = Math.max(0, layout.blocks.findIndex((block) => block === target));
    } else index = Math.max(0, Math.min(layout.blocks.length - 1, index + delta));
    const block = layout.blocks[index]!;
    this.selectedId = block.id;
    this.autoScroll = false;
    if (block.start < this.scrollOffset || block.start >= this.scrollOffset + this.lastViewport) this.scrollOffset = block.start;
    this.invalidate();
  }

  private toggleSelected(): void {
    if (!this.selectedId) this.selectBlock(1);
    const block = this.currentTranscript().blocks.find((item) => item.id === this.selectedId);
    if (!block || !blockCanCollapse(block)) return;
    if (this.outputOnly && block.kind === "tool") {
      this.outputOnly = false;
      this.expanded.set(block.id, true);
      this.autoScroll = false;
      this.invalidate();
      const target = this.currentTranscript().layout.blocks.find((item) => item.id === block.id);
      if (target) this.scrollOffset = target.start;
      return;
    }
    this.expanded.set(block.id, !blockIsExpanded(block, this.expanded));
    // Folding is an inspection action, even when the cursor was at the live tail.
    this.autoScroll = false;
    this.invalidate();
  }

  private toggleOutputFilter(): void {
    if (!this.outputOnly) {
      const anchor = this.currentTranscript().layout.blocks.find((block) => block.end > this.scrollOffset);
      this.fullContentPosition = {
        blockId: anchor?.id, offset: anchor ? this.scrollOffset - anchor.start : 0,
        scroll: this.scrollOffset, follow: this.autoScroll, selectedId: this.selectedId,
      };
      this.outputOnly = true;
      this.selectedId = undefined;
      this.invalidate();
      return;
    }
    this.outputOnly = false;
    const saved = this.fullContentPosition;
    this.fullContentPosition = undefined;
    this.selectedId = saved?.selectedId;
    this.invalidate();
    const layout = this.currentTranscript().layout;
    if (saved) {
      this.autoScroll = saved.follow;
      const anchor = layout.blocks.find((block) => block.id === saved.blockId);
      this.scrollOffset = anchor ? Math.max(0, anchor.start + Math.min(saved.offset, anchor.end - anchor.start - 1)) : saved.scroll;
    }
  }

  private toggleKind(kind: "thinking" | "tool"): void {
    if (this.outputOnly) return;
    const blocks = this.currentTranscript().blocks.filter((block) => block.kind === kind);
    if (blocks.length === 0) return;
    const expand = blocks.some((block) => !blockIsExpanded(block, this.expanded));
    for (const block of blocks) this.expanded.set(block.id, expand);
    this.autoScroll = false;
    this.invalidate();
  }

  private scroll(keyId: string | undefined): boolean {
    const max = Math.max(0, this.currentTranscript().layout.lines.length - this.lastViewport);
    switch (keyId) {
      case "up": case "k": this.scrollOffset = Math.max(0, this.scrollOffset - 1); this.autoScroll = false; return true;
      case "down": case "j": this.scrollOffset = Math.min(max, this.scrollOffset + 1); break;
      case "pageup": case "shift+up": case "shift+k": this.scrollOffset = Math.max(0, this.scrollOffset - this.lastViewport); this.autoScroll = false; return true;
      case "pagedown": case "shift+down": case "shift+j": this.scrollOffset = Math.min(max, this.scrollOffset + this.lastViewport); break;
      case "g": this.scrollOffset = 0; this.autoScroll = false; return true;
      case "shift+g": this.scrollOffset = max; this.autoScroll = true; return true;
      default: return false;
    }
    this.autoScroll = this.scrollOffset >= max;
    return true;
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
        if (this.deps.onMessage?.(message)) this.composer = undefined;
      } else this.deps.onSteerUnavailable?.();
      this.deps.tui.requestRender();
    };
    input.onEscape = () => { this.composer = undefined; this.deps.tui.requestRender(); };
    this.composer = { input, kind };
    this.deps.tui.requestRender();
  }

  /** Render within the navigator's total row budget, including header, filter, position and composer. */
  render(width: number, totalRows: number, theme: ThemeLike): string[] {
    const cap = Math.max(20, width);
    this.lastWidth = cap;
    this.lastTheme = theme;
    const viewport = Math.max(1, totalRows - 4 - (this.composer ? 1 : 0));
    this.lastViewport = viewport;
    const head = this.deps.header();
    const glyph = statusGlyph(head.status, theme, Date.now(), head.status === "running");
    const label = sanitizeTerminalText(head.label);
    const model = sanitizeTerminalText(head.model ? modelEffort(head.model, head.thinking, 28) : "?");
    const title = `${glyph} ${theme.bold(label)} ${theme.fg("dim", `· ${model} · ${head.status} · ${formatTokenUsage(head.usage)}`)}`;
    const mode = this.outputOnly ? "Output only" : "All content";
    const filterHint = this.outputOnly ? `${mode} [f] · failed tools remain visible` : `${mode} [f] · t thinking · o tool details`;
    const lines = [title, theme.fg("muted", filterHint), theme.fg("dim", "─".repeat(cap))];
    const { layout } = this.currentTranscript();
    const max = Math.max(0, layout.lines.length - viewport);
    this.scrollOffset = this.autoScroll ? max : Math.min(this.scrollOffset, max);
    for (let index = 0; index < viewport; index += 1) lines.push(layout.lines[this.scrollOffset + index] ?? "");
    const position = this.autoScroll ? this.isActive() ? "Following latest" : "End of transcript" : "Reading history";
    lines.push(theme.fg("muted", `${position} · g top · G latest · [ ] block · Space fold`));
    if (this.composer) {
      const kind = this.canSteer ? "steer" : this.canMessage ? "message" : this.composer.kind;
      const prefix = `✎ ${kind}: `;
      lines.push(theme.fg("accent", prefix) + (this.composer.input.render(Math.max(1, cap - prefix.length))[0] ?? ""));
    }
    return lines.map((line) => truncateToWidth(line, cap));
  }
}

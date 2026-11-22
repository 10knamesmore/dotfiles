import type { ExtensionContext } from "@earendil-works/pi-coding-agent";
import type { OverlayHandle, TUI } from "@earendil-works/pi-tui";
import { sanitizeTodoDisplayLine } from "./display.js";
import { FLOAT_MAX_HEIGHT, TodoFloat } from "./float.js";
import { cloneTodoPhases, isClosedTodo, type TodoPhase } from "./model.js";

const FLOAT_WIDTH = {
  collapsed: { ratio: 0.42, min: 36, max: 60 },
  expanded: { ratio: 0.6, min: 52, max: 96 },
} as const;

function errorMessage(error: unknown): string {
  return sanitizeTodoDisplayLine(error instanceof Error ? error.message : String(error));
}

function floatWidth(columns: number, collapsed: boolean): number {
  const spec = collapsed ? FLOAT_WIDTH.collapsed : FLOAT_WIDTH.expanded;
  return Math.min(spec.max, Math.max(spec.min, Math.round(columns * spec.ratio)));
}

/** Owns the fullscreen todo float and its collapsed/expanded state. */
export class TodoPresentation {
  private phases: TodoPhase[] = [];
  private collapsed = false;
  private tui: TUI | undefined;
  private overlay: OverlayHandle | undefined;
  private dismiss: (() => void) | undefined;
  private mounting = false;
  private overlayUnavailable = false;
  private mountGeneration = 0;

  /** Toggle the float between one active line and the full phase list. */
  public toggleCollapsed(ctx: ExtensionContext): string | undefined {
    try {
      this.ensureOverlay(ctx);
      this.flipCollapsed();
      return undefined;
    } catch (error) {
      return `Todo state is intact, but its UI could not refresh: ${errorMessage(error)}`;
    }
  }

  /** Replace the rendered state; the float follows every todo mutation and branch restore. */
  public refresh(ctx: ExtensionContext, phases: readonly TodoPhase[]): string | undefined {
    this.phases = cloneTodoPhases(phases);
    if (ctx.mode !== "tui") return undefined;
    try {
      this.ensureOverlay(ctx);
      this.syncVisibility();
      this.tui?.requestRender();
      return undefined;
    } catch (error) {
      return `Todo state is intact, but its UI could not refresh: ${errorMessage(error)}`;
    }
  }

  /** Release the overlay interaction. Safe when nothing was mounted. */
  public dispose(): void {
    this.mountGeneration += 1;
    this.dismiss?.();
    this.dismiss = undefined;
    this.overlay = undefined;
    this.tui = undefined;
    this.phases = [];
    this.collapsed = false;
    this.mounting = false;
    this.overlayUnavailable = false;
  }

  private ensureOverlay(ctx: ExtensionContext): void {
    if (this.overlay || this.mounting || this.overlayUnavailable) return;
    this.mounting = true;
    const generation = ++this.mountGeneration;

    const interaction = ctx.ui.custom<void>(
      (tui, theme, _keybindings, done) => {
        if (tui.mode !== "fullscreen") {
          this.overlayUnavailable = true;
          done();
          return { render: () => [], invalidate: () => {} };
        }
        this.tui = tui;
        this.dismiss = () => done();
        return new TodoFloat(theme, {
          readPhases: () => this.phases,
          isCollapsed: () => this.collapsed,
          toggleCollapsed: () => this.flipCollapsed(),
          terminalRows: () => tui.terminal.rows,
        });
      },
      {
        overlay: true,
        overlayOptions: () => {
          const presentation = this;
          return {
            anchor: "top-right",
            // The overlay keeps this object and reads width on every render, so the
            // getter switches between compact and expanded widths without a remount.
            get width() {
              return floatWidth(
                presentation.tui!.terminal.columns,
                presentation.collapsed || presentation.allTasksClosed(),
              );
            },
            maxHeight: FLOAT_MAX_HEIGHT,
            margin: 1,
            nonCapturing: true,
          };
        },
        onHandle: (handle) => {
          if (generation !== this.mountGeneration) return;
          this.overlay = handle;
          this.mounting = false;
          this.syncVisibility();
          this.tui?.requestRender();
        },
      },
    );

    void interaction
      .catch((error: unknown) => {
        if (generation !== this.mountGeneration) return;
        this.overlayUnavailable = true;
        ctx.ui.notify(`Todo float could not open: ${errorMessage(error)}`, "warning");
      })
      .finally(() => {
        if (generation === this.mountGeneration) this.mounting = false;
      });
  }

  private syncVisibility(): void {
    this.overlay?.setHidden(!this.phases.some((phase) => phase.tasks.length > 0));
  }

  private allTasksClosed(): boolean {
    const tasks = this.phases.flatMap((phase) => phase.tasks);
    return tasks.length > 0 && tasks.every(isClosedTodo);
  }

  private flipCollapsed(): void {
    this.collapsed = !this.collapsed;
    this.tui?.requestRender();
  }
}

import { ScrollView, type Component } from "@earendil-works/pi-tui";
import type { ContextAppearance } from "./presentation.js";
import type { SourceLayouts, SourceDocument, SourceViewport } from "./source-layout.js";

export class SourceContent implements Component {
  public readonly scroll = new ScrollView(this, { scrollbar: "hidden" });
  private viewport: SourceViewport | undefined;
  private contentHeight: number | undefined;
  private viewportKey: string | undefined;
  private revision = 0;
  private requestedRevision = -1;
  private decoratedWidth = 0;
  private decoratedRows = new Map<number, string>();

  public constructor(
    private readonly documentId: number,
    private readonly document: SourceDocument,
    private readonly layouts: SourceLayouts,
    private readonly appearance: ContextAppearance,
    private readonly requestRender: () => void,
  ) {}

  public get position(): string {
    if (this.contentHeight === undefined) return "preparing";
    const start = this.scroll.scrollTop;
    const end = Math.min(start + this.scroll.viewportHeight, this.contentHeight);
    return `${start + 1}–${end}/${this.contentHeight}`;
  }

  public renderViewport(width: number, height: number): string[] {
    this.scroll.updateLayout(this.contentHeight ?? height, height, this.requestRender);
    return this.scroll.render(width);
  }

  public render(width: number): string[] {
    const scrollTop = this.scroll.scrollTop;
    const height = this.scroll.viewportHeight;
    const key = `${width}:${scrollTop}:${height}`;
    // A cached viewport is still a new intent: an older resize must not clamp its scroll position.
    if (this.viewportKey !== key) {
      this.viewportKey = key;
      this.revision++;
    }
    const viewport = this.viewport;
    const end = Math.min(scrollTop + height, this.contentHeight ?? scrollTop + height);
    const bufferEnd = viewport ? viewport.firstRow + viewport.lines.length : 0;
    const ready =
      viewport !== undefined && viewport.width === width && viewport.firstRow <= scrollTop && bufferEnd >= end;
    const margin = Math.floor(height / 2);
    const needsBuffer =
      !ready ||
      viewport.firstRow > Math.max(0, scrollTop - margin) ||
      bufferEnd < Math.min(end + margin, viewport.contentHeight);
    if (needsBuffer && this.requestedRevision !== this.revision) {
      const revision = this.revision;
      this.requestedRevision = revision;
      this.layouts.request({ documentId: this.documentId, width, scrollTop, height }, this.document, (viewport) => {
        if (revision !== this.revision || this.scroll.scrollTop !== scrollTop) return;
        this.viewport = viewport;
        this.contentHeight = viewport.contentHeight;
        this.scroll.updateLayout(viewport.contentHeight, viewport.height, this.requestRender);
        this.viewportKey = `${viewport.width}:${this.scroll.scrollTop}:${viewport.height}`;
        this.requestRender();
      });
    }
    if (!ready) {
      return Array.from({ length: Math.min(height, this.contentHeight ?? height) }, (_, index) =>
        this.appearance.fill(index === 0 ? this.appearance.theme.fg("muted", "Preparing source…") : "", width),
      );
    }
    if (this.decoratedWidth !== width) this.decoratedRows.clear();
    this.decoratedWidth = width;
    const nextDecoratedRows = new Map<number, string>();
    const lines = viewport.lines
      .slice(scrollTop - viewport.firstRow, end - viewport.firstRow)
      .map(({ text, sourceLine }, offset) => {
        const row = scrollTop + offset;
        let decorated = this.decoratedRows.get(row);
        if (decorated === undefined) {
          const number = sourceLine === undefined ? "" : String(sourceLine);
          const gutter = this.appearance.theme.fg("dim", `${number.padStart(viewport.gutterWidth)} │ `);
          decorated = this.appearance.fill(gutter + text, width);
        }
        nextDecoratedRows.set(row, decorated);
        return decorated;
      });
    this.decoratedRows = nextDecoratedRows;
    return lines;
  }

  public invalidate(): void {
    this.revision++;
    this.viewport = undefined;
    this.invalidateAppearance();
  }

  public invalidateAppearance(): void {
    this.decoratedRows.clear();
  }
}

import { performance } from "node:perf_hooks";
import { truncateToWidth, visibleWidth } from "@earendil-works/pi-tui";
import type { EditorActivity } from "../../editor/api.js";
import { sanitizeFooterText } from "../format.js";
import { palette } from "../palette.js";
import { APPEARANCE, CAT_COLUMNS, colorCat, ICONS } from "./art.js";
import { CatMotion } from "./motion.js";

const PET_WIDTH = 24;

function padRow(line: string, width: number): string {
  const fitted = truncateToWidth(line, width, palette.overlay2("…"));
  return fitted + " ".repeat(width - visibleWidth(fitted));
}

/** Footer-owned cat: movement stays inside its reserved area; activity labels never move. */
export class FooterPet {
  private readonly motion = new CatMotion();
  private timer: ReturnType<typeof setTimeout> | undefined;
  private scheduledChangeAt: number | undefined;

  public constructor(private readonly requestRender: () => void) {}

  /** Full cat at 80+ columns; narrower windows retain its moving face and activity on the bottom row. */
  public render(activity: EditorActivity, fastRequested: boolean, width: number): string[] {
    const frame = this.motion.sample(activity.kind);
    this.scheduleRepaint(frame.nextChangeAt);
    const appearance = APPEARANCE[activity.kind];
    const petWidth = Math.min(PET_WIDTH, width);
    // The badge reports the requested mode, not the server's delivered service tier.
    const fast = fastRequested ? palette.yellow(` ${ICONS.bolt}`) : "";
    if (width < 80) {
      const label = appearance.color(activity.kind);
      const face = colorCat(frame.face.padEnd(CAT_COLUMNS), activity.kind);
      const withFace = face + label;
      const body = visibleWidth(withFace + fast) <= petWidth ? withFace : label;
      const line = truncateToWidth(body, Math.max(0, petWidth - visibleWidth(fast)), "") + fast;
      return ["", "", padRow(line, petWidth)];
    }

    const detailWidth = PET_WIDTH - CAT_COLUMNS;
    const extraTools = activity.kind === "tool" && activity.toolCount > 1 ? ` +${activity.toolCount - 1}` : "";
    const detail =
      activity.kind === "tool"
        ? truncateToWidth(sanitizeFooterText(activity.toolName), detailWidth - visibleWidth(extraTools)) + extraTools
        : appearance.detail;
    const labels = [appearance.color(`${appearance.icon} ${activity.kind}`) + fast, palette.overlay2(detail), ""];
    return frame.lines.map((line, index) => padRow(colorCat(line, activity.kind) + labels[index], petWidth));
  }

  /** Reload, session replacement, and shutdown all dispose the containing footer. */
  public dispose(): void {
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.timer = undefined;
    this.scheduledChangeAt = undefined;
  }

  private scheduleRepaint(changeAt: number): void {
    if (this.timer !== undefined && this.scheduledChangeAt === changeAt) return;
    if (this.timer !== undefined) clearTimeout(this.timer);
    this.scheduledChangeAt = changeAt;
    this.timer = setTimeout(() => {
      this.timer = undefined;
      this.scheduledChangeAt = undefined;
      this.requestRender();
    }, Math.max(1, changeAt - performance.now()));
  }
}

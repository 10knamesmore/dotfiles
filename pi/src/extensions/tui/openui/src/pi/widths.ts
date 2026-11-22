// Make our idea of which characters are two cells wide agree with the pi that is running, not only with the pi the committed table
// (src/diagram/wide.ts) was generated from. Unicode adds emoji with every release, and a character we count as one cell that pi
// draws as two slides the rest of its row and pushes the right border out of line.
import { visibleWidth } from "@earendil-works/pi-tui";
import { setWide } from "../diagram/canvas.ts";
import { WIDE } from "../diagram/wide.ts";
import { healWide } from "../diagram/widefix.ts";

/** Ask pi about the blocks where new wide characters appear (see PROBE_SPANS) and use its answers. Never throws: on a failure the committed table stays. */
export function syncWidths(): void {
  try {
    setWide(healWide(WIDE, (cp) => visibleWidth(String.fromCodePoint(cp))));
  } catch {
    setWide(WIDE);
  }
}

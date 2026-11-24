import { performance } from "node:perf_hooks";
import { palette, paletteColors } from "@dotfiles/pi-ui/palette";
import type { EditorActivity } from "@dotfiles/pi-ui/prompt-editor";
import { foregroundAnsi, mixColors, rgbColor, visibleWidth, type Color } from "@earendil-works/pi-tui";
import { THINKING_WORDS } from "./thinking-words.js";

const ACTIVITY_COLORS = {
  waiting: paletteColors.lavender,
  thinking: paletteColors.mauve,
  writing: paletteColors.sky,
  compacting: paletteColors.yellow,
  tool: paletteColors.peach,
};

const graphemes = new Intl.Segmenter();
const HIGHLIGHT = rgbColor(248, 248, 255);

function cycleLabels(labels: readonly string[]): () => string {
  let index = 0;
  return () => {
    const label = labels[index]!;
    index = (index + 1) % labels.length;
    return label;
  };
}

function foregroundSweep(text: string, color: Color, elapsedMs: number): string {
  const fadeWidth = 6;
  const center = (((elapsedMs * 12) / 1_000) % (visibleWidth(text) + 2 * fadeWidth)) - fadeWidth;
  let column = 0;
  let result = "";
  for (const { segment } of graphemes.segment(text)) {
    const width = visibleWidth(segment);
    const distance = column + (width - 1) / 2 - center;
    const strength = Math.exp(-(distance * distance) / 8);
    result += foregroundAnsi(mixColors(color, HIGHLIGHT, strength, "srgb"), "truecolor") + segment;
    column += width;
  }
  return `${result}\x1b[39m`;
}

export function createActivityLabel(): (activity: EditorActivity) => string {
  const nextLabel = {
    ready: cycleLabels([
      "Ready",
      "All set",
      "All ears",
      "Shoot",
      "Prompt me",
      "Yes, hello",
      "Go ahead, over",
      "Keys at the ready",
      "Got a story?",
      "Do tell",
      "Cue me",
      "What's up?",
      "Your move, human",
      "I'm listening",
    ]),
    waiting: cycleLabels([
      "Waiting",
      "Waiting for tokens",
      "Waiting on the model",
      "Any token now",
      "On hold",
      "Awaiting stdout",
      "Observing the silence",
      "Standing by",
      "Fingers hovering",
      "Awaiting copy",
      "Holding that thought",
      "Awaiting my cue",
      "Still here",
      "Blocking politely",
      "Tick, tick, tick",
    ]),
    thinking: cycleLabels(THINKING_WORDS),
    writing: cycleLabels([
      "Writing",
      "Composing",
      "Drafting",
      "Clackety-clack",
      "Tap tap tap",
      "Words on stdout",
      "Putting it in writing",
      "On the air",
      "Clickety-clack",
      "Filing the story",
      "I have words",
      "Delivering my lines",
      "Hear me out",
      "Emitting prose",
      "One word, then another",
    ]),
    compacting: cycleLabels([
      "Compacting",
      "Tidying context",
      "Making room in context",
      "Making a long story short",
      "The gist",
      "Less history, more room",
      "In fewer words",
      "The short version",
      "Saving paper",
      "Cutting to fit",
      "I'll be brief",
      "One-act version",
      "Long story short",
      "TL;DR in progress",
      "Keeping the good bits",
    ]),
  };
  let previous: EditorActivity["kind"] | undefined;
  let startedAt = 0;
  let label = "";

  return (activity) => {
    const now = performance.now();
    if (previous !== activity.kind) {
      previous = activity.kind;
      startedAt = now;
      if (activity.kind !== "tool") label = nextLabel[activity.kind]();
    }
    if (activity.kind === "ready") return palette.green(label);

    const text =
      activity.kind === "tool"
        ? activity.label.replace(/^[A-Z]+(?= )/u, (word) => word[0]! + word.slice(1).toLowerCase())
        : label;
    return foregroundSweep(text, ACTIVITY_COLORS[activity.kind], now - startedAt);
  };
}

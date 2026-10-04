import type { EditorActivity } from "../../editor/api.js";
import { palette } from "../palette.js";

/** All sprite glyphs are single-column ASCII or Nerd Font Mono characters. */
export const ICONS = {
  bolt: "\uf0e7",
  clock: "\uf017",
  bulb: "\uf0eb",
  keyboard: "\uf11c",
  terminal: "\uf120",
  wrench: "\uf0ad",
  archive: "\uf187",
  paper: "\uf15c",
  heart: "\uf004",
  star: "\uf005",
} as const;

export const CAT_COLUMNS = 10;
export type ActivityKind = EditorActivity["kind"];
/** Ordered around the cat by the motion module; rear views hide the face unless it looks back. */
export type CatFacing = "front" | "front-left" | "left" | "back-left" | "back" | "back-right" | "right" | "front-right";
type Posture = "sit" | "crouch" | "hop" | "peek" | "curl" | "roll";
type PawPose = "rest" | "open" | "left-up" | "right-up" | "step-left" | "step-right" | "hold";
type EarPose = "up" | "left-down" | "right-down" | "flat";

/** Body and face move together; blinking and the tail are layered on separately. */
export interface CatPose {
  body: { column: number; posture: Posture; paws: PawPose; facing: CatFacing };
  face: { ears: EarPose; eyes: readonly [string, string]; mouth: string; lookBack: boolean };
  prop: { glyph: string; location: "paws" | "left-paw" | "right-paw" | "above" } | undefined;
  thought: string;
}

/** A gesture changes only the named joints, retaining the rest of its preceding pose. */
export type PoseChange = Partial<Pick<CatPose, "prop" | "thought">> & {
  body?: Partial<CatPose["body"]>;
  face?: Partial<CatPose["face"]>;
};

export interface TailPose {
  glyph: string;
  row: 1 | 2;
}

export const APPEARANCE = {
  ready: { icon: ICONS.heart, color: palette.green, detail: "Your turn" },
  waiting: { icon: ICONS.clock, color: palette.yellow, detail: "Awaiting reply" },
  thinking: { icon: ICONS.bulb, color: palette.mauve, detail: "Pondering..." },
  writing: { icon: ICONS.keyboard, color: palette.sky, detail: "Streaming" },
  tool: { icon: ICONS.terminal, color: palette.peach, detail: "" },
  compacting: { icon: ICONS.archive, color: palette.lavender, detail: "Summarizing" },
} satisfies Record<ActivityKind, { icon: string; color: (text: string) => string; detail: string }>;

/** Return to the current job while retaining the cat's position and orientation. */
export function restingPose(kind: ActivityKind, column: number, facing: CatFacing = "front"): CatPose {
  const prop =
    kind === "writing" ? ICONS.keyboard :
    kind === "tool" ? ICONS.wrench :
    kind === "compacting" ? ICONS.archive : undefined;
  return {
    body: { column, posture: "sit", paws: prop ? "hold" : "rest", facing },
    face: {
      ears: "up",
      lookBack: false,
      eyes: kind === "ready" ? ["^", "^"] : kind === "thinking" ? ["-", "-"] : ["o", "o"],
      mouth: ".",
    },
    prop: prop ? { glyph: prop, location: "paws" } : undefined,
    thought: "",
  };
}

export function changePose(pose: CatPose, change: PoseChange): CatPose {
  return {
    ...pose,
    ...change,
    body: { ...pose.body, ...change.body },
    face: { ...pose.face, ...change.face },
  };
}

const EARS: Record<EarPose, string> = {
  up: " /\\_/\\",
  "left-down": " /\\_,\\",
  "right-down": " ,/_/\\",
  flat: " ,___,",
};
const PAWS: Record<PawPose, string> = {
  rest: '(")_(")',
  open: "o/   \\o",
  "left-up": "o/___\\ ",
  "right-up": " /___\\o",
  "step-left": "o)__(o)",
  "step-right": "(o)__o)",
  hold: " / >_<\\",
};

const SIDE_PAWS: Record<PawPose, string> = {
  rest: " oo_( )",
  open: "o/   \\o",
  "left-up": "o/__( )",
  "right-up": " o/_( )",
  "step-left": "o o_( )",
  "step-right": " oo_(_)",
  hold: " o>_( )",
};
const BACK_PAWS: Record<PawPose, string> = {
  rest: "( )_( )",
  open: "o/___\\o",
  "left-up": "o|___| ",
  "right-up": " |___|o",
  "step-left": "o)___( ",
  "step-right": " )___(o",
  hold: " /|_|\\ ",
};
const MIRRORED_GLYPHS: Record<string, string> = {
  "(": ")", ")": "(", "<": ">", ">": "<", "/": "\\", "\\": "/",
};

function mirrorSprite(sprite: string): string {
  return [...sprite].reverse().map((char) => MIRRORED_GLYPHS[char] ?? char).join("");
}

interface CatView {
  ears: string;
  face: string;
  paws: string;
  /** Tail and props stay attached to the visible side, rather than rotating around a fixed face. */
  tail: { column: number; mirrored: boolean };
  propColumns: Record<NonNullable<CatPose["prop"]>["location"], number | undefined>;
}

/** Rear silhouettes have no eyes; looking over a shoulder exposes only the nearer eye. */
function viewFromAngle(pose: CatPose, eyes: readonly [string, string]): CatView {
  const { facing, paws } = pose.body;
  const { mouth, ears: earPose, lookBack } = pose.face;
  const front = {
    ears: EARS[earPose],
    face: `(=${eyes[0]}${mouth}${eyes[1]}=)`,
    paws: PAWS[paws],
    tail: { column: 7, mirrored: false },
    propColumns: { paws: 4, "left-paw": 0, "right-paw": 7, above: 7 },
  };
  switch (facing) {
    case "front":
      return front;
    case "front-left":
    case "front-right": {
      const right = facing === "front-right";
      const ears = earPose === "flat" ? " ,___, " : earPose === "up" ? " /\\_/| " : " /\\_,| ";
      return {
        ears: right ? mirrorSprite(ears) : ears,
        face: right ? `(=${eyes[0]}${mouth}${eyes[1]}>)` : `(<${eyes[0]}${mouth}${eyes[1]}=)`,
        paws: right ? mirrorSprite(SIDE_PAWS[paws]) : SIDE_PAWS[paws],
        tail: { column: right ? 0 : 7, mirrored: right },
        propColumns: right
          ? { paws: 5, "left-paw": 5, "right-paw": 7, above: 7 }
          : { paws: 2, "left-paw": 0, "right-paw": 2, above: 0 },
      };
    }
    case "left":
    case "right": {
      const right = facing === "right";
      const ears = earPose === "flat" ? " ,___  " : earPose === "up" ? " /\\__  " : " ,/___ ";
      return {
        ears: right ? mirrorSprite(ears) : ears,
        face: right ? `(   ${eyes[1]}${mouth}>` : `<${mouth}${eyes[0]}   )`,
        paws: right ? mirrorSprite(SIDE_PAWS[paws]) : SIDE_PAWS[paws],
        tail: { column: right ? 0 : 7, mirrored: right },
        propColumns: right
          ? { paws: 5, "left-paw": 5, "right-paw": 7, above: 7 }
          : { paws: 2, "left-paw": 0, "right-paw": 2, above: 0 },
      };
    }
    case "back-left":
    case "back-right": {
      const right = facing === "back-right";
      const ears = earPose === "flat" ? " ,___, " : earPose === "up" ? " /|_/\\ " : " ,|_/\\ ";
      return {
        ears: right ? mirrorSprite(ears) : ears,
        face: lookBack
          ? right ? `(   ${eyes[1]}>)` : `(<${eyes[0]}   )`
          : right ? "(   \\ )" : "( /   )",
        paws: right ? mirrorSprite(BACK_PAWS[paws]) : BACK_PAWS[paws],
        tail: { column: right ? 1 : 5, mirrored: right },
        propColumns: right
          ? { paws: 7, "left-paw": undefined, "right-paw": 7, above: 7 }
          : { paws: 0, "left-paw": 0, "right-paw": undefined, above: 0 },
      };
    }
    case "back":
      return {
        ears: EARS[earPose],
        face: "(     )",
        paws: BACK_PAWS[paws],
        tail: { column: 3, mirrored: false },
        // A held object and the farther paw are occluded by the cat's back.
        propColumns: { paws: undefined, "left-paw": 0, "right-paw": 7, above: 7 },
      };
  }
}

/** Paint into a fixed ten-by-three cell area, never into the neighboring footer text. */
export function drawCat(pose: CatPose, eyesClosed: boolean, tail: TailPose): { lines: string[]; face: string } {
  const rows = Array.from({ length: 3 }, () => Array<string>(CAT_COLUMNS).fill(" "));
  const put = (row: number, column: number, text: string): void => {
    for (const char of text) {
      if (column >= 0 && column < CAT_COLUMNS) rows[row]![column] = char;
      column += 1;
    }
  };
  const { column, posture, facing } = pose.body;
  const eyes: readonly [string, string] = eyesClosed ? ["-", "-"] : pose.face.eyes;
  const view = viewFromAngle(pose, eyes);
  const tailGlyph = view.tail.mirrored ? mirrorSprite(tail.glyph) : tail.glyph;
  const liftedFace = facing === "front" ? `o(${eyes[0]}${pose.face.mouth}${eyes[1]})o` : view.face;

  switch (posture) {
    case "sit":
      put(0, column, view.ears);
      put(1, column, view.face);
      put(2, column, view.paws);
      put(tail.row, column + view.tail.column, tailGlyph);
      break;
    case "hop":
      put(0, column, view.ears);
      put(1, column, liftedFace);
      put(2, column + 1, " . . ");
      put(1, column + view.tail.column, tailGlyph);
      break;
    case "peek":
      put(1, column, view.ears);
      put(2, column, liftedFace);
      break;
    case "crouch":
    case "curl":
      put(1, column, view.ears);
      put(2, column, view.face);
      put(2, column + view.tail.column, posture === "curl" ? (view.tail.mirrored ? "(" : ")") : tailGlyph);
      break;
    case "roll":
      put(1, column, " o   o ");
      put(2, column, view.face);
      put(2, column + view.tail.column, tailGlyph);
      break;
  }

  if (pose.prop && posture === "sit") {
    const { glyph, location } = pose.prop;
    const propColumn = view.propColumns[location];
    if (propColumn !== undefined) {
      put(location === "paws" ? 2 : location === "above" ? 0 : 1, column + propColumn, glyph);
    }
  }
  if (pose.thought) put(0, CAT_COLUMNS - 1, pose.thought);
  return { lines: rows.map((row) => row.join("")), face: " ".repeat(column) + view.face };
}

const PROP_GLYPHS = new Set<string>(Object.values(ICONS));

export function colorCat(line: string, kind: ActivityKind): string {
  return [...line]
    .map((char) => {
      if (char === ICONS.star || char === ICONS.bulb) return palette.yellow(char);
      return PROP_GLYPHS.has(char) ? palette.lavender(char) : APPEARANCE[kind].color(char);
    })
    .join("");
}

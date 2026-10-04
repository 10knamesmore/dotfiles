import { performance } from "node:perf_hooks";
import {
  changePose, drawCat, ICONS, restingPose,
  type ActivityKind, type CatFacing, type CatPose, type PoseChange, type TailPose,
} from "./art.js";

interface MotionBeat {
  /** Nominal duration; each performance also varies its tempo and individual holds. */
  milliseconds: number;
  change: PoseChange;
}

const beat = (milliseconds: number, change: PoseChange): MotionBeat => ({ milliseconds, change });
const between = (minimum: number, maximum: number): number => minimum + Math.random() * (maximum - minimum);
const pick = <T>(items: readonly T[]): T => items[Math.floor(Math.random() * items.length)]!;
const repetitions = (minimum: number, maximum: number): number => Math.floor(between(minimum, maximum + 1));

function otherColumn(pose: CatPose): number {
  return pick([0, 1, 2].filter((column) => column !== pose.body.column));
}

const FACINGS: readonly CatFacing[] = [
  "front", "front-left", "left", "back-left", "back", "back-right", "right", "front-right",
];

/** Turn through neighboring views rather than cutting directly between front and back. */
function turnTo(from: CatFacing, to: CatFacing): MotionBeat[] {
  const start = FACINGS.indexOf(from);
  const distance = (FACINGS.indexOf(to) - start + FACINGS.length) % FACINGS.length;
  const direction = distance === 4 ? pick([-1, 1]) : distance < 4 ? 1 : -1;
  const steps = Math.min(distance, FACINGS.length - distance);
  return Array.from({ length: steps }, (_, index) => beat(between(130, 210), {
    body: { facing: FACINGS[(start + direction * (index + 1) + FACINGS.length) % FACINGS.length]! },
    face: { lookBack: false },
  }));
}

function walk(pose: CatPose, destination = otherColumn(pose)): MotionBeat[] {
  const direction = Math.sign(destination - pose.body.column);
  const eye = direction < 0 ? "<" : ">";
  const heading: CatFacing = direction < 0 ? "left" : "right";
  const arrived: CatFacing = direction < 0 ? "front-left" : "front-right";
  const steps = [
    ...turnTo(pose.body.facing, heading),
    beat(220, { body: { posture: "sit" }, face: { eyes: [eye, eye], ears: direction < 0 ? "left-down" : "right-down" } }),
  ];
  for (let column = pose.body.column + direction; column !== destination + direction; column += direction) {
    steps.push(
      beat(160, { body: { column, paws: "step-left" }, face: { ears: "up" } }),
      beat(190, { body: { paws: "step-right" }, face: { mouth: "w" } }),
    );
  }
  steps.push(
    beat(250, { body: { paws: "rest" }, face: { eyes: ["o", "o"], mouth: "." } }),
    ...turnTo(heading, arrived),
    beat(400, { face: { eyes: ["^", "^"] } }),
  );
  return steps;
}

function typing(pose: CatPose, lean: boolean): MotionBeat[] {
  const keyboard = { glyph: ICONS.keyboard, location: "paws" } as const;
  const steps = [beat(220, { body: { paws: "hold" }, prop: keyboard, face: { eyes: ["o", "o"] } })];
  const strokes = repetitions(3, 6);
  for (let index = 0; index < strokes; index += 1) {
    steps.push(
      beat(between(90, 180), { body: { paws: "left-up" }, face: { mouth: index % 2 ? "o" : "." } }),
      beat(between(100, 210), { body: { paws: "right-up" } }),
    );
  }
  if (lean) steps.splice(3, 0, beat(220, { body: { column: otherColumn(pose) }, face: { ears: "left-down" } }));
  steps.push(beat(400, { body: { paws: "hold" }, face: { ears: "up", eyes: ["^", "^"] } }));
  return steps;
}

/** Short performances, not loops: the director chooses another one after each settling pause. */
const GESTURES = {
  glance: (pose: CatPose) => {
    const left = Math.random() < 0.5;
    const eye = left ? "<" : ">";
    const opposite = left ? ">" : "<";
    const quarter: CatFacing = left ? "front-left" : "front-right";
    const profile: CatFacing = left ? "left" : "right";
    const otherQuarter: CatFacing = left ? "front-right" : "front-left";
    return [
      ...turnTo(pose.body.facing, quarter),
      beat(350, { face: { eyes: [eye, eye] } }),
      beat(180, { face: { ears: left ? "left-down" : "right-down" } }),
      ...turnTo(quarter, profile),
      beat(between(450, 900), { face: { ears: "up" } }),
      ...turnTo(profile, otherQuarter),
      beat(300, { face: { eyes: [opposite, opposite] } }),
      ...turnTo(otherQuarter, "front"),
      beat(220, { face: { eyes: ["o", "o"] } }),
    ];
  },
  ears: (_pose: CatPose) => [
    beat(170, { face: { ears: "left-down" } }),
    beat(260, { face: { ears: "up", eyes: ["o", "o"] } }),
    beat(130, { face: { ears: "right-down" } }),
    beat(190, { face: { ears: "flat", eyes: ["-", "-"] } }),
    beat(420, { face: { ears: "up", eyes: ["^", "^"] } }),
  ],
  wander: walk,
  peek: (pose: CatPose) => [
    beat(250, { body: { posture: "crouch" }, prop: undefined, face: { eyes: ["o", "o"] } }),
    beat(420, { body: { posture: "peek", column: otherColumn(pose) }, face: { eyes: [">", ">"] } }),
    beat(450, { face: { eyes: ["<", "<"], ears: "left-down" } }),
    beat(180, { face: { eyes: ["-", "-"], ears: "up" } }),
    beat(300, { body: { posture: "sit" }, face: { eyes: ["^", "^"] } }),
  ],
  stretch: (_pose: CatPose) => [
    beat(300, { body: { posture: "crouch" }, prop: undefined, face: { eyes: ["-", "-"] } }),
    beat(420, { body: { posture: "sit", paws: "open" }, face: { mouth: "o", ears: "flat" } }),
    beat(between(500, 1000), { face: { eyes: ["^", "^"], ears: "up" } }),
    beat(300, { body: { paws: "left-up" }, face: { mouth: "." } }),
    beat(280, { body: { paws: "right-up" } }),
  ],
  groom: (_pose: CatPose) => {
    const paw = pick(["left-up", "right-up"] as const);
    return [
      beat(260, { body: { paws: paw }, prop: undefined, face: { eyes: ["-", "-"], mouth: "w" } }),
      beat(180, { face: { mouth: ".", ears: "flat" } }),
      beat(220, { face: { mouth: "w", ears: "up" } }),
      beat(160, { face: { mouth: ".", ears: "flat" } }),
      beat(350, { body: { paws: "rest" }, face: { eyes: ["^", "^"], ears: "up" } }),
    ];
  },
  wave: (_pose: CatPose) => [
    beat(180, { body: { paws: "right-up" }, prop: undefined, face: { eyes: ["^", "^"], mouth: "w" } }),
    beat(150, { body: { paws: "open" } }),
    beat(180, { body: { paws: "right-up" } }),
    beat(150, { body: { paws: "open" }, thought: ICONS.heart }),
    beat(450, { body: { paws: "rest" }, thought: "" }),
  ],
  hop: (pose: CatPose) => [
    beat(240, { body: { posture: "crouch" }, prop: undefined, face: { eyes: [">", "<"] } }),
    beat(160, { body: { posture: "hop", column: otherColumn(pose) }, face: { eyes: ["^", "^"], mouth: "o" } }),
    beat(170, { body: { posture: "crouch" }, face: { eyes: ["-", "-"], mouth: "." } }),
    beat(420, { body: { posture: "sit" }, face: { eyes: ["^", "^"] }, thought: ICONS.star }),
  ],
  curl: (_pose: CatPose) => [
    beat(500, { face: { eyes: ["-", "-"], mouth: "o", ears: "flat" } }),
    beat(600, { body: { posture: "crouch" }, prop: undefined, face: { mouth: "." } }),
    beat(between(1200, 2200), { body: { posture: "curl" }, thought: "z" }),
    beat(500, { thought: "Z" }),
    beat(220, { thought: "", face: { eyes: ["o", "o"], ears: "up" } }),
    beat(300, { body: { posture: "sit", paws: "open" } }),
  ],
  roll: (pose: CatPose) => [
    beat(300, { body: { posture: "crouch" }, prop: undefined, face: { eyes: ["^", "^"] } }),
    beat(240, { body: { posture: "roll" }, face: { eyes: ["x", "x"], mouth: "w" } }),
    beat(240, { body: { column: otherColumn(pose) }, face: { eyes: ["o", "o"] } }),
    beat(300, { body: { posture: "curl" }, face: { eyes: ["-", "-"] } }),
    beat(500, { body: { posture: "sit" }, face: { eyes: ["^", "^"], mouth: "." } }),
  ],
  chaseTail: (pose: CatPose) => {
    const destination = otherColumn(pose);
    const arrived: CatFacing = destination < pose.body.column ? "front-left" : "front-right";
    const start = FACINGS.indexOf(arrived);
    const direction = pick([-1, 1]);
    const spin = Array.from({ length: FACINGS.length }, (_, index) => beat(between(100, 160), {
      body: {
        facing: FACINGS[(start + direction * (index + 1) + FACINGS.length) % FACINGS.length]!,
        paws: index % 2 ? "step-right" : "step-left",
      },
      face: { eyes: ["o", "o"], mouth: "o" },
    }));
    return [
      beat(320, { face: { eyes: [">", ">"], ears: "right-down" } }),
      beat(200, { body: { posture: "crouch" }, prop: undefined }),
      ...walk(pose, destination),
      ...spin,
      beat(300, { body: { paws: "rest" }, face: { eyes: ["x", "x"] } }),
      ...turnTo(arrived, "front"),
      beat(180, { body: { posture: "hop" }, face: { eyes: ["^", "^"], mouth: "o" } }),
      beat(400, { body: { posture: "sit", paws: "rest" }, face: { mouth: "." } }),
    ];
  },
  turnAway: (pose: CatPose) => [
    ...turnTo(pose.body.facing, "back"),
    beat(between(800, 1600), { body: { paws: "rest" }, prop: undefined, thought: "" }),
    beat(200, { face: { ears: "left-down" } }),
    beat(220, { face: { ears: "right-down" } }),
    beat(600, { face: { ears: "up" } }),
  ],
  lookOverShoulder: (pose: CatPose) => {
    const rear = pick(["back-left", "back-right"] as const);
    return [
      ...turnTo(pose.body.facing, rear),
      beat(500, { prop: undefined, body: { paws: "rest" } }),
      beat(650, { face: { lookBack: true, eyes: ["o", "o"] } }),
      beat(180, { face: { eyes: ["-", "-"] } }),
      beat(650, { face: { eyes: ["^", "^"] }, thought: ICONS.heart }),
      beat(250, { face: { lookBack: false }, thought: "" }),
    ];
  },
  sideSit: (pose: CatPose) => {
    const side = pick(["left", "right"] as const);
    const quarter = side === "left" ? "front-left" : "front-right";
    return [
      ...turnTo(pose.body.facing, side),
      beat(700, { body: { paws: "rest" }, prop: undefined, face: { eyes: ["-", "-"], mouth: "w" } }),
      beat(250, { face: { ears: "flat" } }),
      beat(550, { face: { ears: "up", eyes: ["o", "o"], mouth: "." } }),
      ...turnTo(side, quarter),
      beat(500, { face: { eyes: ["^", "^"] } }),
    ];
  },
  backToWork: (pose: CatPose) => {
    const rear = pick(["back-left", "back-right"] as const);
    const prop = pose.prop;
    const steps = [
      ...turnTo(pose.body.facing, rear),
      beat(450, { body: { paws: "hold" }, face: { lookBack: true, eyes: ["o", "o"] } }),
      ...turnTo(rear, "back"),
      beat(300, { body: { paws: "left-up" } }),
      beat(220, { body: { paws: "right-up" } }),
      beat(250, { body: { paws: "hold" } }),
    ];
    if (prop) steps.push(
      beat(240, { prop: { glyph: prop.glyph, location: "above" }, body: { paws: "right-up" } }),
      beat(450, { prop, body: { paws: "hold" }, thought: ICONS.star }),
    );
    steps.push(
      ...turnTo("back", rear),
      beat(650, { face: { lookBack: true, eyes: ["^", "^"] }, thought: "" }),
    );
    return steps;
  },
  listen: (_pose: CatPose) => [
    beat(350, { face: { eyes: ["o", "o"], ears: "left-down" } }),
    beat(650, { face: { eyes: ["O", "O"], ears: "up" } }),
    beat(240, { face: { ears: "right-down" }, body: { paws: "right-up" } }),
    beat(550, { face: { eyes: ["o", "o"], ears: "up" }, body: { paws: "rest" } }),
  ],
  tapPaw: (_pose: CatPose) => {
    const steps: MotionBeat[] = [];
    const taps = repetitions(2, 4);
    for (let index = 0; index < taps; index += 1) {
      steps.push(
        beat(200, { body: { paws: "left-up" }, face: { eyes: [">", ">"] } }),
        beat(200, { body: { paws: "rest" }, face: { ears: index % 2 ? "up" : "left-down" } }),
      );
    }
    return steps;
  },
  chin: (_pose: CatPose) => [
    beat(350, { face: { eyes: ["o", "-"], ears: "left-down" } }),
    beat(550, { body: { paws: "right-up" }, face: { mouth: "w" }, thought: "?" }),
    beat(250, { face: { eyes: ["-", "o"], ears: "right-down" }, thought: "" }),
    beat(650, { body: { paws: "left-up" }, face: { mouth: "." } }),
  ],
  scratch: (_pose: CatPose) => [
    beat(220, { body: { paws: "right-up" }, face: { eyes: ["o", "-"], ears: "right-down" } }),
    beat(150, { face: { ears: "up" } }),
    beat(150, { face: { ears: "right-down" } }),
    beat(170, { face: { ears: "up" } }),
    beat(500, { body: { paws: "rest" }, face: { eyes: ["o", "o"] }, thought: "?" }),
  ],
  idea: (pose: CatPose) => [
    beat(500, { face: { eyes: ["-", "-"] }, thought: "." }),
    beat(450, { body: { paws: "right-up" }, thought: "o" }),
    beat(260, { face: { eyes: ["O", "O"], ears: "up" }, thought: ICONS.bulb }),
    beat(220, { body: { column: otherColumn(pose), paws: "open" }, face: { eyes: ["^", "^"], mouth: "o" } }),
    beat(650, { body: { paws: "rest" }, face: { mouth: "." } }),
  ],
  ponderWalk: (pose: CatPose) => [
    ...walk(pose),
    beat(650, { body: { paws: "right-up" }, face: { eyes: ["-", "-"], ears: "left-down" }, thought: "?" }),
    beat(400, { body: { paws: "rest" }, face: { eyes: ["o", "o"], ears: "up" }, thought: "" }),
  ],
  typeBurst: (pose: CatPose) => typing(pose, false),
  typeLean: (pose: CatPose) => typing(pose, true),
  readBack: (_pose: CatPose) => [
    beat(700, { body: { paws: "hold" }, face: { eyes: ["<", "<"], ears: "left-down" } }),
    beat(450, { face: { eyes: [">", ">"], ears: "right-down" } }),
    beat(200, { face: { eyes: ["-", "-"], ears: "up" } }),
    beat(400, { face: { eyes: ["^", "^"] }, thought: ICONS.star }),
  ],
  shakePaws: (_pose: CatPose) => [
    beat(180, { body: { paws: "left-up" }, face: { eyes: [">", "<"] } }),
    beat(160, { body: { paws: "right-up" } }),
    beat(180, { body: { paws: "open" }, face: { mouth: "o" } }),
    beat(250, { body: { paws: "hold" }, face: { eyes: ["o", "o"], mouth: "." } }),
  ],
  sendSpark: (pose: CatPose) => [
    ...typing(pose, false).slice(0, 5),
    beat(180, { body: { paws: "right-up" }, thought: "." }),
    beat(200, { body: { paws: "open" }, thought: ICONS.star, face: { eyes: ["^", "^"], mouth: "w" } }),
    beat(400, { body: { paws: "hold" }, thought: "" }),
  ],
  hammer: (_pose: CatPose) => {
    const steps: MotionBeat[] = [];
    const strikes = repetitions(2, 4);
    for (let index = 0; index < strikes; index += 1) {
      steps.push(
        beat(260, { body: { paws: "right-up" }, prop: { glyph: ICONS.wrench, location: "above" }, face: { eyes: ["o", "o"] }, thought: "" }),
        beat(150, { body: { paws: "hold" }, prop: { glyph: ICONS.wrench, location: "paws" }, face: { eyes: [">", "<"], ears: "flat" }, thought: ICONS.star }),
        beat(180, { face: { ears: "up" }, thought: "" }),
      );
    }
    return steps;
  },
  juggleTool: (_pose: CatPose) => [
    beat(250, { body: { paws: "left-up" }, prop: { glyph: ICONS.wrench, location: "left-paw" }, face: { eyes: ["<", "<"] } }),
    beat(200, { body: { paws: "open" }, prop: { glyph: ICONS.wrench, location: "above" }, face: { eyes: ["O", "O"] } }),
    beat(240, { body: { paws: "right-up" }, prop: { glyph: ICONS.wrench, location: "right-paw" }, face: { eyes: [">", ">"] } }),
    beat(400, { body: { paws: "hold" }, prop: { glyph: ICONS.wrench, location: "paws" }, face: { eyes: ["^", "^"] } }),
  ],
  inspectTool: (_pose: CatPose) => [
    beat(550, { prop: { glyph: ICONS.wrench, location: "right-paw" }, body: { paws: "right-up" }, face: { eyes: ["o", "-"], ears: "left-down" } }),
    beat(350, { prop: { glyph: ICONS.wrench, location: "left-paw" }, body: { paws: "left-up" }, face: { eyes: ["-", "o"], ears: "right-down" } }),
    beat(220, { body: { paws: "hold" }, prop: { glyph: ICONS.wrench, location: "paws" }, face: { eyes: ["o", "o"], ears: "up" } }),
    beat(450, { face: { eyes: ["^", "^"] } }),
  ],
  polish: (_pose: CatPose) => [
    beat(240, { body: { paws: "left-up" }, face: { eyes: ["-", "-"], mouth: "w" } }),
    beat(220, { body: { paws: "right-up" }, face: { ears: "left-down" } }),
    beat(240, { body: { paws: "left-up" }, face: { ears: "right-down" } }),
    beat(500, { body: { paws: "hold" }, face: { eyes: ["^", "^"], ears: "up", mouth: "." }, thought: ICONS.star }),
  ],
  collectPaper: (pose: CatPose) => [
    beat(350, { prop: { glyph: ICONS.paper, location: "above" }, face: { eyes: [">", ">"] } }),
    beat(240, { body: { column: otherColumn(pose), paws: "right-up" }, prop: { glyph: ICONS.paper, location: "right-paw" } }),
    beat(250, { body: { paws: "hold" }, prop: { glyph: ICONS.paper, location: "paws" }, face: { eyes: ["o", "o"] } }),
    beat(350, { prop: { glyph: ICONS.archive, location: "paws" }, face: { eyes: ["^", "^"] } }),
  ],
  stuffBox: (_pose: CatPose) => [
    beat(300, { prop: { glyph: ICONS.paper, location: "paws" }, face: { eyes: ["o", "o"] } }),
    beat(260, { body: { paws: "open" }, prop: { glyph: ICONS.archive, location: "paws" }, face: { eyes: [">", "<"], ears: "flat" } }),
    beat(180, { body: { paws: "hold" }, face: { mouth: "o" } }),
    beat(260, { body: { paws: "open" }, face: { mouth: "." } }),
    beat(450, { body: { paws: "hold" }, face: { ears: "up", eyes: ["^", "^"] }, thought: ICONS.star }),
  ],
  patLid: (_pose: CatPose) => [
    beat(200, { body: { paws: "right-up" }, face: { eyes: ["^", "^"] } }),
    beat(180, { body: { paws: "hold" } }),
    beat(220, { body: { paws: "left-up" } }),
    beat(180, { body: { paws: "hold" } }),
    beat(600, { face: { eyes: ["-", "-"], mouth: "w" }, thought: ICONS.heart }),
  ],
  peekBox: (_pose: CatPose) => [
    beat(400, { face: { eyes: ["o", "-"], ears: "left-down" } }),
    beat(350, { body: { paws: "right-up" }, prop: { glyph: ICONS.archive, location: "right-paw" }, face: { eyes: [">", ">"] } }),
    beat(200, { face: { eyes: ["O", "O"], ears: "up" }, thought: "!" }),
    beat(350, { body: { paws: "hold" }, prop: { glyph: ICONS.archive, location: "paws" }, thought: "" }),
  ],
  tossPaper: (_pose: CatPose) => [
    beat(250, { body: { paws: "left-up" }, prop: { glyph: ICONS.paper, location: "left-paw" }, face: { eyes: ["<", "<"] } }),
    beat(220, { body: { paws: "open" }, prop: { glyph: ICONS.paper, location: "above" }, face: { eyes: ["O", "O"] } }),
    beat(240, { body: { paws: "right-up" }, prop: { glyph: ICONS.paper, location: "right-paw" }, face: { eyes: [">", ">"] } }),
    beat(500, { body: { paws: "hold" }, prop: { glyph: ICONS.archive, location: "paws" }, face: { eyes: ["^", "^"] } }),
  ],
} satisfies Record<string, (pose: CatPose) => MotionBeat[]>;

type GestureName = keyof typeof GESTURES;

/** Gestures with their own turns retain the incoming direction; paw work can start at a side angle. */
function startingFacing(name: GestureName, current: CatFacing): CatFacing {
  switch (name) {
    case "glance": case "ears": case "wander": case "ponderWalk": case "chaseTail":
    case "turnAway": case "lookOverShoulder": case "sideSit": case "backToWork":
      return current;
    case "typeBurst": case "typeLean": case "readBack": case "shakePaws": case "sendSpark":
    case "hammer": case "juggleTool": case "inspectTool": case "polish":
    case "collectPaper": case "stuffBox": case "patLid": case "peekBox": case "tossPaper":
      return pick(["front", "front-left", "front-right", "front-left", "front-right", "left", "right"] as const);
    case "groom": case "listen": case "chin": case "scratch":
      return pick(["front", "front-left", "front-right"] as const);
    default:
      return "front";
  }
}

// Repeated names bias selection toward the current job. The last three performances are excluded.
const GESTURES_BY_ACTIVITY: Record<ActivityKind, readonly GestureName[]> = {
  ready: [
    "glance", "ears", "wander", "peek", "stretch", "groom", "wave", "hop", "curl", "roll", "chaseTail",
    "turnAway", "lookOverShoulder", "sideSit",
  ],
  waiting: [
    "glance", "glance", "listen", "listen", "tapPaw", "wander", "peek", "ears", "stretch", "sideSit", "lookOverShoulder",
  ],
  thinking: ["chin", "chin", "scratch", "idea", "ponderWalk", "glance", "ears", "tapPaw", "turnAway", "sideSit"],
  writing: ["typeBurst", "typeBurst", "typeLean", "typeLean", "readBack", "shakePaws", "sendSpark", "ears", "backToWork"],
  tool: ["hammer", "hammer", "juggleTool", "inspectTool", "polish", "shakePaws", "glance", "ears", "backToWork"],
  compacting: ["collectPaper", "collectPaper", "stuffBox", "patLid", "peekBox", "tossPaper", "glance", "ears", "backToWork"],
};
const TAIL_POSES: readonly TailPose[] = [
  { glyph: "_", row: 2 },
  { glyph: "~", row: 2 },
  { glyph: ")", row: 2 },
  { glyph: ")", row: 1 },
  { glyph: "|", row: 1 },
];

/** Independent blink/tail clocks keep repeated gestures from replaying the same complete picture. */
export class CatMotion {
  private kind: ActivityKind = "ready";
  private pose = restingPose("ready", 1);
  private beats: MotionBeat[] = [];
  private performing = false;
  private recentGestures: GestureName[] = [];
  private tempo = 1;
  private poseChangesAt = 0;
  private blinkPhase: "open" | "closed" | "between" | "closed-again" = "open";
  private blinkChangesAt = performance.now() + between(1600, 4500);
  private tailIndex = 1;
  private tailDirection = 1;
  private tailChangesAt = performance.now() + between(250, 900);

  /** Randomness advances only at movement deadlines, never on unrelated footer repaints. */
  public sample(kind: ActivityKind): { lines: string[]; face: string; nextChangeAt: number } {
    const now = performance.now();
    if (kind !== this.kind) {
      this.kind = kind;
      this.pose = restingPose(kind, this.pose.body.column, this.pose.body.facing);
      this.beats = [];
      this.performing = false;
      this.poseChangesAt = 0;
      this.blinkPhase = "open";
      this.blinkChangesAt = now + between(1800, 5000);
    }
    if (now >= this.poseChangesAt) this.advancePose(now);
    if (now >= this.blinkChangesAt) this.advanceBlink(now);
    if (now >= this.tailChangesAt) this.advanceTail(now);
    const eyesClosed = this.blinkPhase === "closed" || this.blinkPhase === "closed-again";
    return {
      ...drawCat(this.pose, eyesClosed, TAIL_POSES[this.tailIndex]!),
      nextChangeAt: Math.min(this.poseChangesAt, this.blinkChangesAt, this.tailChangesAt),
    };
  }

  private advancePose(now: number): void {
    if (this.beats.length === 0) {
      if (this.performing) {
        this.pose = restingPose(this.kind, this.pose.body.column, this.pose.body.facing);
        this.performing = false;
        this.poseChangesAt = now + (this.kind === "ready" ? between(900, 2800) : between(180, 800));
        return;
      }
      const candidates = GESTURES_BY_ACTIVITY[this.kind].filter((name) => !this.recentGestures.includes(name));
      const name = pick(candidates);
      this.recentGestures = [...this.recentGestures, name].slice(-3);
      const facing = startingFacing(name, this.pose.body.facing);
      const entryPose = changePose(this.pose, { body: { facing }, face: { lookBack: false } });
      this.beats = [...turnTo(this.pose.body.facing, facing), ...GESTURES[name](entryPose)];
      this.tempo = between(0.8, 1.25);
      this.performing = true;
    }
    const next = this.beats.shift()!;
    this.pose = changePose(this.pose, next.change);
    this.poseChangesAt = now + Math.max(80, next.milliseconds * this.tempo * between(0.88, 1.12));
  }

  private advanceBlink(now: number): void {
    switch (this.blinkPhase) {
      case "open":
        this.blinkPhase = "closed";
        this.blinkChangesAt = now + between(100, 170);
        break;
      case "closed":
        this.blinkPhase = Math.random() < 0.24 ? "between" : "open";
        this.blinkChangesAt = now + (this.blinkPhase === "between" ? between(90, 150) : between(2400, 6500));
        break;
      case "between":
        this.blinkPhase = "closed-again";
        this.blinkChangesAt = now + between(100, 160);
        break;
      case "closed-again":
        this.blinkPhase = "open";
        this.blinkChangesAt = now + between(2400, 6500);
        break;
    }
  }

  private advanceTail(now: number): void {
    if (this.tailIndex === 0) this.tailDirection = 1;
    else if (this.tailIndex === TAIL_POSES.length - 1) this.tailDirection = -1;
    else if (Math.random() < 0.18) this.tailDirection *= -1;
    this.tailIndex += this.tailDirection;
    this.tailChangesAt = now + (Math.random() < 0.25 ? between(650, 1700) : between(140, 380));
  }
}

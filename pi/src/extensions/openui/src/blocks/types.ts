import type { Cell, Rect } from "../diagram/canvas.ts";
import type { El } from "../lang/library.ts";
import type { Answers, Choice } from "./choice.ts";
import type { TableSort } from "./table.ts";

export interface Tip {
  hint?: string;
  detail?: string;
  code?: string;
  language?: string;
  tone?: string;
  link?: string;
  linkLabel?: string;
}

export type TargetKind =
  | "tab"
  | "button"
  | "sort"
  | "cell"
  | "fold"
  | "source"
  /** one option of a Choice, or its Other row */
  | "choice"
  /** the Send answers button under a widget that has Choices */
  | "send"
  /** the Clear button next to it */
  | "clear"
  | "faq"
  | "node"
  | "copy"
  | "tip"
  | "image"
  /** something a note can be attached to (see notes.ts); only reachable in notes mode */
  | "item"
  /** a row of the notes tray */
  | "note"
  | "noteadd"
  | "noteclear"
  /** the footer chip that turns notes mode on and off */
  | "notes";

/** What a note says about the element it is attached to. All words are already cleaned for a message the user will send. */
export interface ItemInfo {
  /** the element's path in the tree: items of one component share it */
  host: string;
  type: string;
  /** the variable the component was defined under */
  name?: string;
  /** the tooltip vocabulary key ("row:3", "a>b"), or "" for the whole component */
  key: string;
  /** where inside the component, in words: `row 3, cell "CrashLoop"` */
  where: string;
  /** a short label for the hover line and the tray: `Table > row 3` */
  short: string;
  /** the tab it sits under */
  tab?: string;
  /** source line, to identify a component that has no variable */
  line: number;
  /** the item that contains this one (a table cell's row, a column's table), by target id */
  parent?: string;
}

export interface Note {
  id: number;
  itemId: string;
  info: ItemInfo;
  text: string;
}

/** Something the user can click or reach with the keyboard. Rects are relative to the block that owns them. */
export interface Target {
  id: string;
  kind: TargetKind;
  rect: Rect;
  label: string;
  tip?: Tip;
  /** text sent to the assistant when activated */
  send?: string;
  openUrl?: string;
  copy?: string;
  sort?: { table: string; column: number };
  /** for kind "send": what it sends. The message is built from the answers as they are when it is pressed, never from a picture that may be a frame old. */
  bar?: { choices: Choice[]; title: string };
  /** for kind "choice": the question and which row of it this is (an option index, or its Other row) */
  choice?: { of: Choice; index: number | "other" };
  /** extra cells (block coordinates) that belong to the target besides its rect, e.g. the wedge of a pie slice */
  cells?: Array<[number, number]>;
  /** for kind "item": what a note on it is about */
  item?: ItemInfo;
  /** for kind "note": the note this tray row shows */
  noteId?: number;
  /** for kind "image": where the picture goes and how big it is, in cells */
  image?: { key: string; cols: number; rows: number };
  /** a diagram that may be wider than its view: focusing this target should reveal it */
  pan?: { key: string; x: number; w: number; view: number; total: number };
}

export interface Block {
  w: number;
  rows: Cell[][];
  targets: Target[];
}

export interface WState {
  tableSort: Record<string, TableSort>;
  delivery: Record<string, "sending" | "queued" | "sent" | "failed">;
  tab: Record<string, number>;
  faq: Record<string, boolean>;
  /** what the user has staged for each Choice, by question key. Nothing is sent until Send answers. */
  answers: Answers;
  /** the answers have been sent: every Choice is locked */
  sent: boolean;
  /** sent, but pi has not said yet that it took the message (the lock is held; the bar says Sending) */
  sending: boolean;
  pan: Record<string, number>;
  tip: string | null;
  focus: string | null;
  hover: string | null;
  copied: string | null;
  /** the user's notes on this widget, in the order they were written */
  notes: Note[];
  /** notes mode: clicks and Enter write notes instead of acting */
  noteMode: boolean;
  noteSeq: number;
}

export const newState = (): WState => ({
  tableSort: {},
  delivery: {},
  tab: {},
  faq: {},
  answers: {},
  sent: false,
  sending: false,
  pan: {},
  tip: null,
  focus: null,
  hover: null,
  copied: null,
  notes: [],
  noteMode: false,
  noteSeq: 0,
});

/** Supplied by the host when it can draw bitmaps. Without it, an Image is drawn as a labelled frame. */
export interface ImageService {
  /** Load the image and reserve a size for it. Null when it cannot be drawn here (no protocol, unreadable, unsupported). */
  place(src: string, maxCols: number, maxRows: number): { key: string; cols: number; rows: number } | null;
  /** Human text for the frame when the picture itself cannot be shown, e.g. "800×600 png, 120 KB". */
  describe(src: string): string;
}

export interface Services {
  images?: ImageService;
}

export interface Ctx {
  state: WState;
  warnings: string[];
  services: Services;
  /** labels of the tabs the element being drawn sits under, outermost first */
  tabs: string[];
  render: (el: El, width: number, path: string) => Block;
}

export type ParameterValue = string | number | boolean;
export interface Parameter {
  label?: string;
  control: "range" | "number" | "color" | "text" | "boolean" | "select";
  value: ParameterValue;
  min?: number;
  max?: number;
  step?: number;
  options?: string[];
}
export interface DesignData {
  name: string;
  elements: Record<string, { text?: string; style?: Record<string, string | number> }>;
  parameters: Record<string, Parameter>;
}
export interface Artboard {
  id: string;
  name: string;
  route: string;
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface CommentAnchor {
  artboard: string;
  element?: string;
  x?: number;
  y?: number;
}
export interface DesignComment {
  id: string;
  text: string;
  resolved: boolean;
  anchor: CommentAnchor;
}
export interface Selection {
  artboard: string | null;
  element: string | null;
}
export interface Project {
  id: string;
  name: string;
  path: string;
}
export interface State {
  project: Project | null;
  revision: number;
  design: DesignData;
  workspace: { artboards: Artboard[]; comments: DesignComment[] };
  selection: Selection;
  preview_url: string | null;
  can_undo: boolean;
  can_redo: boolean;
  history: {
    current: number;
    entries: Array<{ id: number; summary: string }>;
  } | null;
}
export type Edit =
  | { type: "text"; element: string; text: string }
  | { type: "style"; element: string; style: Record<string, string | number | null> }
  | { type: "parameter"; name: string; value: ParameterValue }
  | { type: "artboard"; artboard: Artboard }
  | { type: "remove_artboard"; id: string }
  | { type: "comment"; comment: DesignComment }
  | { type: "remove_comment"; id: string };
export type Mode = "select" | "comment" | "play";
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface ElementBounds {
  id: string;
  tag: string;
  text: string;
  rect: Rect;
  kind?: "text" | "box";
}
export type RuntimeMessage =
  | { source: "pi-design"; type: "ready"; artboard: string }
  | {
      source: "pi-design";
      type: "selection";
      artboard: string;
      element: string | null;
      rect: Rect | null;
      text: string;
      tag: string;
    }
  | { source: "pi-design"; type: "comment"; artboard: string; element?: string; x: number; y: number }
  | { source: "pi-design"; type: "bounds"; artboard: string; elements: ElementBounds[] };
export type EditorMessage =
  { source: "pi-design"; type: "mode"; mode: Mode } | { source: "pi-design"; type: "select"; element: string | null };

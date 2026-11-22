import { createContext, useContext } from "react";
import type { DesignConnection } from "./client.js";
import type { DesignData, Mode } from "./types.js";
import type { EditorEnvironment } from "./environment.js";

export interface RuntimeContext {
  data: DesignData;
  connection: DesignConnection | null;
  environment: EditorEnvironment | null;
  mode: Mode;
  setMode: (mode: Mode) => void;
  selected: string | null;
  setSelected: (id: string | null) => void;
  editing: string | null;
  setEditing: (id: string | null) => void;
}
export const DesignContext = createContext<RuntimeContext | null>(null);
export function useDesign() {
  const value = useContext(DesignContext);
  if (!value) throw new Error("Editable components and useParameters must be inside DesignProvider.");
  return value;
}

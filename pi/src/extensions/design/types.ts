export interface DesignProject {
  id: string;
  name: string;
  path: string;
}

export interface DesignState {
  project: DesignProject | null;
  revision: number;
  design: {
    name: string;
    elements: Record<string, { text?: string; style?: Record<string, string | number> }>;
    parameters: Record<string, { value: string | number | boolean; [key: string]: unknown }>;
  };
  workspace: {
    artboards: Array<{ id: string; name: string; route: string; [key: string]: unknown }>;
    comments: Array<{
      id: string;
      text: string;
      resolved: boolean;
      anchor: { artboard: string; element?: string; x?: number; y?: number };
    }>;
  };
  selection: { artboard: string | null; element: string | null };
  preview_url: string | null;
}

export interface DesignChanges {
  revision: number;
  changes: Array<{ revision: number; file: string; patch: unknown }>;
}

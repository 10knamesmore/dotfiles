/** Globals supplied by the workflow VM, exported here for editor declarations. */
export interface WorkflowAgentOptions {
  /** Required model tier; provider/model IDs are not accepted. */
  model: "mid" | "high" | "max";
  /** Omit to inherit the parent's thinking level. */
  thinkingLevel?: "medium" | "xhigh" | "max";
  /** Tool allowlist. Omit to use tools discovered for the child's cwd. */
  tools?: string[];
  /** Tools to exclude from the child's available tools. */
  excludeTools?: string[];
  /** JSON Schema for the child's validated structured result. */
  schema?: Record<string, unknown>;
  /** Omit to inherit the parent's working directory. */
  cwd?: string;
  /** Short name shown in workflow status. */
  label?: string;
  /** Phase for this child; use inside parallel and pipeline branches. */
  phase?: string;
}

/** Run a child with a self-contained prompt. Child failure resolves to null. */
export function agent(prompt: string, options: WorkflowAgentOptions & { schema?: undefined }): Promise<string | null>;
/** Return the schema-validated result, or null when the child fails. */
export function agent(prompt: string, options: WorkflowAgentOptions): Promise<unknown>;

export interface WorkflowConcurrency {
  /** Positive integer limiting this group. Omit to use the process-wide limit. */
  concurrency?: number;
}

/** Run branches concurrently and return results in input order. */
export function parallel<Results extends unknown[]>(
  thunks: { [Index in keyof Results]: () => Results[Index] },
  options?: WorkflowConcurrency,
): Promise<{ [Index in keyof Results]: Awaited<Results[Index]> }>;

export type WorkflowStage<Item> = (item: Item, previous: unknown) => unknown;

/** Run each item through the stages; stage one receives undefined as previous. */
export function pipeline<Item>(items: Item[], ...stages: WorkflowStage<Item>[]): Promise<unknown[]>;
/** Limit concurrent items while running each item's stages sequentially. */
export function pipeline<Item>(
  items: Item[],
  options: WorkflowConcurrency,
  ...stages: WorkflowStage<Item>[]
): Promise<unknown[]>;

/** Set the phase for subsequent children. Only call while the root scope is idle. */
export function phase(title: string): void;

/** Show a progress message in the workflow UI. */
export function log(message: string): void;

export const console: {
  /** Stringify and join values into one workflow progress message. */
  log(...values: unknown[]): void;
};

export type DeepReadonly<Value> = Value extends object
  ? { readonly [Key in keyof Value]: DeepReadonly<Value[Key]> }
  : Value;

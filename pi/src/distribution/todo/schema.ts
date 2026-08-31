import { StringEnum } from "@earendil-works/pi-ai";
import { type Static, Type } from "typebox";

/** Operation names accepted by the model-facing todo tool. */
export const TODO_OPERATIONS = [
  "init",
  "start",
  "done",
  "drop",
  "block",
  "unblock",
  "append",
  "rm",
  "view",
] as const;

const TodoInitPhaseParameters = Type.Object(
  {
    phase: Type.String({ description: "Unique human-readable phase name" }),
    items: Type.Array(
      Type.String({ description: "Unique human-readable task content" }),
    ),
  },
  { additionalProperties: false },
);

/** TypeBox schema for one todo operation. Operation-specific rules run atomically in the executor. */
export const TodoParameters = Type.Object(
  {
    op: StringEnum(TODO_OPERATIONS, {
      description: "Single todo operation to apply",
    }),
    list: Type.Optional(
      Type.Array(TodoInitPhaseParameters, {
        description: "Complete phased list for init",
      }),
    ),
    task: Type.Optional(
      Type.String({ description: "Exact task content identifying one task" }),
    ),
    phase: Type.Optional(
      Type.String({ description: "Exact phase name identifying one phase" }),
    ),
    items: Type.Optional(
      Type.Array(
        Type.String({ description: "Tasks for a flat init or append" }),
      ),
    ),
    reason: Type.Optional(
      Type.String({ description: "Optional single-line blocker reason" }),
    ),
  },
  { additionalProperties: false },
);

/** Parameters for one model-facing todo call. */
export type TodoParams = Static<typeof TodoParameters>;

/** Discriminator for one todo call. */
export type TodoOperation = (typeof TODO_OPERATIONS)[number];

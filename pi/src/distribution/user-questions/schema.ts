import { type Static, Type } from "typebox";
import { GLOBAL_CONFIG } from "../../config.ts";

const UserQuestionOptionSchema = Type.Object(
  {
    value: Type.String({ description: "Value returned when this option is selected" }),
    label: Type.String({ description: "Text shown to the user" }),
    description: Type.Optional(
      Type.String({ description: "Additional context shown with the option" }),
    ),
  },
  { additionalProperties: false },
);

const UserQuestionSchema = Type.Object(
  {
    id: Type.String({ description: "Unique identifier for this question" }),
    question: Type.String({ description: "Question text shown to the user" }),
    options: Type.Optional(
      Type.Array(UserQuestionOptionSchema, {
        description:
          "Options for this question; omit or leave empty for a free-text question",
      }),
    ),
    allowOther: Type.Optional(
      Type.Boolean({
        description:
          "For option questions, whether the user may enter a custom answer; defaults to true. When enabled, an option labeled Other is also treated as the custom-answer entry",
      }),
    ),
    placeholder: Type.Optional(
      Type.String({ description: "Placeholder shown for a free-text answer" }),
    ),
  },
  { additionalProperties: false },
);

/** Parameters for the model-facing user questions tool. */
export const AskUserQuestionsParameters = Type.Object(
  {
    questions: Type.Array(UserQuestionSchema, {
      description: `Questions to ask, each with its own input configuration, shoule be in ${GLOBAL_CONFIG.language}`,
    }),
  },
  { additionalProperties: false },
);

export type AskUserQuestionsParams = Static<typeof AskUserQuestionsParameters>;
export type UserQuestion = AskUserQuestionsParams["questions"][number];
export type UserQuestionOption = NonNullable<UserQuestion["options"]>[number];

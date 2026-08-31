import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { registerUserQuestionsTool } from "./tool.js";

/** Register the session-scoped tool that collects answers from the user. */
export function registerUserQuestions(pi: ExtensionAPI): void {
  registerUserQuestionsTool(pi);
}

export default registerUserQuestions;

export type {
  AskUserQuestionsDetails,
  UserQuestionAnswer,
} from "./tool.js";
export type {
  AskUserQuestionsParams,
  UserQuestion,
  UserQuestionOption,
} from "./schema.js";

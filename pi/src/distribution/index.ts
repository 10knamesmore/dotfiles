import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { registerEffortAlias } from "./effort/index.js";
import { registerFooter as registerFooter } from "./footer/index.js";
import { registerHook } from "./hook/index.js";
import { registerSettledNotification } from "./notification/index.js";
import { registerSessionTodo } from "./todo/index.js";
import { registerUserQuestions } from "./user-questions/index.js";

/** Register every first-party capability owned by the dotfiles Pi distribution. */
export default function registerDistribution(pi: ExtensionAPI): void {
  registerEffortAlias(pi);
  registerHook(pi);
  registerSettledNotification(pi);
  registerSessionTodo(pi);
  registerUserQuestions(pi);
  registerFooter(pi);
}

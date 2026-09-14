import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { registerBashBackground } from "./bash-background/index.js";
import { registerEffortAlias } from "./effort/index.js";
import { registerFooter as registerFooter } from "./footer/index.js";
import { registerHook } from "./hook/index.js";
import { registerSettledNotification } from "./notification/index.js";
import { registerProviderScope } from "./provider-scope/index.js";
import { registerTitlebar } from "./titlebar/index.js";
import { registerSessionTodo } from "./todo/index.js";
import { registerUserQuestions } from "./user-questions/index.js";

/** Register every first-party capability owned by the dotfiles Pi distribution. */
export default function registerDistribution(pi: ExtensionAPI): void {
  registerBashBackground(pi);
  registerEffortAlias(pi);
  registerHook(pi);
  registerSettledNotification(pi);
  registerProviderScope(pi);
  registerSessionTodo(pi);
  registerTitlebar(pi);
  registerUserQuestions(pi);
  registerFooter(pi);
}

import { matchesKey, parseKey } from "@earendil-works/pi-tui";

export type ContextNavigationAction =
  | { type: "move"; rows: number }
  | { type: "page"; direction: number }
  | { type: "scroll"; lines: number }
  | { type: "scrollPages"; pages: number }
  | { type: "first" | "last" | "open" | "back" | "close" | "filter" | "help" | "none" };

/** Match /agents: J/K move seven items, Ctrl+U/D scroll one line, Ctrl+B/F scroll half a page. */
export function contextNavigationAction(data: string): ContextNavigationAction {
  if (matchesKey(data, "shift+j")) return { type: "move", rows: 7 };
  if (matchesKey(data, "shift+k")) return { type: "move", rows: -7 };
  if (matchesKey(data, "shift+g")) return { type: "last" };
  if (matchesKey(data, "?") || matchesKey(data, "shift+/")) return { type: "help" };
  switch (parseKey(data)?.toLowerCase()) {
    case "j":
    case "down":
      return { type: "move", rows: 1 };
    case "k":
    case "up":
      return { type: "move", rows: -1 };
    case "ctrl+u":
      return { type: "scroll", lines: -1 };
    case "ctrl+d":
      return { type: "scroll", lines: 1 };
    case "ctrl+b":
      return { type: "scrollPages", pages: -0.5 };
    case "ctrl+f":
      return { type: "scrollPages", pages: 0.5 };
    case "pageup":
      return { type: "page", direction: -1 };
    case "pagedown":
      return { type: "page", direction: 1 };
    case "g":
    case "home":
      return { type: "first" };
    case "end":
      return { type: "last" };
    case "enter":
    case "return":
    case "l":
    case "right":
      return { type: "open" };
    case "h":
    case "left":
    case "escape":
    case "esc":
      return { type: "back" };
    case "q":
    case "ctrl+c":
      return { type: "close" };
    case "/":
    case "f":
      return { type: "filter" };
    default:
      return { type: "none" };
  }
}

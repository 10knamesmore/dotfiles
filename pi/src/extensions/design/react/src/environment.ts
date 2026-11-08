export interface EditorEnvironment { endpoint: string; origin: string; artboard: string }

const query = typeof window === "undefined" ? null : new URLSearchParams(window.location.search);
const endpoint = query?.get("__pi_design");
const artboard = query?.get("__pi_artboard");

export const editorEnvironment: EditorEnvironment | null = (() => {
  if (!endpoint || !artboard || window.parent === window) return null;
  try {
    const url = new URL(endpoint);
    if (url.protocol !== "http:" && url.protocol !== "https:") return null;
    const origin = document.referrer ? new URL(document.referrer).origin : url.origin;
    return { endpoint: url.href.replace(/\/$/, ""), origin, artboard };
  } catch { return null; }
})();

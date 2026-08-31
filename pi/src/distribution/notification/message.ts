import { basename } from "node:path";

const MAX_PROJECT_CHARACTERS = 80;

/** Plain-text fields delivered to every notification backend. */
export interface SettledNotificationMessage {
  /** Short title containing Pi and the current project directory. */
  title: string;

  /** Stable state description shown below the title. */
  body: string;
}

/** Build a bounded, single-line notification for a settled Pi session. */
export function createSettledNotificationMessage(
  cwd: string,
): SettledNotificationMessage {
  const project =
    truncate(sanitizeSingleLine(basename(cwd)), MAX_PROJECT_CHARACTERS) ||
    "workspace";
  return {
    title: `Pi · ${project}`,
    body: "已完成，等待输入",
  };
}

/** Remove terminal controls and directional formatting from visible text. */
export function sanitizeSingleLine(value: string): string {
  return value
    .replace(
      /[\u0000-\u001f\u007f-\u009f\u2028\u2029\u202a-\u202e\u2066-\u2069]/g,
      " ",
    )
    .replace(/\s+/g, " ")
    .trim();
}

/** Bound text by Unicode code points without cutting a surrogate pair. */
function truncate(value: string, maxCharacters: number): string {
  const characters = [...value];
  if (characters.length <= maxCharacters) return value;
  return `${characters.slice(0, maxCharacters - 1).join("")}…`;
}

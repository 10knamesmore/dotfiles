const TERMINAL_CONTROL_CHARACTER =
  /[\u0000-\u0009\u000b-\u001f\u007f-\u009f\u2028\u2029]/gu;

/** Remove terminal controls while preserving ordinary line feeds in trusted multiline text. */
export function sanitizeTodoDisplayText(value: string): string {
  return value.replace(TERMINAL_CONTROL_CHARACTER, " ");
}

/** Reduce untrusted tool arguments and diagnostics to one bounded display line. */
export function sanitizeTodoDisplayLine(value: string, maxChars = 160): string {
  const clean = sanitizeTodoDisplayText(value).replace(/\s+/gu, " ").trim();
  const chars = Array.from(clean);
  return chars.length <= maxChars
    ? clean
    : `${chars.slice(0, maxChars - 1).join("")}…`;
}

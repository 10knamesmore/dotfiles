const TERMINAL_CONTROL_CHARACTER =
  /[\u0000-\u0009\u000b-\u001f\u007f-\u009f\u2028\u2029]/gu;

/** Remove terminal controls before displaying user-provided question content. */
export function sanitizeQuestionDisplay(value: string): string {
  return value.replace(TERMINAL_CONTROL_CHARACTER, " ");
}

/** Keep a question or answer readable in a compact tool row. */
export function compactQuestionDisplay(value: string, maxChars = 160): string {
  const clean = sanitizeQuestionDisplay(value).replace(/\s+/gu, " ").trim();
  const chars = Array.from(clean);
  return chars.length <= maxChars
    ? clean
    : `${chars.slice(0, maxChars - 1).join("")}…`;
}

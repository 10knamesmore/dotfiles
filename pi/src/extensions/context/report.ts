export type ReportTone = "value" | "number" | "success" | "warning" | "error";

export type ReportRow =
  | { kind: "heading"; text: string }
  | { kind: "field"; label: string; value: string; detail?: string; tone: ReportTone }
  | { kind: "note"; text: string }
  | { kind: "change"; operation: "added" | "removed" | "changed"; text: string };

export const heading = (text: string): ReportRow => ({ kind: "heading", text });
export const note = (text: string): ReportRow => ({ kind: "note", text });
export const field = (label: string, value: string, detail?: string, tone: ReportTone = "value"): ReportRow => ({
  kind: "field",
  label,
  value,
  detail,
  tone,
});
export const change = (operation: "added" | "removed" | "changed", text: string): ReportRow => ({
  kind: "change",
  operation,
  text,
});

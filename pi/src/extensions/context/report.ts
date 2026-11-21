import type { ThemeColor } from "@earendil-works/pi-coding-agent";

export interface TableColumn {
  weight: number;
  align?: "right";
}

export type TableCell = string | { text: string; color?: ThemeColor; bold?: boolean };

export interface ReportSection {
  title: string;
  columns: readonly (TableColumn & { heading: string })[];
  rows: readonly (readonly TableCell[])[];
  note?: string;
}

export const numberCell = (text: string): TableCell => ({ text, color: "syntaxNumber" });

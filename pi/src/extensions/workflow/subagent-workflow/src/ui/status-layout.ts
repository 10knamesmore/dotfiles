import { truncateToWidth, visibleWidth, wrapTextWithAnsi } from "@earendil-works/pi-tui";
import type { SubagentStatus } from "../types.js";
import { statusGlyph, type ThemeLike } from "./format.js";

export interface StatusRow {
  key: string;
  kind: "workflow" | "agent" | "child" | "more";
  label: string;
  prefix: string;
  model?: string;
  context?: string;
  input?: string;
  output?: string;
  phaseProgress?: string;
  status?: SubagentStatus;
}

const COLUMN_GAP = "  ";
const MIN_LABEL_WIDTH = 8;

function padToWidth(text: string, width: number): string {
  return text + " ".repeat(Math.max(0, width - visibleWidth(text)));
}

function rightAlign(text: string, width: number): string {
  return " ".repeat(Math.max(0, width - visibleWidth(text))) + text;
}

function renderWrappedRow(
  title: string,
  fields: string[],
  indent: number,
  separator: string,
  theme: ThemeLike,
  width: number,
): string[] {
  const lines = [title];
  const continuation = " ".repeat(Math.min(indent, Math.floor(width / 4)));
  for (const [index, field] of fields.filter(Boolean).entries()) {
    const gap = index === 0 ? separator : COLUMN_GAP;
    const fieldWidth = visibleWidth(field);
    const last = lines[lines.length - 1]!;
    if (visibleWidth(last) + visibleWidth(gap) + fieldWidth <= width) {
      lines[lines.length - 1] = last + gap + theme.fg("dim", field);
    } else {
      const divider = index === 0 && width - continuation.length >= 4 ? separator.trimStart() : "";
      const prefix = continuation + divider;
      const fieldRoom = width - visibleWidth(prefix);
      for (const [i, part] of wrapTextWithAnsi(field, fieldRoom).entries()) {
        const margin = i === 0 ? prefix : " ".repeat(visibleWidth(prefix));
        lines.push(margin + theme.fg("dim", truncateToWidth(part, fieldRoom, "…")));
      }
    }
  }
  return lines;
}

type Section = "Agents" | "Workflows";
type RenderedRow = { section: Section; lines: string[] };

function sectionHeading(section: Section, theme: ThemeLike, width: number, overflow = ""): string {
  const label = `  ${section} `;
  const rule = "─".repeat(Math.max(0, width - visibleWidth(label + overflow)));
  return truncateToWidth(theme.fg("muted", label) + theme.fg("borderMuted", rule) + theme.fg("dim", overflow), width);
}

function bodyWindow(body: RenderedRow[], focused: number, maxLines: number, theme: ThemeLike, width: number) {
  const lines: string[] = [];
  let first = focused;
  let remaining = maxLines - 1 - (body[focused]?.lines.length ?? 0);
  while (first > 0) {
    const previous = body[first - 1]!;
    const previousHeight = previous.lines.length + Number(previous.section !== body[first]!.section);
    if (previousHeight > remaining) break;
    remaining -= previousHeight;
    first--;
  }

  let end = first;
  let clipped = false;
  while (end < body.length && lines.length < maxLines) {
    const row = body[end]!;
    const heading = end === first || row.section !== body[end - 1]!.section;
    const room = maxLines - lines.length - Number(heading);
    if (row.lines.length > room && end !== focused) break;
    if (heading) lines.push(sectionHeading(row.section, theme, width));
    clipped = row.lines.length > room;
    lines.push(...row.lines.slice(0, room));
    end++;
  }
  const below = body.length - end;
  const overflow = [first && `↑${first}`, below && `↓${below}`, clipped && "…"].filter(Boolean).join(" ");
  if (overflow) lines[0] = sectionHeading(body[first]!.section, theme, width, ` ${overflow}`);
  return lines;
}

export function renderStatusRows(
  rows: StatusRow[],
  selectedKey: string | undefined,
  theme: ThemeLike,
  width: number,
  maxBodyLines: number,
): string[] {
  if (width < 1) return [];

  let modelWidth = 0;
  let contextWidth = 0;
  let phaseWidth = 0;
  let inputWidth = 0;
  let outputWidth = 0;
  let preferredTitleWidth = 0;
  let minimumTitleWidth = 0;
  const entries = rows.map((row) => {
    const selected = row.key === selectedKey;
    const cursor = selected ? `${theme.fg("accent", "❯")} ` : "  ";
    const glyph = row.status ? `${statusGlyph(row.status, theme, 0, false)} ` : "";
    const lead = cursor + row.prefix + glyph;
    const { kind, label, output = "" } = row;
    const input = kind === "workflow" ? `Σ ${row.input}` : (row.input ?? "");
    const model = kind === "workflow" ? "" : (row.model ?? "");
    const context = kind === "workflow" ? "" : (row.context ?? "");
    const phase = kind === "workflow" ? (row.phaseProgress ?? "") : "";
    modelWidth = Math.max(modelWidth, visibleWidth(model));
    contextWidth = Math.max(contextWidth, visibleWidth(context));
    phaseWidth = Math.max(phaseWidth, visibleWidth(phase));
    inputWidth = Math.max(inputWidth, visibleWidth(input));
    outputWidth = Math.max(outputWidth, visibleWidth(output));
    const leadWidth = visibleWidth(lead);
    if (kind !== "more") {
      const labelWidth = visibleWidth(label);
      preferredTitleWidth = Math.max(preferredTitleWidth, leadWidth + labelWidth);
      minimumTitleWidth = Math.max(minimumTitleWidth, leadWidth + Math.min(labelWidth, MIN_LABEL_WIDTH));
    }
    return { kind, selected, lead, leadWidth, label, model, context, phase, input, output };
  });

  const agentMiddleWidth = modelWidth + contextWidth + (modelWidth ? COLUMN_GAP.length : 0);
  const middleWidth = Math.max(agentMiddleWidth, phaseWidth);
  const separator = ` ${theme.fg("borderMuted", "│")} `;
  const metadataWidth = visibleWidth(separator) + middleWidth + inputWidth + outputWidth + 2 * COLUMN_GAP.length;
  const titleWidth = Math.min(preferredTitleWidth, width - metadataWidth);
  const aligned = titleWidth >= minimumTitleWidth;

  const body = entries.map(
    ({ kind, selected, lead, leadWidth, label, model, context, phase, input, output }): RenderedRow => {
      const nameWidth = aligned && kind !== "more" ? titleWidth : width;
      const shortLabel = truncateToWidth(label, nameWidth - leadWidth, "…");
      const styledLabel =
        selected || kind === "workflow"
          ? theme.bold(shortLabel)
          : kind === "more"
            ? theme.fg("dim", shortLabel)
            : shortLabel;
      const title =
        nameWidth >= leadWidth ? lead + styledLabel : truncateToWidth(lead.trimStart() || label, nameWidth, "");
      let lines: string[];
      if (kind === "more") {
        lines = [title];
      } else if (!aligned) {
        const fields = kind === "workflow" ? [phase, input, output] : [model, context, input, output];
        lines = renderWrappedRow(title, fields, leadWidth, separator, theme, width);
      } else {
        const middle =
          kind === "workflow"
            ? padToWidth(phase, middleWidth)
            : padToWidth(model, modelWidth) + rightAlign(context, middleWidth - modelWidth);
        const metadata = [middle, rightAlign(input, inputWidth), rightAlign(output, outputWidth)].join(COLUMN_GAP);
        lines = [padToWidth(title, titleWidth) + separator + theme.fg("dim", metadata)];
      }
      return {
        section: kind === "agent" ? "Agents" : "Workflows",
        lines: selected ? lines.map((line) => theme.bg("selectedBg", padToWidth(line, width))) : lines,
      };
    },
  );

  const selectedIndex = entries.findIndex((entry) => entry.selected);
  const selectedHeight = body[selectedIndex]?.lines.length ?? 1;
  const bodyLimit = Math.max(Math.floor(maxBodyLines), selectedHeight + 1);
  return bodyWindow(body, Math.max(0, selectedIndex), bodyLimit, theme, width);
}

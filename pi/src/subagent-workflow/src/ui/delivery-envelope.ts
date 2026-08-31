export const DELIVERY_ENVELOPE_BUDGET = 16_000;
export const ENVELOPE_LINE_MAX = 2_000;
const FIXED_SECTIONS_TRUNCATED = "[fixed sections truncated]";

export interface DeliveryEnvelopeSections {
  /** Identity, durable location, and terminal status. */
  header: readonly string[];
  /** Actionable errors. */
  failures?: readonly string[];
  /** Recovery invocations and instructions that must survive truncation. */
  recovery?: readonly string[];
  warnings?: readonly string[];
  /** Durable locations a reader needs to recover the full result. */
  artifacts?: readonly string[];
  /** Convenience locations, such as child transcripts, discoverable from the mandatory run record. */
  auxiliaryArtifacts?: readonly string[];
  /** Already-bounded verification summary. */
  toolActivity?: string;
  /** Serialized result text. This is the only truncatable section. */
  resultPreview?: string;
  /** Required when resultPreview can be truncated. */
  truncationMarker?: string;
}

function boundedLines(lines: readonly string[] | undefined): string[] {
  return lines?.flatMap((value) => value.split("\n"))
    .filter((line) => line.length > 0)
    .map((line) => line.length <= ENVELOPE_LINE_MAX ? line : `${line.slice(0, ENVELOPE_LINE_MAX - 3)}...`) ?? [];
}

function boundedSlice(value: string, length: number): string {
  let sliced = value.slice(0, Math.max(0, length));
  const last = sliced.charCodeAt(sliced.length - 1);
  if (last >= 0xd800 && last <= 0xdbff) sliced = sliced.slice(0, -1);
  return sliced;
}

/**
 * Build one priority-ordered delivery envelope. Callers sanitize untrusted
 * fields while constructing sections; this helper bounds every line and the
 * complete envelope.
 */
export function buildDeliveryEnvelope(sections: DeliveryEnvelopeSections, budget = DELIVERY_ENVELOPE_BUDGET): string {
  const limit = Number.isFinite(budget) ? Math.max(0, Math.floor(budget)) : DELIVERY_ENVELOPE_BUDGET;
  if (limit === 0) return "";

  const header = boundedLines(sections.header);
  const failures = boundedLines(sections.failures);
  const recovery = boundedLines(sections.recovery);
  const warnings = boundedLines(sections.warnings);
  const artifacts = boundedLines(sections.artifacts);
  const auxiliaryArtifacts = boundedLines(sections.auxiliaryArtifacts);
  const toolActivity = boundedLines(sections.toolActivity ? [sections.toolActivity.replace(/^\n/, "")] : []);
  const previewSupplied = sections.resultPreview !== undefined;
  const fixed = [
    ...header,
    ...failures,
    ...recovery,
    ...warnings,
    ...artifacts,
    ...auxiliaryArtifacts,
    ...toolActivity,
    ...(previewSupplied ? ["Result preview:"] : []),
  ];
  const fixedBlock = fixed.join("\n");
  if (!previewSupplied && fixedBlock.length <= limit) return fixedBlock;

  const full = `${fixedBlock}\n${sections.resultPreview ?? ""}`;
  if (previewSupplied && full.length <= limit) return full;

  const optional = [
    ...header.map((line) => ({ section: "header" as const, line })),
    ...failures.map((line) => ({ section: "failures" as const, line })),
    ...warnings.map((line) => ({ section: "warnings" as const, line })),
    ...auxiliaryArtifacts.map((line) => ({ section: "auxiliaryArtifacts" as const, line })),
    ...toolActivity.map((line) => ({ section: "toolActivity" as const, line })),
  ];
  const truncationMarker = sections.truncationMarker ?? "[truncated]";
  let includeFixedMarker = false;
  let includeTruncationMarker = false;
  let selected: typeof optional = [];
  let preview = "";
  let includePreviewFrame = false;

  // Marker requirements are monotonic: reserving either marker can only omit
  // more optional text or result text, never make the marker unnecessary.
  for (let pass = 0; pass < 3; pass += 1) {
    const reserved: string[] = [
      ...recovery,
      ...artifacts,
      ...(includeFixedMarker ? [FIXED_SECTIONS_TRUNCATED] : []),
      ...(includeTruncationMarker ? [truncationMarker] : []),
    ];
    let used: number = reserved.join("\n").length;
    let partCount = reserved.length;
    selected = [];
    if (used <= limit) {
      for (const entry of optional) {
        const cost = entry.line.length + (partCount > 0 ? 1 : 0);
        if (used + cost > limit) break;
        selected.push(entry);
        used += cost;
        partCount += 1;
      }
    }
    const previewOverhead = "Result preview:".length + 1 + (partCount > 0 ? 1 : 0);
    includePreviewFrame = previewSupplied && used + previewOverhead <= limit;
    const previewBudget = includePreviewFrame ? limit - used - previewOverhead : 0;
    preview = includePreviewFrame ? boundedSlice(sections.resultPreview!, previewBudget) : "";
    const needsFixedMarker: boolean = selected.length < optional.length || used > limit;
    const needsTruncationMarker = previewSupplied && preview.length < sections.resultPreview!.length;
    if (needsFixedMarker === includeFixedMarker && needsTruncationMarker === includeTruncationMarker) break;
    includeFixedMarker ||= needsFixedMarker;
    includeTruncationMarker ||= needsTruncationMarker;
  }

  const mandatory = [
    ...recovery,
    ...artifacts,
    ...(includeFixedMarker ? [FIXED_SECTIONS_TRUNCATED] : []),
    ...(includeTruncationMarker ? [truncationMarker] : []),
  ];
  if (mandatory.join("\n").length > limit) {
    const markers = [
      ...(includeFixedMarker ? [FIXED_SECTIONS_TRUNCATED] : []),
      ...(includeTruncationMarker ? [truncationMarker] : []),
    ];
    const markerBlock = markers.join("\n");
    if (markerBlock.length > limit) return boundedSlice(markerBlock, limit);

    let used = markerBlock.length;
    let partCount = markers.length;
    const admitWholeLines = (lines: readonly string[]): string[] => {
      const admitted: string[] = [];
      for (const line of lines) {
        const cost = line.length + (partCount > 0 ? 1 : 0);
        if (used + cost > limit) continue;
        admitted.push(line);
        used += cost;
        partCount += 1;
      }
      return admitted;
    };
    const admittedRecovery = admitWholeLines(recovery);
    const admittedArtifacts = admitWholeLines(artifacts);
    return [...admittedRecovery, ...admittedArtifacts, ...markers].join("\n");
  }

  const linesFor = (section: typeof optional[number]["section"]): string[] => selected
    .filter((entry) => entry.section === section)
    .map((entry) => entry.line);
  return [
    ...linesFor("header"),
    ...linesFor("failures"),
    ...recovery,
    ...linesFor("warnings"),
    ...artifacts,
    ...linesFor("auxiliaryArtifacts"),
    ...linesFor("toolActivity"),
    ...(includePreviewFrame ? ["Result preview:", preview] : []),
    ...(includeFixedMarker ? [FIXED_SECTIONS_TRUNCATED] : []),
    ...(includeTruncationMarker ? [truncationMarker] : []),
  ].join("\n");
}

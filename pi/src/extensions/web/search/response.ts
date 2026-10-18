/** Remove fragments and tracking parameters so both providers use the same URL identity. */
export function cleanUrl(raw: unknown): string | undefined {
  if (typeof raw !== "string") return undefined;
  try {
    const url = new URL(raw);
    if (url.protocol !== "http:" && url.protocol !== "https:") return undefined;
    url.hash = "";
    url.searchParams.delete("accessToken");
    for (const key of [...url.searchParams.keys()]) {
      if (key.startsWith("utm_")) url.searchParams.delete(key);
    }
    return url.toString();
  } catch {
    return undefined;
  }
}

/** Only the publication day is useful for source selection. */
export function displayDate(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  return /^(\d{4}-\d{2}-\d{2})(?:T|$)/.exec(value)?.[1];
}

export function boundedText(value: unknown, maximum: number): string {
  return typeof value === "string" ? value.trim().slice(0, maximum) : "";
}

export function parseJson(value: string): unknown {
  try {
    return JSON.parse(value) as unknown;
  } catch {
    return undefined;
  }
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

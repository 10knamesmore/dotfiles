/** Complete reader output, before caching or pagination. */
export interface WebDocument {
  url: string;
  title: string;
  content: string;
  format: "markdown" | "text";
}

export interface WebReader {
  /** Match supported URL shapes without network or process I/O. */
  supports(url: URL): boolean;
  /** Read fresh content; all I/O shares the caller's cancellation and deadline. */
  read(url: URL, signal: AbortSignal): Promise<WebDocument>;
}

/** Search backends exposed by the distribution's websearch tool. */
export type SearchProvider = "exa" | "codex";

/** One query sent to the selected backend; filters remain provider-side constraints. */
export interface SearchRequest {
  query: string;
  /** Maximum number of sources returned to the agent, from 1 to 20. */
  limit: number;
  /** Publication recency requested from the backend. */
  recency?: "day" | "week" | "month" | "year";
  /** Hostnames to include; a leading minus excludes a hostname. */
  domains?: string[];
}

/** A linked source, retaining the provider's excerpt rather than inventing a summary. */
export interface SearchSource {
  title: string;
  url: string;
  snippet: string;
}

/** Provider answer and sources; an empty source list is distinct from a transport error. */
export interface SearchResult {
  provider: SearchProvider;
  answer: string;
  sources: SearchSource[];
}

/** One query sent to DeepSeek's search API. */
export interface SearchRequest {
  query: string;
  /** The user's original question when `query` is a keyword rewrite; helps the backend understand intent. */
  oriQuestion?: string;
  /** Maximum number of sources returned to the agent, from 1 to 20. */
  limit: number;
}

/** One ranked source: title and URL identity, with a model-readable summary when available. */
export interface SearchSource {
  title: string;
  url: string;
  summary: string;
  /** Publication date as a `YYYY-MM-DD` prefix, absent when unknown. */
  date?: string;
  siteName?: string;
  /** Webpage authority from 0 to 4; absent when the API reports unknown. */
  authority?: number;
  /** Composite relevance, authority and timeliness score from 0 to 3. */
  rerankScore?: number;
}

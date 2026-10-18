/** One query searched by both DeepSeek and Exa. */
export interface SearchRequest {
  query: string;
  /** The user's original question when `query` is a keyword rewrite; sent to DeepSeek for intent. */
  oriQuestion?: string;
  /** Maximum number of sources returned to the agent, from 1 to 20. */
  limit: number;
}

/** One ranked source: title and URL identity, with a model-readable summary when available. */
export interface SearchSource {
  /** Search engines that returned this URL; blended duplicates retain both. */
  providers: ("deepseek" | "exa")[];
  title: string;
  url: string;
  summary: string;
  /** Publication date as a `YYYY-MM-DD` prefix, absent when unknown. */
  date?: string;
  siteName?: string;
  /** DeepSeek webpage authority from 0 to 4; absent when unknown. */
  authority?: number;
  /** DeepSeek relevance, authority and timeliness score from 0 to 3, not the blended rank. */
  rerankScore?: number;
}

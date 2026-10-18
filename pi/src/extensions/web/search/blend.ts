import { searchDeepseek } from "./deepseek.js";
import { searchExa } from "./exa.js";
import type { SearchRequest, SearchSource } from "./types.js";

const RANK_CONSTANT = 60;

/** Both engines must succeed. Cancelling or failing either request stops its sibling. */
export async function searchBlended(request: SearchRequest, signal?: AbortSignal): Promise<SearchSource[]> {
  const controller = new AbortController();
  const requestSignal = signal ? AbortSignal.any([signal, controller.signal]) : controller.signal;
  try {
    const results = await Promise.all([
      searchDeepseek(request, requestSignal),
      searchExa(request, requestSignal),
    ]);
    requestSignal.throwIfAborted();
    return blendSources(results, request.limit);
  } finally {
    controller.abort();
  }
}

/**
 * Equal-weight reciprocal rank fusion: sum 1 / (60 + one-based rank) for each URL.
 * Provider scores are not comparable. Ties retain DeepSeek-first insertion order;
 * duplicate URLs keep DeepSeek's summary/metadata and record both providers.
 * https://www.elastic.co/docs/reference/elasticsearch/rest-apis/reciprocal-rank-fusion
 */
function blendSources(results: SearchSource[][], limit: number): SearchSource[] {
  const ranked = new Map<string, { source: SearchSource; score: number }>();
  for (const sources of results) {
    for (const [index, source] of sources.entries()) {
      const score = 1 / (RANK_CONSTANT + index + 1);
      const existing = ranked.get(source.url);
      if (existing) {
        existing.score += score;
        existing.source.providers.push(...source.providers);
      } else {
        ranked.set(source.url, { source: { ...source, providers: [...source.providers] }, score });
      }
    }
  }
  return [...ranked.values()]
    .sort((left, right) => right.score - left.score)
    .slice(0, limit)
    .map(({ source }) => source);
}

import { randomUUID } from "node:crypto";
import { mkdir, readdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { WebError } from "../errors.js";
import type { FetchedPage } from "./extract.js";

const CACHE_TTL_MS = 60 * 60 * 1000;
const MAX_CACHE_ENTRIES = 128;
const MAX_CACHE_BYTES = 128 * 1024 * 1024;
const RESPONSE_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

/** Immutable downloaded content, addressable across reloads until expiry or cache eviction. */
export interface CachedPage extends FetchedPage {
  responseId: string;
  fetchedAt: number;
}

/** On-disk public-page cache. Each fetch gets a new ID so pagination never switches to a newer download. */
export class PageCache {
  constructor(private readonly directory: string) {}

  async save(page: FetchedPage): Promise<CachedPage> {
    const cached: CachedPage = { ...page, responseId: randomUUID(), fetchedAt: Date.now() };
    const text = JSON.stringify(cached);
    if (Buffer.byteLength(text) > MAX_CACHE_BYTES)
      throw new WebError("too_large", "Extracted page exceeds the cache size limit.");
    await mkdir(this.directory, { recursive: true, mode: 0o700 });
    const path = join(this.directory, `${cached.responseId}.json`);
    const temporary = `${path}.tmp`;
    try {
      await writeFile(temporary, text, { mode: 0o600, flag: "wx" });
      await rename(temporary, path);
    } finally {
      await rm(temporary, { force: true });
    }
    await this.prune(cached.responseId);
    return cached;
  }

  async read(responseId: string): Promise<CachedPage> {
    if (!RESPONSE_ID.test(responseId)) throw new WebError("invalid_input", "Use a responseId returned by webfetch.");
    const path = join(this.directory, `${responseId}.json`);
    let data: unknown;
    try {
      const metadata = await stat(path);
      if (metadata.size > MAX_CACHE_BYTES)
        throw new WebError("cache_miss", "Cached content is unavailable; fetch the URL again.");
      data = JSON.parse(await readFile(path, "utf8"));
    } catch (error) {
      if (error instanceof WebError) throw error;
      if ((error as NodeJS.ErrnoException).code !== "ENOENT" && !(error instanceof SyntaxError)) throw error;
      throw new WebError("cache_miss", "Cached content is unavailable; fetch the URL again.");
    }
    if (!isCachedPage(data) || data.responseId !== responseId) {
      throw new WebError("cache_miss", "Cached content is invalid; fetch the URL again.");
    }
    if (Date.now() - data.fetchedAt >= CACHE_TTL_MS) {
      await rm(path, { force: true });
      throw new WebError("cache_miss", "Cached content expired; fetch the URL again.");
    }
    return data;
  }

  private async prune(keepId: string): Promise<void> {
    const entries = await Promise.all(
      (await readdir(this.directory))
        .filter((name) => name.endsWith(".json") && RESPONSE_ID.test(name.slice(0, -5)))
        .map(async (name) => {
          const path = join(this.directory, name);
          try {
            const metadata = await stat(path);
            return { path, name, bytes: metadata.size, created: metadata.mtimeMs };
          } catch (error) {
            // Other Pi processes can evict a page between the directory listing and stat.
            if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined;
            throw error;
          }
        }),
    );
    const sorted = entries
      .filter((entry) => entry !== undefined)
      .sort(
        (a, b) => Number(b.name === `${keepId}.json`) - Number(a.name === `${keepId}.json`) || b.created - a.created,
      );
    let bytes = 0;
    let count = 0;
    for (const entry of sorted) {
      const keep = entry.name === `${keepId}.json`;
      if (
        !keep &&
        (Date.now() - entry.created >= CACHE_TTL_MS ||
          count >= MAX_CACHE_ENTRIES ||
          bytes + entry.bytes > MAX_CACHE_BYTES)
      ) {
        await rm(entry.path, { force: true });
      } else {
        count += 1;
        bytes += entry.bytes;
      }
    }
  }
}

function isCachedPage(value: unknown): value is CachedPage {
  if (!value || typeof value !== "object") return false;
  const page = value as Partial<CachedPage>;
  return (
    typeof page.responseId === "string" &&
    typeof page.fetchedAt === "number" &&
    Number.isFinite(page.fetchedAt) &&
    typeof page.url === "string" &&
    typeof page.title === "string" &&
    typeof page.contentType === "string" &&
    typeof page.content === "string"
  );
}

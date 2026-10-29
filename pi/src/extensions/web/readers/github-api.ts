import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { WebError } from "../errors.js";

const executeFile = promisify(execFile);
const MAX_OUTPUT_BYTES = 5 * 1024 * 1024;

/** Uses gh's existing credentials; the host and GET method do not depend on shell or repository defaults. */
export class GitHubApi {
  constructor(private readonly signal: AbortSignal) {}

  async json<T>(endpoint: string, mediaType = "application/vnd.github+json"): Promise<T> {
    return JSON.parse(
      (await this.request(endpoint, ["--header", `Accept: ${mediaType}`])).toString("utf8"),
    ) as T;
  }

  async list<T>(endpoint: string): Promise<T[]> {
    const pages = JSON.parse(
      (await this.request(endpoint, ["--paginate", "--slurp"])).toString("utf8"),
    ) as T[][];
    return pages.flat();
  }

  async text(endpoint: string): Promise<string> {
    const bytes = await this.request(endpoint, ["--header", "Accept: application/vnd.github.raw+json"]);
    let text: string;
    try {
      text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    } catch {
      throw new WebError("unsupported", "The GitHub file is not UTF-8 text.");
    }
    if (text.includes("\0")) throw new WebError("unsupported", "Binary GitHub files are not supported.");
    return text;
  }

  private async request(endpoint: string, options: string[] = []): Promise<Buffer> {
    this.signal.throwIfAborted();
    try {
      const { stdout } = await executeFile(
        "gh",
        ["api", "--hostname", "github.com", "--method", "GET", endpoint, ...options],
        { signal: this.signal, encoding: "buffer", maxBuffer: MAX_OUTPUT_BYTES },
      );
      return stdout;
    } catch (error) {
      this.signal.throwIfAborted();
      const { code, stderr } = error as { code?: string | number; stderr?: Buffer };
      if (code === "ENOENT") throw new WebError("command", "GitHub reading requires gh to be installed in PATH.");
      if (code === "ERR_CHILD_PROCESS_STDIO_MAXBUFFER") {
        throw new WebError("too_large", "GitHub CLI output exceeds 5 MiB.");
      }
      const statusMatch = stderr?.toString("utf8").match(/\(HTTP (\d{3})\)/);
      if (statusMatch) {
        const status = Number(statusMatch[1]);
        throw new WebError(
          status === 401 ? "authentication" : "http",
          `GitHub returned HTTP ${status} for ${endpoint}.`,
          status,
        );
      }
      if (code === 4) throw new WebError("authentication", "GitHub reading requires gh auth login.");
      throw new WebError(
        "command",
        `gh api failed while reading ${endpoint}. Check gh auth status and repository access.`,
      );
    }
  }
}

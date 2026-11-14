import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { createWriteStream } from "node:fs";
import { access, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";
import { readPythonEnvironment } from "../python/environment.js";
import type { DesignChanges, DesignProject, DesignState } from "./types.js";

const WEB_DIRECTORY = fileURLToPath(new URL("./web/dist/", import.meta.url));

/** One native service and its Vite preview live until this Pi session shuts down. */
export class DesignService {
  private constructor(
    public readonly url: string,
    private readonly child: ChildProcessWithoutNullStreams,
    private readonly exited: Promise<void>,
  ) {}

  static async start(agentDirectory: string, sessionId: string): Promise<DesignService> {
    await access(join(WEB_DIRECTORY, "index.html")).catch(() => {
      throw new Error("Design editor is not built. Run dots sync.");
    });
    const environment = await readPythonEnvironment();
    const logDirectory = join(agentDirectory, "design", "logs");
    await mkdir(logDirectory, { recursive: true });
    const logPath = join(logDirectory, `${sessionId}.log`);
    const log = createWriteStream(logPath, { flags: "a" });
    const child = spawn(
      environment.executable,
      [
        "-u",
        "-c",
        "import sys, pi_design; pi_design.serve(sys.argv[1], sys.argv[2], sys.argv[3])",
        agentDirectory,
        WEB_DIRECTORY,
        sessionId,
      ],
      { stdio: ["pipe", "pipe", "pipe"], env: process.env },
    );
    child.stderr.pipe(log, { end: false });
    const exited = new Promise<void>((resolve) => {
      child.once("close", () => {
        log.end();
        resolve();
      });
    });
    const lines = createInterface({ input: child.stdout });
    try {
      const url = await new Promise<string>((resolve, reject) => {
        const timer = setTimeout(() => {
          reject(new Error(`Design service did not start. See ${logPath}`));
        }, 20_000);
        const finish = (error?: Error, url?: string): void => {
          clearTimeout(timer);
          if (error) reject(error);
          else resolve(url!);
        };
        child.once("error", (error) => finish(error));
        child.once("exit", (code) => finish(new Error(`Design service exited (${code}). See ${logPath}`)));
        lines.on("line", (line) => {
          try {
            const ready = JSON.parse(line) as { url?: string };
            if (ready.url) finish(undefined, ready.url);
          } catch {
            log.write(`${line}\n`);
          }
        });
      });
      return new DesignService(url, child, exited);
    } catch (error) {
      child.stdin.end();
      child.kill();
      throw error;
    }
  }

  async request<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(`${this.url}${path}`, {
      method: body === undefined ? "GET" : "POST",
      headers: body === undefined ? undefined : { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(30_000),
    });
    if (!response.ok) throw new Error(`Design: ${await response.text()}`);
    return (await response.json()) as T;
  }

  async projects(): Promise<DesignProject[]> {
    return (await this.request<{ projects: DesignProject[] }>("/api/projects")).projects;
  }

  state(): Promise<DesignState> {
    return this.request("/api/state");
  }

  changes(since: number): Promise<DesignChanges> {
    return this.request(`/api/changes?since=${since}`);
  }

  open(path: string): Promise<DesignState> {
    return this.request("/api/open", { path });
  }

  async close(): Promise<void> {
    this.child.stdin.end();
    const timer = setTimeout(() => this.child.kill("SIGTERM"), 5_000);
    try {
      await this.exited;
    } finally {
      clearTimeout(timer);
    }
  }
}

export async function openBrowser(url: string): Promise<void> {
  const child = spawn(process.platform === "darwin" ? "open" : "xdg-open", [url], {
    stdio: "ignore",
  });
  await new Promise<void>((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`Could not open ${url} (exit ${code}).`));
    });
  });
}

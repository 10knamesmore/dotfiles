import { randomBytes } from "node:crypto";
import { readFile } from "node:fs/promises";
import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import { Type } from "typebox";
import { Value } from "typebox/value";
import { logPythonEvent } from "../diagnostics.js";
import type { PythonSession } from "../session.js";
import type { InspectRequest } from "./types.js";

const Offset = Type.Optional(Type.Integer({ minimum: 0, maximum: Number.MAX_SAFE_INTEGER }));
const Query = Type.Union([
  Type.Object({ view: Type.Literal("variables"), search: Type.Optional(Type.String({ maxLength: 512 })), definitions: Type.Optional(Type.Boolean()), offset: Offset }, { additionalProperties: false }),
  Type.Object({ view: Type.Literal("value"), name: Type.String({ minLength: 1, maxLength: 4096 }), path: Type.Array(Type.Object({ kind: Type.Union([Type.Literal("index"), Type.Literal("entry")]), index: Type.Integer({ minimum: 0, maximum: Number.MAX_SAFE_INTEGER }) }, { additionalProperties: false }), { maxItems: 32 }), offset: Offset }, { additionalProperties: false }),
  Type.Object({ view: Type.Literal("terminals") }, { additionalProperties: false }),
  Type.Object({ view: Type.Literal("terminal"), id: Type.String({ minLength: 1, maxLength: 256 }), mode: Type.Union([Type.Literal("screen"), Type.Literal("raw")]), since: Offset }, { additionalProperties: false }),
]);

const ASSETS = new Map([
  ["/", { name: "index.html", contentType: "text/html; charset=utf-8" }],
  ["/app.js", { name: "app.js", contentType: "text/javascript; charset=utf-8" }],
  ["/style.css", { name: "style.css", contentType: "text/css; charset=utf-8" }],
]);

/** Session-owned loopback server. Its only application operations read state. */
export class PythonObserverServer {
  private readonly token = randomBytes(32).toString("hex");
  private server?: Server;
  private starting?: Promise<string>;
  private origin?: string;

  public constructor(private readonly getSession: () => PythonSession | undefined) {}

  public get isOpen(): boolean {
    return this.starting !== undefined;
  }

  public open(): Promise<string> {
    return (this.starting ??= this.listen().catch((error: unknown) => {
      this.starting = undefined;
      throw error;
    }));
  }

  public async close(): Promise<void> {
    if (this.starting) await this.starting.catch(() => undefined);
    const server = this.server;
    this.server = undefined;
    this.starting = undefined;
    if (!server) return;
    server.closeAllConnections();
    await new Promise<void>((resolve) => server.close(() => resolve()));
    this.log("observer_server_closed");
  }

  private async listen(): Promise<string> {
    // Fail before opening a port if the distributed UI is incomplete.
    const assets = new Map<string, { bytes: Buffer; contentType: string }>();
    for (const [path, asset] of ASSETS) {
      assets.set(path, { bytes: await readFile(new URL(`./ui/${asset.name}`, import.meta.url)), contentType: asset.contentType });
    }
    const server = createServer((request, response) => {
      void this.handle(request, response, assets).catch((error: unknown) => {
        this.log("observer_http_failed", error instanceof Error ? error.message : String(error));
        if (!response.headersSent) this.json(response, 500, { error: "internal_error" });
        else response.end();
      });
    });
    await new Promise<void>((resolve, reject) => {
      server.once("error", reject);
      server.listen(0, "127.0.0.1", () => {
        server.removeListener("error", reject);
        resolve();
      });
    });
    server.on("error", (error) => this.log("observer_server_failed", error.message));
    this.server = server;
    const port = (server.address() as AddressInfo).port;
    this.origin = `http://127.0.0.1:${port}`;
    this.log("observer_server_started", `port=${port}`);
    return `${this.origin}/#token=${this.token}`;
  }

  private async handle(
    request: IncomingMessage,
    response: ServerResponse,
    assets: Map<string, { bytes: Buffer; contentType: string }>,
  ): Promise<void> {
    response.setHeader("Cache-Control", "no-store");
    response.setHeader("X-Content-Type-Options", "nosniff");
    response.setHeader("Referrer-Policy", "no-referrer");
    response.setHeader("Content-Security-Policy", "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'");
    if (`http://${request.headers.host}` !== this.origin || (request.headers.origin && request.headers.origin !== this.origin)) {
      this.json(response, 403, { error: "unauthorized" });
      return;
    }
    const url = new URL(request.url ?? "/", this.origin);
    const path = url.pathname;
    const asset = assets.get(path);
    if (request.method === "GET" && asset) {
      response.writeHead(200, { "Content-Type": asset.contentType });
      response.end(asset.bytes);
      return;
    }
    if (request.headers.authorization !== `Bearer ${this.token}`) {
      this.json(response, 401, { error: "unauthorized" });
      return;
    }
    const session = this.getSession();
    if (!session) {
      this.json(response, 503, { error: "unavailable" });
      return;
    }
    if (request.method === "GET" && path === "/api/status") {
      this.json(response, 200, await session.observeStatus(url.searchParams.get("output") === "1"));
      return;
    }
    if (request.method === "POST" && path === "/api/inspect") {
      const query = await this.readQuery(request);
      if (!query) {
        this.json(response, 400, { error: "invalid_request" });
        return;
      }
      const result = await session.inspect(query);
      if (!response.destroyed) this.json(response, 200, result);
      return;
    }
    this.json(response, 404, { error: "invalid_request" });
  }

  private async readQuery(request: IncomingMessage): Promise<InspectRequest | undefined> {
    if (request.headers["content-type"]?.split(";")[0]?.trim() !== "application/json") return undefined;
    let size = 0;
    const chunks: Buffer[] = [];
    try {
      for await (const chunk of request.iterator({ destroyOnReturn: false }) as AsyncIterable<Buffer>) {
        size += chunk.length;
        if (size > 16_384) {
          request.resume();
          return undefined;
        }
        chunks.push(chunk);
      }
      const query: unknown = JSON.parse(Buffer.concat(chunks).toString("utf8"));
      return Value.Check(Query, query) ? query as InspectRequest : undefined;
    } catch {
      return undefined;
    }
  }

  private json(response: ServerResponse, status: number, data: unknown): void {
    response.writeHead(status, { "Content-Type": "application/json; charset=utf-8" });
    response.end(JSON.stringify(data));
  }

  private log(phase: string, reason?: string): void {
    logPythonEvent({ sessionId: this.getSession()?.status().sessionId ?? "closed", phase, reason });
  }
}

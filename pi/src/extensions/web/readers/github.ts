import { WebError } from "../errors.js";
import { GitHubApi } from "./github-api.js";
import type { WebDocument, WebReader } from "./reader.js";

const GITHUB_SITE_ROUTES = new Set([
  "account",
  "apps",
  "collections",
  "customer-stories",
  "enterprise",
  "events",
  "explore",
  "features",
  "issues",
  "login",
  "marketplace",
  "notifications",
  "orgs",
  "organizations",
  "pulls",
  "readme",
  "search",
  "security",
  "settings",
  "signup",
  "site",
  "sponsors",
  "topics",
  "trending",
  "users",
]);

type GitHubTarget = { repository: string } & (
  | { kind: "repository" }
  | { kind: "issue" | "pull"; number: string }
  | { kind: "blob" | "tree"; referenceAndPath: string }
);

interface Repository {
  full_name: string;
  description: string | null;
  html_url: string;
  default_branch: string;
}

interface Discussion {
  title: string;
  number: number;
  body: string | null;
  state: string;
  html_url: string;
  user: { login: string } | null;
  created_at: string;
  updated_at: string;
  merged_at?: string | null;
  draft?: boolean;
  base?: { label: string };
  head?: { label: string };
  pull_request?: { html_url: string };
}

interface Comment {
  body: string | null;
  html_url: string;
  user: { login: string } | null;
  created_at?: string;
  submitted_at?: string | null;
  state?: string;
  path?: string;
  line?: number | null;
  original_line?: number | null;
  in_reply_to_id?: number;
}

interface GitReference {
  ref: string;
}

interface DirectoryEntry {
  name: string;
  type: string;
  html_url: string | null;
}

type RepositoryContent =
  | { type: "dir"; entries: DirectoryEntry[] }
  | { type: "file" | "symlink" | "submodule"; path: string; html_url: string };

export const githubReader: WebReader = {
  supports: (url) => parseGitHubUrl(url) !== undefined,
  async read(url, signal) {
    const target = parseGitHubUrl(url);
    if (!target) throw new WebError("unsupported", "This URL is not supported by the GitHub reader.");
    const api = new GitHubApi(signal);
    switch (target.kind) {
      case "repository":
        return readRepository(api, target.repository);
      case "issue":
      case "pull":
        return readDiscussion(api, target);
      case "blob":
      case "tree":
        return readRepositoryPath(api, target, url);
    }
  },
};

function parseGitHubUrl(url: URL): GitHubTarget | undefined {
  if (url.hostname !== "github.com" || url.port) return undefined;
  const match = /^\/([a-z0-9-]+\/[a-z0-9_.-]+)(?:\/(.*?))?\/?$/i.exec(url.pathname);
  if (!match) return undefined;
  const repository = match[1]!;
  if (GITHUB_SITE_ROUTES.has(repository.split("/")[0]!.toLowerCase())) return undefined;
  const path = match[2] ?? "";
  if (!path) return { kind: "repository", repository };
  const discussion = /^(issues|pull)\/([1-9]\d*)$/.exec(path);
  if (discussion) {
    return { kind: discussion[1] === "issues" ? "issue" : "pull", repository, number: discussion[2]! };
  }
  const file = /^(blob|tree)\/(.+)$/.exec(path);
  if (file) {
    return { kind: file[1] as "blob" | "tree", repository, referenceAndPath: file[2]! };
  }
  return undefined;
}

async function readRepository(api: GitHubApi, repository: string): Promise<WebDocument> {
  const details = await api.json<Repository>(`repos/${repository}`);
  const readme = await api.text(`repos/${repository}/readme`);
  return {
    url: details.html_url,
    title: details.full_name,
    format: "markdown",
    content: [
      `# ${details.full_name}`,
      details.description,
      `Default branch: ${details.default_branch}`,
      "## README",
      readme,
    ]
      .filter((part) => part != null)
      .join("\n\n"),
  };
}

async function readDiscussion(
  api: GitHubApi,
  target: GitHubTarget & { kind: "issue" | "pull" },
): Promise<WebDocument> {
  const root = `repos/${target.repository}`;
  let isPull = target.kind === "pull";
  let discussion = await api.json<Discussion>(`${root}/${isPull ? "pulls" : "issues"}/${target.number}`);
  if (!isPull && discussion.pull_request) {
    isPull = true;
    discussion = await api.json<Discussion>(`${root}/pulls/${target.number}`);
  }
  const comments = await api.list<Comment>(`${root}/issues/${target.number}/comments?per_page=100`);
  const title = `${target.repository} #${discussion.number}: ${discussion.title}`;
  const sections = [
    `# ${title}`,
    [
      `State: ${discussion.merged_at ? "merged" : discussion.state}${discussion.draft ? " (draft)" : ""}`,
      `Author: ${discussion.user?.login ?? "deleted user"}`,
      `Created: ${discussion.created_at}`,
      `Updated: ${discussion.updated_at}`,
      ...(isPull ? [`Branches: ${discussion.head!.label} → ${discussion.base!.label}`] : []),
      discussion.html_url,
    ].join("\n"),
    discussion.body ?? "",
    ...commentSection("Comments", comments),
  ];
  if (isPull) {
    const reviews = await api.list<Comment>(`${root}/pulls/${target.number}/reviews?per_page=100`);
    const reviewComments = await api.list<Comment>(`${root}/pulls/${target.number}/comments?per_page=100`);
    sections.push(...commentSection("Reviews", reviews), ...commentSection("Review comments", reviewComments));
  }
  return { url: discussion.html_url, title, format: "markdown", content: sections.join("\n\n") };
}

function commentSection(heading: string, comments: Comment[]): string[] {
  if (comments.length === 0) return [];
  return [
    `## ${heading}`,
    ...comments.map((comment) => {
      const location = comment.path
        ? `${comment.path}:${comment.line ?? comment.original_line ?? "outdated"}`
        : undefined;
      return [
        `### ${comment.user?.login ?? "deleted user"}`,
        [comment.state, comment.created_at ?? comment.submitted_at, location].filter(Boolean).join(" · "),
        comment.in_reply_to_id ? `Reply to comment ${comment.in_reply_to_id}` : undefined,
        comment.html_url,
        comment.body,
      ]
        .filter((part) => part != null && part !== "")
        .join("\n\n");
    }),
  ];
}

async function readRepositoryPath(
  api: GitHubApi,
  target: GitHubTarget & { kind: "blob" | "tree" },
  url: URL,
): Promise<WebDocument> {
  const { ref, path } = await resolveRepositoryPath(api, target.repository, target.referenceAndPath);
  const endpoint = `repos/${target.repository}/contents/${encodePath(path)}?ref=${encodeURIComponent(ref)}`;
  const contents = await api.json<RepositoryContent>(endpoint, "application/vnd.github.object+json");
  const title = `${target.repository} / ${path || "."} @ ${ref}`;
  if (contents.type === "file") {
    return {
      url: contents.html_url,
      title,
      content: await api.text(endpoint),
      format: /\.(md|markdown|mdown)$/i.test(path) ? "markdown" : "text",
    };
  }
  if (contents.type !== "dir") {
    throw new WebError("unsupported", `GitHub ${contents.type} content is not supported.`);
  }
  const entries = contents.entries;
  return {
    url: url.toString(),
    title,
    format: "markdown",
    content: [
      `# ${title}`,
      entries
        .map((entry) => {
          const name = (entry.name + (entry.type === "dir" ? "/" : "")).replace(/[\\[\]*_`]/g, "\\$&");
          return entry.html_url ? `- [${name}](<${entry.html_url}>)` : `- ${name}`;
        })
        .join("\n"),
      // The Contents API does not paginate directory listings.
      ...(entries.length === 1_000 ? ["GitHub's Contents API returns at most 1,000 directory entries."] : []),
    ].join("\n\n"),
  };
}

/** Branch and tag names can contain slashes; resolve them before separating the file path. */
async function resolveRepositoryPath(
  api: GitHubApi,
  repository: string,
  encodedReferenceAndPath: string,
): Promise<{ ref: string; path: string }> {
  let referenceAndPath: string;
  try {
    referenceAndPath = decodeURIComponent(encodedReferenceAndPath);
  } catch {
    throw new WebError("invalid_input", "The GitHub path contains invalid URL encoding.");
  }
  const firstSegment = referenceAndPath.split("/")[0]!;
  if (/^[0-9a-f]{7,40}$/i.test(firstSegment) || firstSegment === "HEAD") {
    return { ref: firstSegment, path: referenceAndPath.slice(firstSegment.length + 1) };
  }
  const referenceLists = await Promise.all(
    ["heads", "tags"].map((namespace) =>
      api.list<GitReference>(`repos/${repository}/git/matching-refs/${namespace}/${encodeURIComponent(firstSegment)}`),
    ),
  );
  const candidates = referenceLists
    .flat()
    .map(({ ref }) => ref.replace(/^refs\/(heads|tags)\//, ""))
    .filter((name) => referenceAndPath === name || referenceAndPath.startsWith(`${name}/`))
    .sort((a, b) => b.length - a.length);
  const match = candidates[0];
  if (!match) throw new WebError("invalid_input", "No GitHub branch or tag matches this URL.");
  return { ref: match, path: referenceAndPath.slice(match.length + 1) };
}

function encodePath(path: string): string {
  return path.split("/").map(encodeURIComponent).join("/");
}

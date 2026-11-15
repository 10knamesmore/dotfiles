// One OpenUI Lang example per component. They are compiled by the real parser, so they double as tests.
import { SAMPLE_PNG_DATA_URI } from "../data/sample-image.ts";

export interface LangSample {
  key: string;
  title: string;
  source: string;
}

export const LANG_SAMPLES: LangSample[] = [
  {
    key: "dashboard",
    title: "Stack, Text, Metric, Card, BarChart, Button",
    source: `root = Stack([title, kpis, load, go])
title = Text("Cluster status", "title")
kpis = Stack([m1, m2, m3], "row")
m1 = Metric("GPU util", "87%", "+4% vs 1h ago", "warn")
m2 = Metric("Nodes up", "4 / 4", "all healthy", "good")
m3 = Metric("Queue", "12", "-3", "good")
load = Card([chart], "Load by node")
chart = BarChart(["spark-1", "spark-2", "spark-3", "spark-4"], [91, 84, 77, 62], null, "%")
go = Button("Show slowest requests", @ToAssistant("Show the 10 slowest vLLM requests from the last hour"))`,
  },
  {
    key: "text",
    title: "Text variants and Callout tones",
    source: `root = Stack([t1, t2, t3, t4, t5, c1, c2, c3, c4], "column", "s")
t1 = Text("Title text", "title")
t2 = Text("Heading text", "heading")
t3 = Text("Body text is the default and wraps to the available width, so a long sentence flows onto the next line instead of running off the edge.")
t4 = Text("Muted text for secondary notes", "muted")
t5 = Text("code_variant(42)", "code")
c1 = Callout("Heads up: this is an info callout.", "info")
c2 = Callout("Deploy finished without errors.", "success")
c3 = Callout("Disk usage is above 80% on spark-3.", "warn")
c4 = Callout("Health check failed twice in a row.", "error")`,
  },
  {
    key: "table",
    title: "Table (numbers right-aligned) with tooltips",
    source: `root = Stack([tbl])
tbl = Table(["Node", "State", "GPU %", "Uptime"], [["spark-1", "serving", 91, "12 d"], ["spark-2", "serving", 84, "12 d"], ["spark-3", "draining", 77, "3 d"], ["spark-4", "serving", 62, "12 d"]], [t1, t2])
t1 = Tooltip("row:3", "Rebooted 21 Oct after a driver update", "The node was drained first, then rebooted.\\n\\n- queue paused\\n- 3 requests migrated", "sudo systemctl restart vllm", "bash", "warn")
t2 = Tooltip("col:GPU %", "Average over the last 5 minutes")`,
  },
  {
    key: "tabs",
    title: "Tabs",
    source: `root = Stack([tabs])
tabs = Tabs([overview, nodes, notes])
overview = TabItem("Overview", [kpis])
kpis = Stack([m1, m2], "row")
m1 = Metric("Nodes up", "4 / 4", "all healthy", "good")
m2 = Metric("Queue", "12", "-3", "good")
nodes = TabItem("Nodes", [tbl])
tbl = Table(["Node", "State"], [["spark-1", "serving"], ["spark-2", "serving"]])
notes = TabItem("Notes", [note])
note = Callout("spark-3 was rebooted on 21 Oct.", "info")`,
  },
  {
    key: "interactive",
    title: "Button, Choice, Faq",
    source: `root = Stack([b, ch, fq])
b = Stack([b1, b2], "row")
b1 = Button("Restart vllm", @ToAssistant("Restart the vllm service on dgx-spark-1"))
b2 = Button("Drain node")
ch = Choice("Which model should serve chat?", ["qwen3-32b", "llama-3.3-70b", "gemma-3-27b"], "I will use it for the next request")
fq = Faq([q1, q2, q3], true)
q1 = FaqItem("What does draining do?", "It stops **new** requests and lets running ones finish.\\n\\n1. queue pauses\\n2. in-flight requests complete\\n3. node is marked *drained*")
q2 = FaqItem("How do I undo it?", "Run \`uncordon\` on the node: it rejoins the pool immediately.")
q3 = FaqItem("Why did spark-3 reboot?", "A driver update needed it. See the [runbook](https://example.com/runbook).")`,
  },
  {
    key: "choice-card",
    title: "Choice: several questions, one Send",
    source: `root = Stack([plan])
plan = Card([q1, q2, q3], "Before I restart")
q1 = Choice("Which node should I restart first?", ["web-1", "web-2"])
q2 = Choice("Which checks should I run?", ["Disk", "Memory", "Network"], null, true)
q3 = Choice("Clear the cache on db?", ["Yes", "No"], null, null, false)`,
  },
  {
    key: "markdown",
    title: "Markdown and CodeBlock",
    source: `root = Stack([md, code])
md = Markdown("# Release notes\\n\\nThis release adds **streaming**, *better* retries and \`--fast\` mode.\\n\\n- faster cold start\\n- fewer 5xx errors\\n  - nested item\\n\\n> Upgrade during a quiet window.\\n\\n| Model | Tokens/s |\\n|---|---|\\n| qwen3 | 82 |\\n| llama | 41 |\\n\\n---\\nSee [docs](https://example.com).")
code = CodeBlock("def serve(model, port=8000):\\n    # start the server\\n    cfg = {\\"model\\": model, \\"port\\": port}\\n    for i in range(3):\\n        print(f\\"attempt {i}\\", cfg)\\n    return True", "python", "serve.py")`,
  },
  {
    key: "code-langs",
    title: "CodeBlock: js, sql, yaml, json, bash, go, rust",
    source: `root = Stack([a, b, c, d, e, f, g], "column", "s")
a = CodeBlock("const res = await fetch(url, { method: \\"POST\\" });\\nconsole.log(res.status); // 200", "js")
b = CodeBlock("SELECT id, count(*) AS n\\nFROM jobs WHERE status = 'done' -- finished\\nGROUP BY id LIMIT 10;", "sql")
c = CodeBlock("server:\\n  port: 8080\\n  tls: true # enabled", "yaml")
d = CodeBlock("{\\"name\\": \\"vllm\\", \\"gpus\\": 8, \\"ok\\": true}", "json")
e = CodeBlock("for f in *.log; do\\n  echo $f | grep -c ERROR\\ndone", "bash")
f = CodeBlock("func main() {\\n\\tfmt.Println(\\"hi\\", 42)\\n}", "go")
g = CodeBlock("fn main() {\\n    let x: i32 = 42; // answer\\n    println!(\\"{}\\", x);\\n}", "rust")`,
  },
  {
    key: "line",
    title: "LineChart (natural, linear, step)",
    source: `root = Stack([natural, linear, step])
natural = LineChart(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"], [s1, s2], "natural", "day", "requests (k)", null, [t1, t2, t3])
linear = LineChart(["Mon", "Tue", "Wed", "Thu", "Fri"], [s3], "linear")
step = LineChart(["Mon", "Tue", "Wed", "Thu", "Fri"], [s3], "step")
s1 = Series("api", [12, 19, 15, 25, 32, 28, 35])
s2 = Series("worker", [8, 11, 14, 12, 18, 22, 20])
s3 = Series("errors", [3, 7, 4, 9, 5])
t1 = Tooltip("Thu", "Release v2.3 went out at 14:00", "Traffic jumped 60% after the release. The worker queue grew too, but drained by Friday.\\n\\n- cache warm-up took 20 minutes\\n- two retries logged", "kubectl rollout history deploy/api", "bash", "warn")
t2 = Tooltip("Sun", "Weekend peak: a batch import")
t3 = Tooltip("series:worker", "Background jobs: indexing and emails")`,
  },
  {
    key: "area",
    title: "AreaChart (stacked)",
    source: `root = Stack([c])
c = AreaChart(["00:00", "04:00", "08:00", "12:00", "16:00", "20:00"], [a, b, d], "natural", null, "tokens/min", null, [t])
a = Series("qwen3", [10, 14, 30, 42, 38, 20])
b = Series("llama", [5, 8, 15, 22, 18, 9])
d = Series("gemma", [2, 3, 8, 10, 9, 4])
t = Tooltip("12:00", "Lunch peak", "All three models peak together at midday. qwen3 alone is over half of the tokens.", null, null, "info")`,
  },
  {
    key: "pie",
    title: "PieChart (pie, donut, semi-circular)",
    source: `root = Stack([pie, row])
pie = PieChart(["qwen3-32b", "llama-3.3-70b", "gemma-3-27b", "embeddings", "free"], [38, 52, 18, 6, 14], null, null, [t1, t2, t3])
t1 = Tooltip("llama-3.3-70b", "Biggest tenant: the support bot", "Serves 41% of the cluster's memory. Pinned to spark-2 and spark-3 with tensor parallelism.", "vllm serve llama-3.3-70b --tensor-parallel-size 2", "bash", "info")
t2 = Tooltip("free", "About one more 14 GB model fits", null, null, null, "good")
t3 = Tooltip("embeddings", "Small but always loaded")
row = Stack([donut, semi], "row")
donut = PieChart(["a", "b", "c"], [5, 3, 2], "donut")
semi = PieChart(["x", "y", "z"], [4, 3, 1], "pie", "semiCircular")`,
  },
  {
    key: "radar",
    title: "RadarChart",
    source: `root = Stack([r])
r = RadarChart(["Speed", "Cost", "Quality", "Context", "Tools"], [a, b], [t1, t2])
t1 = Tooltip("Cost", "Higher is cheaper per million tokens")
t2 = Tooltip("series:llama", "llama-3.3-70b, 4-bit", "Best quality and context of the two, at the price of speed.", null, null, "warn")
a = Series("qwen3", [80, 90, 70, 60, 85])
b = Series("llama", [55, 60, 88, 90, 70])`,
  },
  {
    key: "geo",
    title: "GeoMap with a link and tooltips",
    source: `root = Stack([m])
m = GeoMap([tokyo, sf, fra, syd], [["Tokyo", "Frankfurt"]], "Regions", [t])
tokyo = GeoPoint("Tokyo", 35.68, 139.69, "good")
sf = GeoPoint("San Francisco", 37.77, -122.42, "info")
fra = GeoPoint("Frankfurt", 50.11, 8.68, "warn")
syd = GeoPoint("Sydney", -33.87, 151.21)
t = Tooltip("Tokyo", "Primary region", "Serves APAC traffic. Failover is Sydney.")`,
  },
  {
    key: "dag",
    title: "Dag with statuses and a tooltip",
    source: `root = Stack([flow])
flow = Dag([n1, n2, n3, n4, n5, n6], [["build", "unit"], ["build", "lint"], ["build", "bench"], ["unit", "package"], ["lint", "package"], ["bench", "deploy"]], null, "CI run", [t])
n1 = DagNode("build", "Build image", "done")
n2 = DagNode("unit", "Unit tests", "done")
n3 = DagNode("lint", "Lint", "failed")
n4 = DagNode("bench", "Benchmark", "running")
n5 = DagNode("package", "Package", "skipped")
n6 = DagNode("deploy", "Deploy", "pending")
t = Tooltip("lint", "2 rules failed", "ruff reported two violations in src/api.", "ruff check src/", "bash", "bad")`,
  },
  {
    key: "erd",
    title: "Erd with a column tooltip",
    source: `root = Stack([schema])
schema = Erd([users, projects, jobs], [r1, r2], "Job queue schema", [t])
users = Entity("users", ["id uuid PK", "email text UNIQUE", "created_at timestamptz"])
projects = Entity("projects", ["id uuid PK", "owner_id uuid FK", "name text"])
jobs = Entity("jobs", ["id uuid PK", "project_id uuid FK", "status text"])
r1 = Relation("users", "projects", "1:N", "owns")
r2 = Relation("projects", "jobs", "1:N", "runs")
t = Tooltip("jobs.status", "queued | running | done | failed")`,
  },
  {
    key: "infra",
    title: "InfraMap",
    source: `root = Stack([map])
map = InfraMap([z1, z2, z3], [l1, l2, l3, l4], "Web platform", null, [t])
z1 = InfraZone("Edge", [lb, cdn], "info")
z2 = InfraZone("Application", [api, worker])
z3 = InfraZone("Data", [db, cache], "good")
lb = InfraNode("lb", "Load balancer", "gateway", "up", "HTTPS :443")
cdn = InfraNode("cdn", "CDN", "cloud", "up")
api = InfraNode("api", "API", "service", "up", "REST")
worker = InfraNode("worker", "Worker", "service", "degraded", "2 of 3 running")
db = InfraNode("db", "Postgres", "database", "up")
cache = InfraNode("cache", "Redis", "database", "up")
l1 = InfraLink("cdn", "lb", "cache miss")
l2 = InfraLink("lb", "api", "HTTP")
l3 = InfraLink("api", "db", "SQL")
l4 = InfraLink("api", "cache", "get / set", "dashed")
t = Tooltip("worker", "One replica is down", "2 of 3 workers are running.", "kubectl get pods -l app=worker", "bash", "warn")`,
  },
  {
    key: "sequence",
    title: "SequenceDiagram with alt/else, note, self-call",
    source: `root = Stack([d])
d = SequenceDiagram([user, api, cache, db], [m1, g, m5, m6], "Cache lookup", true, [t])
user = SeqParticipant("user", "User", "actor")
api = SeqParticipant("api", "API", "service")
cache = SeqParticipant("cache", "Cache", "database")
db = SeqParticipant("db", "Postgres", "database")
m1 = SeqMessage("user", "api", "GET /item/42", "sync", "req")
g = SeqGroup("alt", "cache hit?", [m2, m3], [m4, n1, m4b], "miss")
m2 = SeqMessage("api", "cache", "get item:42")
m3 = SeqMessage("cache", "api", "item", "return")
m4 = SeqMessage("api", "db", "SELECT * FROM items")
n1 = SeqNote("db", "slow path: about 40 ms")
m4b = SeqMessage("db", "api", "row", "return")
m5 = SeqMessage("api", "api", "serialize")
m6 = SeqMessage("api", "user", "200 OK", "return")
t = Tooltip("req", "The entry point", "Authenticated by the gateway before it reaches the API.")`,
  },
  {
    key: "markdown-image",
    title: "Markdown with a picture on a line of its own",
    source: `root = Stack([doc])
doc = Markdown("## A picture in the text\\n\\nThe line below is only an image: it is drawn by the terminal, between the paragraphs.\\n\\n![Sunset over the ridge](${SAMPLE_PNG_DATA_URI} \\"Generated test picture\\")\\n\\nText continues underneath, and a path with spaces goes in angle brackets.")`,
  },
  {
    key: "image",
    title: "Image (a real picture where the terminal can show one) with a tooltip",
    source: `root = Stack([img])
img = Image("${SAMPLE_PNG_DATA_URI}", "Sunset over the ridge", "Generated test picture, 192x108", [t])
t = Tooltip("image", "Test picture, drawn by the extension", "Pictures use the terminal's image protocol (Kitty, Ghostty, WezTerm, Warp, herdr). Other terminals get a labelled frame with the file details.", null, null, "info")`,
  },
];

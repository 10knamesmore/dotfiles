# OpenUI Lang

Write OpenUI Lang statements in a top-level `openui` fenced code block in the assistant reply.

## Syntax

1. One statement per line: `name = Expression`. `root = Stack([...])` comes first.
2. Expressions: "strings" (backslash escapes, use \n for newlines), numbers, true/false, null, [arrays], {objects}, Component(args), @Step(args), references to other names.
3. Arguments are POSITIONAL. Write Metric("CPU", "38%", "+2%", "good"), never delta: "+2%". Skip an optional argument in the middle with null.
4. Every name except root must be referenced by another statement, or it is an error (it would not render). References may be used before they are defined.
5. Helper components (TabItem, DagNode, Entity, Relation, Series, GeoPoint, FaqItem, Infra*, Seq*, Tooltip) belong only in their parent's argument.
6. The widget is drawn in a terminal of about 100 columns: keep labels short, tables under about 12 rows.

## Components

```text
Stack(children: component[], direction?: "row" | "column", gap?: "none" | "xs" | "s" | "m" | "l" | "xl", wrap?: boolean) — Layout container. direction "column" (default) or "row". gap none|xs|s|m|l|xl (default m). wrap lets a row wrap.
Text(text: string, variant?: "title" | "heading" | "body" | "muted" | "code") — A line or paragraph of text (default body).
Card(children: component[], title?: string) — Bordered panel with an optional title.
Metric(label: string, value: string, delta?: string, tone?: "good" | "warn" | "bad" | "neutral", tooltips?: Tooltip[]) — KPI tile. value and delta are strings. tone colours the delta.
Table(columns: string[], rows: ((string | number)[])[], tooltips?: Tooltip[]) — Data table. rows is an array of rows, each the same length as columns (strings or numbers). Click headers to sort, cells to view/copy their full value, or copy CSV to export the displayed row order. Tooltip row numbers identify the original data rows even after sorting.
BarChart(labels: string[], values: number[], title?: string, unit?: string, tooltips?: Tooltip[]) — Bar chart of one series. labels and values are parallel arrays. unit is a suffix such as %.
Callout(text: string, tone?: "info" | "success" | "warn" | "error") — Highlighted note.
Button(label: string, action?: action) — Clickable button. Without an action it sends its label to the assistant.
TabItem(label: string, content: component[]) — One tab. Only valid inside Tabs.
Tabs(items: TabItem[]) — Tabbed container.
Markdown(text: string) — Rich text from markdown: headings, **bold**, *italic*, `code`, lists, > quotes, tables, fenced code, ---. A line that is only ![alt](path "caption") draws a picture (a local file or data: URI, as for Image).
CodeBlock(code: string, language?: string, title?: string) — Syntax highlighted code. language: python|js|ts|bash|sql|yaml|json|go|rust (others unhighlighted). Click copies it.
Image(src: string, alt?: string, caption?: string, tooltips?: Tooltip[]) — An image rendered through the Kitty graphics protocol. Other terminals show a labelled frame with the file name and size.
DagNode(id: string, label: string, status?: "done" | "running" | "failed" | "pending" | "skipped") — One DAG node.
Dag(nodes: DagNode[], edges: string[][], direction?: "LR" | "TB", title?: string, tooltips?: Tooltip[]) — Directed graph with automatic layered layout. edges are [fromId, toId] pairs. direction omitted = automatic.
Entity(name: string, fields: string[]) — ERD table. fields are strings like "id uuid PK", "user_id uuid FK", "email text UNIQUE".
Relation(from: string, to: string, kind?: "1:1" | "1:N" | "N:1" | "N:M", label?: string) — ERD relation between two entity names. 1:N (default): the first entity is the one side.
Erd(entities: Entity[], relations: Relation[], title?: string, tooltips?: Tooltip[]) — Entity-relationship diagram with crow's-foot relations.
GeoPoint(label: string, lat: number, lon: number, tone?: "good" | "warn" | "bad" | "info" | "neutral") — Map pin in decimal degrees.
GeoMap(points: GeoPoint[], links?: string[][], title?: string, tooltips?: Tooltip[]) — Offline world map (coarse coastlines) zoomed to fit the pins. links are [labelA, labelB] pairs drawn dashed.
FaqItem(question: string, answer: string) — One Q&A entry (answer is markdown). Only valid inside Faq.
Faq(items: FaqItem[], firstOpen?: boolean) — Collapsible Q&A list.
Choice(question: string, options: string[], hint?: string, multiple?: boolean, other?: boolean) — A question for the USER. Answers are staged and sent together as ONE message when the user presses Send answers. multiple (4th argument) true allows several options. other (5th argument, default true) adds an Other... row where the user types their own answer; false hides it.
Series(category: string, values: number[]) — One data series: name and numbers parallel to the chart labels.
LineChart(labels: string[], series: Series[], variant?: "linear" | "natural" | "step", xLabel?: string, yLabel?: string, height?: number, tooltips?: Tooltip[]) — Line chart over categories, several series. variant natural (smooth, default), linear or step.
AreaChart(labels: string[], series: Series[], variant?: "linear" | "natural" | "step", xLabel?: string, yLabel?: string, height?: number, tooltips?: Tooltip[]) — Filled STACKED area chart: series stack into a growing total.
PieChart(labels: string[], values: number[], variant?: "pie" | "donut", appearance?: "circular" | "semiCircular", tooltips?: Tooltip[]) — Parts of a whole, at most 8 slices. Values are raw numbers, not percentages.
RadarChart(labels: string[], series: Series[], tooltips?: Tooltip[]) — Radar chart: one axis per label, each series parallel to labels.
InfraNode(id: string, label: string, kind?: "client" | "gateway" | "service" | "gpu" | "database" | "storage" | "queue" | "cloud", status?: "up" | "degraded" | "down" | "unknown", detail?: string) — One box inside an InfraZone. id is unique across the map.
InfraZone(name: string, nodes: InfraNode[], tone?: "neutral" | "info" | "good" | "warn" | "bad") — A labelled box that groups nodes.
InfraLink(from: string, to: string, label?: string, style?: "solid" | "dashed") — An arrow between two InfraNode ids.
InfraMap(zones: InfraZone[], links: InfraLink[], title?: string, direction?: "LR" | "TB", tooltips?: Tooltip[]) — Infrastructure map: zones are columns (LR, default) or rows (TB).
SeqParticipant(id: string, label: string, kind?: "actor" | "service" | "database" | "queue" | "external") — One column of a SequenceDiagram.
SeqMessage(from: string, to: string, label: string, style?: "sync" | "async" | "return", id?: string) — An arrow between participant ids. from == to is a self-call.
SeqNote(over: string, text: string, to?: string) — A note over a participant, or spanning to another.
SeqGroup(kind: "alt" | "opt" | "loop" | "par", label: string, steps: (SeqMessage | SeqNote | SeqGroup)[], elseSteps?: (SeqMessage | SeqNote | SeqGroup)[], elseLabel?: string) — A frame around steps. elseSteps adds a dashed divider and a second branch.
SequenceDiagram(participants: SeqParticipant[], steps: (SeqMessage | SeqNote | SeqGroup)[], title?: string, numbered?: boolean, tooltips?: Tooltip[]) — Calls between participants: columns left to right, steps top to bottom.
Tooltip(target: string, hint?: string, detail?: string, code?: string, language?: string, tone?: "info" | "good" | "warn" | "bad", link?: string, linkLabel?: string) — Hover hint with an optional click-to-open panel for one element of the host component.
```

## Tooltips

`tooltips: Tooltip[]` is the LAST argument of Dag, Erd, InfraMap, SequenceDiagram, Table, Metric, BarChart, LineChart, AreaChart, PieChart, RadarChart, GeoMap and Image, so write null for each skipped optional argument before it, e.g. Dag(nodes, edges, null, null, [t1]).
Targets: Dag node id; Erd entity name or entity.column; InfraMap node id, zone:<zone name> or <fromId>><toId>; SequenceDiagram participant id or SeqMessage id; Table row:3, col:status or cell:3.status; Metric its own label; BarChart a bar label; GeoMap a point label or A>B; Image the word image; LineChart and AreaChart an x label (that point) or series:<name>; PieChart a slice label; RadarChart an axis label or series:<name>.
An element that has a tooltip shows a small ◆ on it. Hovering shows the hint; clicking (or Enter) opens the panel with detail, code and link. Use a tooltip for context the reader may want but should not have to read: why a point spiked, what a slice contains, what a node does.
hint is the short text shown on hover (140 characters). detail (800), code (40 lines) and link (http/https) open in a panel when the element is clicked.

## Limits

A widget over a limit is not drawn, and the error says which. Source 100 KB, 1500 components, 16 levels of nesting, 40 children per Stack or Card, 12 tabs. Text 8000 characters, Markdown 30000, any other string 2000, a table cell 300. Table 100 rows by 16 columns. BarChart 40 bars. LineChart and AreaChart 200 points and 8 series. RadarChart 16 axes. Dag 60 nodes and 150 edges. Erd 20 entities, 30 fields each, 40 relations. InfraMap 30 nodes and 45 links. GeoMap 100 points and 60 links. Faq 30 items. Pictures 12 (Image components and markdown picture lines together). CodeBlock 400 lines. Choices 12 per widget, a question 200 characters, an option 120, 6 options, the user's Other answer 500. A SequenceDiagram 100 steps. Prefer far less: the reader sees one terminal screen.

## Actions

Button("Restart", @ToAssistant("Restart the vllm service")) sends that text to the assistant as the user's next turn (500 characters at most). A Button without an action sends its label. Several steps: Action([@ToAssistant("...")]). Do not use @OpenUrl: put links in a Text.
Choice(question, options, hint, multiple, other) asks the user. A click only stages an answer; the user's Send answers button sends every Choice of the widget together as ONE message (one single question gives "<question> — <option>"; otherwise a numbered list, "(no answer)" for a skipped question, "other: ..." for typed text). multiple: true allows several options; other (default true) adds an Other... row for the user's own answer. Write null for a skipped optional argument: Choice("Which checks?", ["Disk", "Memory"], null, true).

## Choosing components

Metric for a number, Table for comparisons, BarChart for one series of categories, LineChart for trends with several series, AreaChart for stacked volume, PieChart for parts of a whole (8 slices at most), RadarChart for comparing entities across metrics, Dag for task graphs and pipelines, Erd for database schemas, InfraMap for how systems connect, SequenceDiagram for call flows, GeoMap for places, Tabs/Card to organise. Series values must have the same length as labels. All data is literal: no queries, no variables. Image shows a real picture in terminals that support it (Kitty, Ghostty, WezTerm, Warp, herdr): src is an absolute file path (png, jpg, gif, webp) or a data: URI.

## Example

```openui
root = Stack([title, kpis, load, go])
title = Text("Cluster status", "title")
kpis = Stack([m1, m2, m3], "row")
m1 = Metric("GPU util", "87%", "+4% vs 1h ago", "warn")
m2 = Metric("Nodes up", "4 / 4", "all healthy", "good")
m3 = Metric("Queue", "12", "-3", "good")
load = Card([chart], "Load by node")
chart = BarChart(["spark-1", "spark-2", "spark-3", "spark-4"], [91, 84, 77, 62], null, "%")
go = Button("Show slowest requests", @ToAssistant("Show the 10 slowest vLLM requests from the last hour"))
```

# Pi Design

## Files

Each project is a normal React/Vite project. `design.json` stores editable
content and parameters; `.design/workspace.json` stores artboards and comments.
Both are ordinary JSON files that the model may edit with its file tools.
The service reads external edits and pushes the resulting state to the editor.

```json
{
  "name": "Music Player",
  "elements": {
    "title": { "text": "Now playing", "style": { "fontSize": 32 } }
  },
  "parameters": {
    "coverSize": {
      "label": "Cover size",
      "control": "range",
      "value": 240,
      "min": 120,
      "max": 400,
      "step": 4
    }
  }
}
```

Parameter controls are `range`, `number`, `color`, `text`, `boolean`, and
`select`. Select options are strings. Styles use React CSS property names.

```json
{
  "artboards": [
    {
      "id": "player",
      "name": "Player",
      "route": "/",
      "x": 80,
      "y": 80,
      "width": 1200,
      "height": 800
    }
  ],
  "comments": [
    {
      "id": "comment-1",
      "text": "Make this more prominent",
      "anchor": { "artboard": "player", "element": "title" },
      "resolved": false
    }
  ]
}
```

A coordinate anchor is `{ "artboard": "player", "x": 420, "y": 180 }`,
in unscaled artboard CSS pixels. Element IDs refer to `data-design-id` in
the preview. The runtime supplies that attribute.

## Local service

One service belongs to one Pi session and has one active project. Project
registration is global in `<agent-dir>/design/projects.json`:
`{ "projects": [{ "id": "...", "name": "...", "path": "..." }] }`.
Project IDs are stable for a canonical directory.

The native module exposes
`serve(agent_dir, web_dir, session_id)` for the extension's child process.
It binds an ephemeral loopback port, writes
`<agent-dir>/design/sessions/<session-id>.json` with `{ "url": "http://127.0.0.1:..." }`,
and prints the same object as one flushed stdout line when ready.
The parent closes stdin to shut down the service and its preview process.

The Python SDK locates that endpoint using `PI_PYTHON_SESSION_ID` and
`PI_DESIGN_AGENT_DIR` (default `~/.pi/agent`). An explicit `endpoint` argument
allows standalone inspection. Python JSON results are native dict/list objects.

HTTP routes:

| Method | Route | Body / result |
| --- | --- | --- |
| GET | `/api/projects` | Global `{ projects: [...] }` |
| GET | `/api/preferences` | Global editor `{ theme: "dark" | "light" }`; defaults to dark |
| POST | `/api/preferences` | `{ theme: "dark" | "light" }` → saved preferences |
| POST | `/api/open` | `{ path, name?, start_preview?: true }` → full state; register and select |
| GET | `/api/state` | Full state |
| GET | `/api/changes?since=N` | `{ revision, changes: [...] }` |
| POST | `/api/edit` | `{ edits: [...], snapshot?: boolean }` → full state |
| POST | `/api/undo` | Full state |
| POST | `/api/redo` | Full state |
| POST | `/api/history` | `{ id: number }` → restore a design-data snapshot, return full state |
| POST | `/api/selection` | `{ artboard: string|null, element: string|null }` → selection |
| GET | `/api/events` | SSE `state` with full state; SSE `error` with `{error}` |

Full state:

```ts
type State = {
  project: { id: string; name: string; path: string } | null;
  revision: number;
  design: {
    name: string;
    elements: Record<string, { text?: string; style?: Record<string, string | number> }>;
    parameters: Record<string, {
      label?: string;
      control: "range" | "number" | "color" | "text" | "boolean" | "select";
      value: string | number | boolean;
      min?: number;
      max?: number;
      step?: number;
      options?: string[];
    }>;
  };
  workspace: {
    artboards: Array<{
      id: string; name: string; route: string;
      x: number; y: number; width: number; height: number;
    }>;
    comments: Array<{
      id: string; text: string; resolved: boolean;
      anchor: { artboard: string; element?: string; x?: number; y?: number };
    }>;
  };
  selection: { artboard: string | null; element: string | null };
  preview_url: string | null;
  can_undo: boolean;
  can_redo: boolean;
  history: {
    current: number;
    entries: Array<{ id: number; summary: string }>;
  } | null;
};
```

Each change contains `{ revision, file, patch }`; `patch` is an RFC 6902
JSON Patch describing actual file contents changing, including external edits.
`read_changes` is read-only. It never marks edits as submitted.

History snapshots contain `design.json` and `.design/workspace.json`. The
service keeps them while the project remains open. Switching projects or
restarting the service begins a new history; React source files are outside
these snapshots. Selecting an earlier snapshot preserves later entries for
redo. Editing from that point replaces the later entries.

`/api/edit` accepts semantic edits:

```ts
type Edit =
  | { type: "text"; element: string; text: string }
  | { type: "style"; element: string; style: Record<string, string | number | null> }
  | { type: "parameter"; name: string; value: string | number | boolean }
  | { type: "artboard"; artboard: {
      id: string; name: string; route: string;
      x: number; y: number; width: number; height: number;
    } }
  | { type: "remove_artboard"; id: string }
  | { type: "comment"; comment: {
      id: string; text: string; resolved: boolean;
      anchor: { artboard: string; element?: string; x?: number; y?: number };
    } }
  | { type: "remove_comment"; id: string };
```

The service reloads files before editing. It writes only changed files.
External file edits participate in state updates and history. Invalid JSON
reports an error and is never silently replaced with defaults.

`snapshot` defaults to `true`. A parameter slider sends `snapshot: false`
during a drag, so values still persist and update the page. On pointer release,
it sends `{ edits: [], snapshot: true }` after the queued values to create one
history entry. A gesture ending at its starting value creates no snapshot.

Editor preferences live in `<agent-dir>/design/preferences.json` and do not
change a project's appearance, revision history, or model context.

Opening creates missing `design.json` and `.design/workspace.json` only, with
empty elements/parameters and one default artboard. It does not scaffold React.
If `package.json` exists and `start_preview` is true, the service launches
`pnpm exec vite --host 127.0.0.1 --port <free-port> --strictPort` in the project.
The preview process belongs to the service. Selecting another project stops it.

## Browser bridge

The editor renders the project in iframes. It appends `__pi_design=<service-url>`
and `__pi_artboard=<id>` query parameters. The React runtime reads those once,
connects to the service for state updates, and uses `window.parent.postMessage`
for editor interactions. Outside the editor it renders from the imported
`design.json` and has no service dependency.

Messages have `source: "pi-design"`.

Editor → runtime:

- `{ type: "mode", mode: "select" | "comment" | "play" }`
- `{ type: "select", element: string|null }`

Runtime → editor:

- `{ type: "ready", artboard }`
- `{ type: "selection", artboard, element, rect, text, tag }`
- `{ type: "comment", artboard, element?, x, y }`
- `{ type: "bounds", artboard, elements: [{ id, tag, text, rect }] }`

Rects use artboard viewport coordinates `{ x, y, width, height }`.
Bounds also include `kind: "text" | "box"` so only text components expose
inline text editing; editing containers preserves their React children.
Receivers check source window and origin; browser messages are UI data.
Runtime text edits and resize gestures persist through `/api/edit`.
The UI never sends model prompts.

The runtime's `@pi-design/react/vite` plugin sends design JSON changes as
data updates rather than full-page reloads. This keeps typing and drag gestures
active while their values are saved. Normal source changes still use Vite HMR.

## Python interface

```python
import pi_design

pi_design.list_projects()
project = pi_design.open("/absolute/path", name="Music Player")
project = pi_design.current()
project.read_state()
project.read_changes(since=0)
project.read_element("title")
project.read_artboard("player")
project.resolve_comment("comment-1")
project.undo()
project.redo()
```

`open` and `current` return a `Project`; it exposes `id`, `path`, `url`
(the editor URL). Module calls accept optional keyword-only `endpoint`.
`Project` methods verify the active project ID rather than reading another
project after the session selection changes. Source edits use existing file
tools. Screenshots use browser-use.

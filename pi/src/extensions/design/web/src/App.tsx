import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { Check, CircleAlert, LoaderCircle, Moon, Redo2, Sun, Undo2, X } from "lucide-react";
import { DesignConnection } from "@pi-design/react/client";
import type { CommentAnchor, DesignComment, ElementBounds, Mode } from "@pi-design/react/protocol";
import { Canvas } from "./Canvas";
import type { CanvasHandle } from "./Canvas";
import { Comments } from "./Comments";
import { Inspector } from "./Inspector";
import { Layers } from "./Layers";
import { IconButton } from "./ui";

export function App() {
  const connection = useMemo(() => new DesignConnection(new URLSearchParams(location.search).get("service") ?? location.origin), []);
  const snapshot = useSyncExternalStore(connection.subscribe, connection.getSnapshot);
  const { state, pending, error, failed, connected } = snapshot;
  const [theme, setTheme] = useState<"dark" | "light">("dark");
  const [mode, setMode] = useState<Mode>("select");
  const [tab, setTab] = useState<"design" | "comments">("design");
  const [bounds, setBounds] = useState<Record<string, ElementBounds[]>>({});
  const [draft, setDraft] = useState<CommentAnchor | null>(null);
  const [activeComment, setActiveComment] = useState<string | null>(null);
  const canvas = useRef<CanvasHandle>(null);
  useEffect(() => connection.start(), [connection]);
  useEffect(() => {
    void connection.request<{ theme: "dark" | "light" }>("/api/preferences")
      .then(preferences => setTheme(preferences.theme))
      .catch(error => connection.report(error));
  }, [connection]);
  useEffect(() => { document.documentElement.dataset.theme = theme; }, [theme]);
  function toggleTheme() {
    const next = theme === "dark" ? "light" : "dark";
    setTheme(next);
    void connection.request("/api/preferences", { theme: next }).catch(error => connection.report(error));
  }
  useEffect(() => { setBounds({}); setDraft(null); setActiveComment(null); }, [state?.project?.id]);
  useEffect(() => {
    function keyboard(event: KeyboardEvent) {
      const target = event.target;
      if (target instanceof HTMLElement && (target.matches("input,textarea,select") || target.isContentEditable)) return;
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
        event.preventDefault();
        void connection.history(event.shiftKey ? "redo" : "undo");
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "y") {
        event.preventDefault(); void connection.history("redo");
      }
      if (event.key === "Escape") { setDraft(null); setMode("select"); }
    }
    window.addEventListener("keydown", keyboard);
    return () => window.removeEventListener("keydown", keyboard);
  }, [connection]);
  const receiveBounds = useCallback((artboard: string, elements: ElementBounds[]) => setBounds(previous => ({ ...previous, [artboard]: elements })), []);
  const comment = useCallback((anchor: CommentAnchor) => { setDraft(anchor); setTab("comments"); setActiveComment(null); }, []);
  const selectComment = useCallback((comment: DesignComment) => {
    setActiveComment(comment.id);
    setDraft(null);
    setTab("comments");
    void connection.select({ artboard: comment.anchor.artboard, element: comment.anchor.element ?? null });
  }, [connection]);
  const element = state?.selection.artboard ? bounds[state.selection.artboard]?.find(item => item.id === state.selection.element) : undefined;

  return <div className="app">
    <header className="topbar">
      <div className="brand" title="Pi Design"><svg width="23" height="23" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <rect x="2" y="2" width="8" height="8" rx="2" fill="currentColor" /><rect x="14" y="2" width="8" height="8" rx="2" fill="currentColor" opacity=".5" />
        <rect x="2" y="14" width="8" height="8" rx="2" fill="currentColor" opacity=".5" /><path d="M14 14h8v4a4 4 0 0 1-4 4h-4v-8Z" fill="currentColor" /></svg>
      </div>
      <span className="project-switcher">{state?.project?.name ?? "Pi Design"}</span>
      <span className="topbar-divider" />
      <IconButton icon={Undo2} title="Undo · ⌘Z / Ctrl+Z" disabled={!state?.can_undo || pending > 0} onClick={() => void connection.history("undo")} />
      <IconButton icon={Redo2} title="Redo · ⇧⌘Z / Ctrl+Shift+Z" disabled={!state?.can_redo || pending > 0} onClick={() => void connection.history("redo")} />
      <div className="topbar-center"><span className="document-dot" />{state?.design.name ?? "Pi Design"}</div>
      <div className={`save-status ${failed ? "save-failed" : ""}`} title={connected ? "Service connected" : "Reconnecting to service"}>
        {failed ? <CircleAlert size={14} /> : pending ? <LoaderCircle className="spin" size={14} /> : connected ? <Check size={14} /> : <LoaderCircle className="spin" size={14} />}
        <span>{failed ? "Not saved" : pending ? "Saving" : connected ? "Saved" : "Reconnecting"}</span>
      </div>
      <IconButton className="theme-toggle" icon={theme === "dark" ? Sun : Moon}
        title={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"} onClick={toggleTheme} />
    </header>
    {error && <div className="error-banner" role="alert"><CircleAlert size={16} /><span>{error}</span>
      <button onClick={connection.retry}>Retry</button>
      {failed && <button onClick={connection.discard}>Discard edits</button>}
      {!failed && <IconButton icon={X} title="Dismiss error" onClick={connection.dismissError} />}
    </div>}
    {state?.project ? <main className="workspace">
      <Layers state={state} connection={connection} bounds={bounds} onFocus={id => canvas.current?.focus(id)} pending={pending > 0} />
      <Canvas ref={canvas} state={state} connection={connection} mode={mode} onMode={setMode} bounds={bounds} onBounds={receiveBounds}
        onComment={comment} onCommentClick={selectComment} activeComment={activeComment} draft={draft} />
      <aside className="inspector">
        <div className="inspector-tabs">
          <button className={tab === "design" ? "active" : ""} onClick={() => setTab("design")}>Design</button>
          <button className={tab === "comments" ? "active" : ""} onClick={() => setTab("comments")}>Comments
            {state.workspace.comments.some(item => !item.resolved) && <span>{state.workspace.comments.filter(item => !item.resolved).length}</span>}
          </button>
        </div>
        {tab === "design" ? <Inspector key={`${state.selection.artboard}:${state.selection.element}`} state={state} connection={connection} element={element} />
          : <Comments state={state} connection={connection} draft={draft} onCancel={() => setDraft(null)} active={activeComment} onActive={selectComment} />}
      </aside>
    </main> : <main className="empty-workspace">
      {state ? <code>/design</code>
        : <LoaderCircle className="spin" size={24} />}
    </main>}
  </div>;
}

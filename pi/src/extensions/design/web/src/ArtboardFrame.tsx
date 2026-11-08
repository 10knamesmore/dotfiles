import { useEffect, useMemo, useRef, useState } from "react";
import { LoaderCircle, RotateCw } from "lucide-react";
import type { Artboard, Mode, RuntimeMessage } from "@pi-design/react/protocol";
import { IconButton } from "./ui";

export function ArtboardFrame({ board, previewUrl, endpoint, mode, selected, onMessage }: {
  board: Artboard; previewUrl: string; endpoint: string; mode: Mode; selected: string | null;
  onMessage: (message: RuntimeMessage) => void;
}) {
  const iframe = useRef<HTMLIFrameElement>(null);
  const [ready, setReady] = useState(false);
  const [retry, setRetry] = useState(0);
  const [reachable, setReachable] = useState(false);
  const [waiting, setWaiting] = useState(false);
  const current = useRef({ mode, selected, onMessage });
  current.current = { mode, selected, onMessage };
  const src = useMemo(() => {
    const url = new URL(board.route || "/", `${previewUrl.replace(/\/$/, "")}/`);
    url.searchParams.set("__pi_design", endpoint);
    url.searchParams.set("__pi_artboard", board.id);
    return url.href;
  }, [board.route, board.id, endpoint, previewUrl]);
  const origin = new URL(src).origin;

  useEffect(() => {
    function receive(event: MessageEvent<RuntimeMessage>) {
      if (event.source !== iframe.current?.contentWindow || event.origin !== origin) return;
      const message = event.data;
      if (!message || message.source !== "pi-design" || message.artboard !== board.id) return;
      if (!["ready", "selection", "comment", "bounds"].includes(message.type)) return;
      if (message.type === "bounds" && (!Array.isArray(message.elements) || message.elements.some(item =>
        typeof item.id !== "string" || !item.rect || !Object.values(item.rect).every(value => typeof value === "number" && Number.isFinite(value)),
      ))) return;
      if (message.type === "ready") {
        setReady(true);
        iframe.current?.contentWindow?.postMessage({ source: "pi-design", type: "mode", mode: current.current.mode }, origin);
        iframe.current?.contentWindow?.postMessage({ source: "pi-design", type: "select", element: current.current.selected }, origin);
      }
      current.current.onMessage(message);
    }
    window.addEventListener("message", receive);
    return () => window.removeEventListener("message", receive);
  }, [origin, board.id]);

  useEffect(() => {
    if (!ready) return;
    iframe.current?.contentWindow?.postMessage({ source: "pi-design", type: "mode", mode }, origin);
    iframe.current?.contentWindow?.postMessage({ source: "pi-design", type: "select", element: selected }, origin);
  }, [mode, selected, ready, origin]);

  useEffect(() => {
    if (ready) return;
    let disposed = false;
    let previouslyReachable = false;
    let timer: ReturnType<typeof setTimeout>;
    const controller = new AbortController();
    const waitTimer = setTimeout(() => setWaiting(true), 8000);
    async function probe() {
      try {
        await fetch(src, { mode: "no-cors", cache: "no-store", signal: controller.signal });
        if (disposed) return;
        setReachable(true);
        if (!previouslyReachable) {
          previouslyReachable = true;
          setRetry(value => value + 1);
        }
      } catch {
        if (disposed) return;
        previouslyReachable = false;
        setReachable(false);
      }
      if (!disposed) timer = setTimeout(() => void probe(), 1600);
    }
    timer = setTimeout(() => void probe(), 1200);
    return () => { disposed = true; controller.abort(); clearTimeout(timer); clearTimeout(waitTimer); };
  }, [src, ready]);

  return <>
    <iframe key={`${src}:${retry}`} ref={iframe} title={board.name} src={src} className="preview-frame"
      style={{ width: board.width, height: board.height }} />
    {!ready && <div className={`preview-status ${waiting ? "waiting" : ""}`}>
      <LoaderCircle size={18} className="spin" />
      <span>{!reachable ? "Starting preview" : waiting ? "Waiting for DesignProvider" : "Connecting preview"}</span>
      {waiting && <IconButton icon={RotateCw} title="Reload preview" onClick={() => { setWaiting(false); setRetry(value => value + 1); }} />}
    </div>}
  </>;
}

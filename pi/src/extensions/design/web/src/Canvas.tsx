import { forwardRef, useCallback, useEffect, useImperativeHandle, useLayoutEffect, useRef, useState } from "react";
import { Frame, Hand, Maximize2, MessageCircle, Minus, MousePointer2, Play, Plus } from "lucide-react";
import type { DesignConnection } from "@pi-design/react/client";
import type {
  Artboard,
  CommentAnchor,
  DesignComment,
  ElementBounds,
  Mode,
  RuntimeMessage,
  State,
} from "@pi-design/react/protocol";
import { ArtboardFrame } from "./ArtboardFrame";
import { IconButton } from "./ui";

export interface CanvasHandle {
  fit: () => void;
  zoom: (factor: number) => void;
  focus: (artboard: string) => void;
}
interface Props {
  state: State;
  connection: DesignConnection;
  mode: Mode;
  onMode: (mode: Mode) => void;
  bounds: Record<string, ElementBounds[]>;
  onBounds: (artboard: string, elements: ElementBounds[]) => void;
  onComment: (anchor: CommentAnchor) => void;
  onCommentClick: (comment: DesignComment) => void;
  activeComment: string | null;
  draft: CommentAnchor | null;
}
interface View {
  x: number;
  y: number;
  scale: number;
}
const clampScale = (value: number) => Math.max(0.08, Math.min(3, value));

export const Canvas = forwardRef<CanvasHandle, Props>(function Canvas(
  { state, connection, mode, onMode, bounds, onBounds, onComment, onCommentClick, activeComment, draft },
  forwardedRef,
) {
  const viewport = useRef<HTMLDivElement>(null);
  const world = useRef<HTMLDivElement>(null);
  const zoomSlider = useRef<HTMLInputElement>(null);
  const [view, setView] = useState<View>({ x: 60, y: 70, scale: 0.65 });
  const [animateView, setAnimateView] = useState(false);
  const viewRef = useRef(view);
  viewRef.current = view;
  const [hand, setHand] = useState(false);
  const [space, setSpace] = useState(false);
  const [panning, setPanning] = useState(false);
  const [moving, setMoving] = useState<Record<string, Artboard>>({});
  const gestureCleanup = useRef<(() => void) | null>(null);
  const projectId = state.project?.id;
  const boards = state.workspace.artboards;
  const boardsRef = useRef(boards);
  boardsRef.current = boards;
  const activeBoard = state.selection.artboard;
  const fit = useCallback(() => {
    const element = viewport.current;
    const all = boardsRef.current;
    if (!element || !all.length) return;
    const left = Math.min(...all.map((board) => board.x));
    const top = Math.min(...all.map((board) => board.y));
    const right = Math.max(...all.map((board) => board.x + board.width));
    const bottom = Math.max(...all.map((board) => board.y + board.height));
    const scale = clampScale(
      Math.min(
        (element.clientWidth - 120) / Math.max(1, right - left),
        (element.clientHeight - 160) / Math.max(1, bottom - top),
        1,
      ),
    );
    setAnimateView(true);
    setView({
      x: (element.clientWidth - (right - left) * scale) / 2 - left * scale,
      y: (element.clientHeight - (bottom - top) * scale) / 2 - top * scale - 12,
      scale,
    });
  }, []);
  function zoomAt(factor: number, x?: number, y?: number) {
    const element = viewport.current;
    if (!element) return;
    const px = x ?? element.clientWidth / 2;
    const py = y ?? element.clientHeight / 2;
    setAnimateView(true);
    setView((previous) => {
      const scale = clampScale(previous.scale * factor);
      return {
        x: px - ((px - previous.x) * scale) / previous.scale,
        y: py - ((py - previous.y) * scale) / previous.scale,
        scale,
      };
    });
  }
  useImperativeHandle(forwardedRef, () => ({
    fit,
    zoom: zoomAt,
    focus(id) {
      const board = boardsRef.current.find((item) => item.id === id);
      const element = viewport.current;
      if (!board || !element) return;
      const scale = clampScale(
        Math.min((element.clientWidth - 100) / board.width, (element.clientHeight - 130) / board.height, 1),
      );
      setAnimateView(true);
      setView({
        x: (element.clientWidth - board.width * scale) / 2 - board.x * scale,
        y: (element.clientHeight - board.height * scale) / 2 - board.y * scale - 8,
        scale,
      });
    },
  }));
  useLayoutEffect(fit, [projectId, fit]);
  useEffect(() => () => gestureCleanup.current?.(), []);
  useEffect(() => {
    function key(event: KeyboardEvent) {
      const target = event.target;
      if (target instanceof HTMLElement && (target.matches("input,textarea,select") || target.isContentEditable))
        return;
      if (event.code === "Space") {
        event.preventDefault();
        setSpace(event.type === "keydown");
      }
      if (event.type !== "keydown" || event.ctrlKey || event.metaKey || event.altKey) return;
      if (event.key.toLowerCase() === "h") setHand(true);
      if (event.key.toLowerCase() === "v") {
        setHand(false);
        onMode("select");
      }
      if (event.key.toLowerCase() === "c") {
        setHand(false);
        onMode("comment");
      }
      if (event.key.toLowerCase() === "p") {
        setHand(false);
        onMode("play");
      }
      if (event.key === "1") fit();
    }
    const blur = () => setSpace(false);
    window.addEventListener("keydown", key);
    window.addEventListener("keyup", key);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("keyup", key);
      window.removeEventListener("blur", blur);
    };
  }, [fit, onMode]);
  useEffect(() => {
    const element = viewport.current!;
    function wheel(event: WheelEvent) {
      if (event.target instanceof Element && event.target.closest("button")) return;
      event.preventDefault();
      if (hand || space || event.ctrlKey || event.metaKey || event.altKey) {
        const rect = element.getBoundingClientRect();
        const delta = event.deltaY * (event.deltaMode === WheelEvent.DOM_DELTA_LINE ? 16 : 1);
        zoomAt(Math.exp(-delta * 0.002), event.clientX - rect.left, event.clientY - rect.top);
      } else {
        setAnimateView(true);
        setView((previous) => ({ ...previous, x: previous.x - event.deltaX, y: previous.y - event.deltaY }));
      }
    }
    element.addEventListener("wheel", wheel, { passive: false });
    return () => element.removeEventListener("wheel", wheel);
  }, [hand, space]);
  useEffect(() => {
    const slider = zoomSlider.current!;
    function wheel(event: WheelEvent) {
      event.preventDefault();
      event.stopPropagation();
      const delta = event.deltaY * (event.deltaMode === WheelEvent.DOM_DELTA_LINE ? 16 : 1);
      zoomAt(Math.exp(-delta * 0.002));
    }
    slider.addEventListener("wheel", wheel, { passive: false });
    return () => slider.removeEventListener("wheel", wheel);
  }, []);

  function track(event: React.PointerEvent, change: (x: number, y: number) => void, finish: () => void) {
    event.preventDefault();
    event.stopPropagation();
    gestureCleanup.current?.();
    const startX = event.clientX;
    const startY = event.clientY;
    const move = (pointer: PointerEvent) => change(pointer.clientX - startX, pointer.clientY - startY);
    const cleanup = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      document.body.classList.remove("canvas-gesture");
      gestureCleanup.current = null;
    };
    const end = () => {
      cleanup();
      finish();
      setPanning(false);
    };
    document.body.classList.add("canvas-gesture");
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
    gestureCleanup.current = cleanup;
  }
  function pan(event: React.PointerEvent) {
    if (event.button !== 0 && event.button !== 1) return;
    const transform = new DOMMatrixReadOnly(getComputedStyle(world.current!).transform);
    const previous = { x: transform.e, y: transform.f, scale: transform.a };
    setAnimateView(false);
    setView(previous);
    setPanning(true);
    track(
      event,
      (x, y) => setView({ ...previous, x: previous.x + x, y: previous.y + y }),
      () => {},
    );
  }
  function manipulate(event: React.PointerEvent, board: Artboard, resize = false) {
    if (event.button !== 0) return;
    let next = board;
    void connection.select({ artboard: board.id, element: null });
    track(
      event,
      (x, y) => {
        next = resize
          ? {
              ...board,
              width: Math.max(100, Math.round(board.width + x / viewRef.current.scale)),
              height: Math.max(100, Math.round(board.height + y / viewRef.current.scale)),
            }
          : {
              ...board,
              x: Math.round(board.x + x / viewRef.current.scale),
              y: Math.round(board.y + y / viewRef.current.scale),
            };
        setMoving((previous) => ({ ...previous, [board.id]: next }));
      },
      () => {
        if (next !== board) connection.edit([{ type: "artboard", artboard: next }]);
        setMoving((previous) => {
          const copy = { ...previous };
          delete copy[board.id];
          return copy;
        });
      },
    );
  }
  function runtimeMessage(message: RuntimeMessage) {
    if (message.type === "selection") {
      void connection.select({ artboard: message.artboard, element: message.element });
    } else if (message.type === "bounds") {
      onBounds(message.artboard, message.elements);
    } else if (message.type === "comment") {
      onComment({
        artboard: message.artboard,
        ...(message.element ? { element: message.element } : { x: message.x, y: message.y }),
      });
    }
  }
  function anchorPosition(anchor: CommentAnchor) {
    if (anchor.element) {
      const element = bounds[anchor.artboard]?.find((item) => item.id === anchor.element);
      if (!element) return null;
      return { x: element.rect.x + element.rect.width, y: element.rect.y };
    }
    return { x: anchor.x ?? 0, y: anchor.y ?? 0 };
  }
  return (
    <div
      className={`canvas-viewport ${animateView ? "animated-view" : ""} ${hand || space ? "hand-tool" : ""} ${panning ? "panning" : ""}`}
      ref={viewport}
      style={{
        backgroundPosition: `${view.x}px ${view.y}px`,
        backgroundSize: `${24 * view.scale}px ${24 * view.scale}px`,
      }}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) {
          if (event.button === 1 || hand || space) pan(event);
          else if (event.button === 0) void connection.select({ artboard: null, element: null });
        }
      }}
    >
      <div
        ref={world}
        className="canvas-world"
        style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})` }}
      >
        {boards.map((original) => {
          const board = moving[original.id] ?? original;
          return (
            <div
              key={board.id}
              className={`artboard ${activeBoard === board.id ? "selected" : ""}`}
              style={{ left: board.x, top: board.y, width: board.width, height: board.height }}
            >
              <div
                className="artboard-heading"
                style={{ width: board.width }}
                onPointerDown={(event) => manipulate(event, board)}
              >
                <Frame size={14} />
                <span>{board.name}</span>
                <span className="artboard-dimensions">
                  {board.width} × {board.height}
                </span>
              </div>
              <div className="artboard-content">
                {state.preview_url ? (
                  <ArtboardFrame
                    key={`${projectId}:${board.id}:${board.route}:${state.preview_url}`}
                    board={board}
                    previewUrl={state.preview_url}
                    endpoint={connection.endpoint}
                    mode={mode}
                    selected={activeBoard === board.id ? state.selection.element : null}
                    onMessage={runtimeMessage}
                  />
                ) : (
                  <div className="preview-empty">
                    <Frame size={24} />
                    <span>Preview unavailable</span>
                  </div>
                )}
              </div>
              {mode !== "play" &&
                state.workspace.comments
                  .filter((comment) => comment.anchor.artboard === board.id && !comment.resolved)
                  .map((comment, index) => {
                    const position = anchorPosition(comment.anchor);
                    return (
                      position && (
                        <button
                          key={comment.id}
                          className={`comment-pin ${activeComment === comment.id ? "active" : ""}`}
                          aria-label={comment.text}
                          title={comment.text}
                          style={{
                            left: position.x,
                            top: position.y,
                            transform: `translate(-50%, -50%) scale(${1 / view.scale})`,
                          }}
                          onClick={(event) => {
                            event.stopPropagation();
                            onCommentClick(comment);
                          }}
                        >
                          {index + 1}
                        </button>
                      )
                    );
                  })}
              {draft?.artboard === board.id &&
                (() => {
                  const position = anchorPosition(draft);
                  return (
                    position && (
                      <span
                        className="comment-pin draft-pin"
                        style={{
                          left: position.x,
                          top: position.y,
                          transform: `translate(-50%, -50%) scale(${1 / view.scale})`,
                        }}
                      >
                        <Plus size={14} />
                      </span>
                    )
                  );
                })()}
              {activeBoard === board.id && mode !== "play" && (
                <div className="artboard-resize" onPointerDown={(event) => manipulate(event, board, true)} />
              )}
            </div>
          );
        })}
      </div>
      {(hand || space || panning) && <div className="pan-shield" onPointerDown={pan} />}
      <div className="canvas-tools">
        <IconButton
          icon={MousePointer2}
          title="Select · V"
          active={mode === "select" && !hand}
          onClick={() => {
            setHand(false);
            onMode("select");
          }}
        />
        <IconButton icon={Hand} title="Pan · H / Space" active={hand} onClick={() => setHand(!hand)} />
        <IconButton
          icon={MessageCircle}
          title="Comment · C"
          active={mode === "comment" && !hand}
          onClick={() => {
            setHand(false);
            onMode("comment");
          }}
        />
        <span className="tool-separator" />
        <IconButton
          icon={Play}
          title="Play · P"
          active={mode === "play" && !hand}
          onClick={() => {
            setHand(false);
            onMode("play");
          }}
        />
      </div>
      <div className="canvas-zoom">
        <IconButton icon={Minus} title="Zoom out" onClick={() => zoomAt(1 / 1.2)} />
        <input
          ref={zoomSlider}
          className="zoom-slider"
          type="range"
          aria-label="Canvas zoom"
          min={8}
          max={300}
          step={1}
          value={Math.round(view.scale * 100)}
          onChange={(event) => zoomAt(Number(event.target.value) / 100 / viewRef.current.scale)}
        />
        <button
          className="zoom-value"
          title="Reset zoom"
          onClick={() => {
            const previous = viewRef.current;
            zoomAt(1 / previous.scale);
          }}
        >
          {Math.round(view.scale * 100)}%
        </button>
        <IconButton icon={Plus} title="Zoom in" onClick={() => zoomAt(1.2)} />
        <span className="tool-separator" />
        <IconButton icon={Maximize2} title="Fit artboards · 1" onClick={fit} />
      </div>
      {state.project && <div className="canvas-project-name">{state.design.name || state.project.name}</div>}
    </div>
  );
});

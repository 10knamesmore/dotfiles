import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useDesign } from "./context.js";
import type { ElementBounds, Rect, RuntimeMessage } from "./types.js";

function rectOf(element: HTMLElement): Rect {
  const rect = element.getBoundingClientRect();
  return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
}
function editableFrom(target: EventTarget | null) {
  return target instanceof Element ? target.closest<HTMLElement>("[data-design-id]") : null;
}
function allElements() {
  return [...document.querySelectorAll<HTMLElement>("[data-design-id]")];
}

export function RuntimeBridge() {
  const runtime = useDesign();
  const current = useRef(runtime);
  current.current = runtime;
  const [bounds, setBounds] = useState<ElementBounds[]>([]);
  const [hovered, setHovered] = useState<string | null>(null);
  const scheduleRef = useRef<() => void>(() => {});

  useEffect(() => {
    const environment = current.current.environment;
    if (!environment) return;
    const send = (message: Omit<RuntimeMessage, "source" | "artboard"> | Record<string, unknown>) => {
      window.parent.postMessage({ ...message, source: "pi-design", artboard: environment.artboard }, environment.origin);
    };
    let frame = 0;
    let previous = "";
    const observed = new Set<HTMLElement>();
    const observer = new ResizeObserver(() => schedule());
    function measure() {
      frame = 0;
      const elements = allElements();
      for (const node of observed) if (!node.isConnected) { observer.unobserve(node); observed.delete(node); }
      const values = elements.map(node => {
        if (!observed.has(node)) { observed.add(node); observer.observe(node); }
        return {
          id: node.dataset.designId!, tag: node.tagName.toLowerCase(), text: node.innerText ?? "", rect: rectOf(node),
          kind: node.hasAttribute("data-design-text") ? "text" as const : "box" as const,
        };
      });
      const serialized = JSON.stringify(values);
      if (serialized !== previous) {
        previous = serialized;
        setBounds(values);
        send({ type: "bounds", elements: values });
      }
    }
    function schedule() { if (!frame) frame = requestAnimationFrame(measure); }
    scheduleRef.current = schedule;
    const mutation = new MutationObserver(records => {
      if (records.some(record => !(record.target instanceof Element && record.target.closest("[data-design-overlay]")))) schedule();
    });
    mutation.observe(document.body, { subtree: true, childList: true, attributes: true, characterData: true });
    observer.observe(document.documentElement);

    function select(node: HTMLElement | null) {
      const id = node?.dataset.designId ?? null;
      current.current.setSelected(id);
      send({ type: "selection", element: id, rect: node ? rectOf(node) : null, text: node?.innerText ?? "", tag: node?.tagName.toLowerCase() ?? "" });
    }
    function click(event: MouseEvent) {
      const value = current.current;
      if (value.mode === "play" || value.editing) return;
      if (event.target instanceof Element && event.target.closest("[data-design-overlay]")) return;
      event.preventDefault();
      event.stopPropagation();
      const node = editableFrom(event.target);
      if (value.mode === "comment") {
        send({ type: "comment", ...(node ? { element: node.dataset.designId } : {}), x: event.clientX, y: event.clientY });
      } else select(node);
    }
    function pointerdown(event: PointerEvent) {
      if (current.current.mode === "play" || current.current.editing) return;
      if (event.target instanceof Element && event.target.closest("[data-design-overlay]")) return;
      event.preventDefault();
      event.stopPropagation();
    }
    function doubleClick(event: MouseEvent) {
      if (current.current.mode !== "select") return;
      const node = event.target instanceof Element ? event.target.closest<HTMLElement>("[data-design-text]") : null;
      if (!node) return;
      event.preventDefault();
      event.stopPropagation();
      select(node);
      current.current.setEditing(node.dataset.designId!);
    }
    function pointermove(event: PointerEvent) {
      if (current.current.mode === "play") return;
      setHovered(editableFrom(event.target)?.dataset.designId ?? null);
    }
    function keyboard(event: KeyboardEvent) {
      const target = event.target;
      if (target instanceof HTMLElement && (target.isContentEditable || target.matches("input,textarea,select"))) return;
      if (!(event.ctrlKey || event.metaKey)) return;
      if (event.key.toLowerCase() === "z" || event.key.toLowerCase() === "y") {
        event.preventDefault();
        event.stopPropagation();
        void current.current.connection?.history(event.shiftKey || event.key.toLowerCase() === "y" ? "redo" : "undo");
      }
    }
    function message(event: MessageEvent) {
      if (event.source !== window.parent || event.origin !== environment!.origin) return;
      const data: unknown = event.data;
      if (!data || typeof data !== "object" || !("source" in data) || data.source !== "pi-design" || !("type" in data)) return;
      if (data.type === "mode" && "mode" in data && (data.mode === "select" || data.mode === "comment" || data.mode === "play")) {
        current.current.setMode(data.mode);
        if (data.mode === "play") current.current.setEditing(null);
      }
      if (data.type === "select" && "element" in data && (typeof data.element === "string" || data.element === null)) current.current.setSelected(data.element);
    }
    document.addEventListener("click", click, true);
    document.addEventListener("pointerdown", pointerdown, true);
    document.addEventListener("dblclick", doubleClick, true);
    document.addEventListener("pointermove", pointermove, true);
    document.addEventListener("keydown", keyboard, true);
    window.addEventListener("message", message);
    window.addEventListener("resize", schedule);
    window.addEventListener("scroll", schedule, true);
    document.fonts?.ready.then(schedule);
    send({ type: "ready" });
    schedule();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      mutation.disconnect();
      document.removeEventListener("click", click, true);
      document.removeEventListener("pointerdown", pointerdown, true);
      document.removeEventListener("dblclick", doubleClick, true);
      document.removeEventListener("pointermove", pointermove, true);
      document.removeEventListener("keydown", keyboard, true);
      window.removeEventListener("message", message);
      window.removeEventListener("resize", schedule);
      window.removeEventListener("scroll", schedule, true);
      scheduleRef.current = () => {};
    };
  }, []);

  useEffect(() => { scheduleRef.current(); }, [runtime.data, runtime.selected, runtime.editing]);
  const selected = bounds.find(item => item.id === runtime.selected);
  const hover = bounds.find(item => item.id === hovered && item.id !== runtime.selected);
  if (!runtime.environment || runtime.mode === "play") return null;

  function resize(event: React.PointerEvent) {
    event.preventDefault();
    event.stopPropagation();
    const node = allElements().find(item => item.dataset.designId === runtime.selected);
    if (!node) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    const x = event.clientX;
    const y = event.clientY;
    const computed = getComputedStyle(node);
    const width = parseFloat(computed.width) || node.offsetWidth;
    const height = parseFloat(computed.height) || node.offsetHeight;
    let nextWidth = width;
    let nextHeight = height;
    const originalWidth = node.style.width;
    const originalHeight = node.style.height;
    function move(pointer: PointerEvent) {
      nextWidth = Math.max(1, Math.round(width + pointer.clientX - x));
      nextHeight = Math.max(1, Math.round(height + pointer.clientY - y));
      node!.style.width = `${nextWidth}px`;
      node!.style.height = `${nextHeight}px`;
      scheduleRef.current();
    }
    function finish(pointer: PointerEvent) {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", cancel);
      move(pointer);
      current.current.connection?.edit([{ type: "style", element: node!.dataset.designId!, style: { width: nextWidth, height: nextHeight } }]);
    }
    function cancel() {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", cancel);
      node!.style.width = originalWidth;
      node!.style.height = originalHeight;
      scheduleRef.current();
    }
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", cancel);
  }
  function outline(item: ElementBounds, active: boolean) {
    return <div key={active ? "selected" : "hover"} style={{
      position: "fixed", left: item.rect.x, top: item.rect.y, width: item.rect.width, height: item.rect.height,
      boxSizing: "border-box", border: `1.5px solid ${active ? "#6856e8" : "#9a8ff0"}`, pointerEvents: "none",
    }}>
      {active && !runtime.editing && <div onPointerDown={resize} style={{
        position: "absolute", right: -4, bottom: -4, width: 8, height: 8, background: "white",
        border: "1px solid #6856e8", pointerEvents: "auto", cursor: "nwse-resize",
      }} />}
    </div>;
  }
  return createPortal(<div data-design-overlay style={{ position: "fixed", inset: 0, zIndex: 2147483646, pointerEvents: "none" }}>
    {hover && outline(hover, false)}
    {selected && outline(selected, true)}
  </div>, document.body);
}

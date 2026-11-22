import { createElement, forwardRef, useLayoutEffect, useRef } from "react";
import type { ComponentPropsWithoutRef, ElementType, ForwardedRef, ReactNode } from "react";
import { useDesign } from "./context.js";

type EditableProps<T extends ElementType> = {
  id: string;
  as?: T;
  children?: ReactNode;
} & Omit<ComponentPropsWithoutRef<T>, "id" | "as" | "children">;
type EditableComponent = <T extends ElementType = "div">(
  props: EditableProps<T> & { ref?: ForwardedRef<HTMLElement> },
) => ReactNode;

function createEditable(kind: "text" | "box", defaultTag: string) {
  const Component = forwardRef<HTMLElement, EditableProps<ElementType>>(function EditableElement(
    { id, as, children, style, onBlur, onKeyDown, ...props },
    forwardedRef,
  ) {
    const runtime = useDesign();
    const node = useRef<HTMLElement | null>(null);
    const editing = runtime.editing === id && kind === "text";
    const saved = runtime.data.elements[id];
    const originalText = useRef("");
    const frozenChildren = useRef<ReactNode>(undefined);
    const previousEditing = useRef(false);
    if (editing && !previousEditing.current) {
      originalText.current = node.current?.innerText ?? saved?.text ?? "";
      frozenChildren.current = saved?.text ?? children;
    }
    previousEditing.current = editing;

    useLayoutEffect(() => {
      if (!editing || !node.current) return;
      node.current.focus();
      const selection = window.getSelection();
      const range = document.createRange();
      range.selectNodeContents(node.current);
      selection?.removeAllRanges();
      selection?.addRange(range);
    }, [editing]);

    return createElement(
      as ?? defaultTag,
      {
        ...props,
        id,
        ref: (element: HTMLElement | null) => {
          node.current = element;
          if (typeof forwardedRef === "function") forwardedRef(element);
          else if (forwardedRef) forwardedRef.current = element;
        },
        style: { ...style, ...saved?.style },
        "data-design-id": id,
        "data-design-text": kind === "text" ? "true" : undefined,
        contentEditable: editing ? "plaintext-only" : props.contentEditable,
        suppressContentEditableWarning: editing || props.suppressContentEditableWarning,
        onInput: (event: React.FormEvent<HTMLElement>) => {
          if (editing) runtime.connection?.edit([{ type: "text", element: id, text: event.currentTarget.innerText }]);
          props.onInput?.(event);
        },
        onBlur: (event: React.FocusEvent<HTMLElement>) => {
          if (editing) {
            const text = event.currentTarget.innerText;
            runtime.setEditing(null);
            runtime.connection?.edit([{ type: "text", element: id, text }]);
          }
          onBlur?.(event);
        },
        onKeyDown: (event: React.KeyboardEvent<HTMLElement>) => {
          if (editing && (event.key === "Escape" || (event.key === "Enter" && !event.shiftKey))) {
            event.preventDefault();
            event.stopPropagation();
            if (event.key === "Escape") {
              event.currentTarget.textContent = originalText.current;
              runtime.connection?.edit([{ type: "text", element: id, text: originalText.current }]);
              runtime.setEditing(null);
              event.currentTarget.blur();
            } else event.currentTarget.blur();
          }
          onKeyDown?.(event);
        },
      },
      kind === "box" ? children : editing ? frozenChildren.current : (saved?.text ?? children),
    );
  });
  Component.displayName = kind === "text" ? "Editable.Text" : "Editable.Box";
  return Component as EditableComponent;
}

export const Editable = {
  Text: createEditable("text", "span"),
  Box: createEditable("box", "div"),
};

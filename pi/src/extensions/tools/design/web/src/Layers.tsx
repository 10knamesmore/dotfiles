import { Box, ChevronRight, Frame, Plus, Type } from "lucide-react";
import type { DesignConnection } from "@pi-design/react/client";
import type { ElementBounds, State } from "@pi-design/react/protocol";
import { IconButton } from "./ui";
import { History } from "./History";

export function Layers({
  state,
  connection,
  bounds,
  onFocus,
  pending,
}: {
  state: State;
  connection: DesignConnection;
  bounds: Record<string, ElementBounds[]>;
  onFocus: (id: string) => void;
  pending: boolean;
}) {
  function add() {
    const boards = state.workspace.artboards;
    const last = boards[boards.length - 1];
    const artboard = {
      id: crypto.randomUUID(),
      name: `Artboard ${boards.length + 1}`,
      route: "/",
      x: last ? last.x + last.width + 80 : 80,
      y: last?.y ?? 80,
      width: last?.width ?? 1200,
      height: last?.height ?? 800,
    };
    connection.edit([{ type: "artboard", artboard }]);
    void connection.select({ artboard: artboard.id, element: null });
  }
  return (
    <aside className="layers-panel">
      <div className="sidebar-heading">
        <h2>Layers</h2>
        <IconButton icon={Plus} title="Add artboard" disabled={!state.project} onClick={add} />
      </div>
      <div className="layers-scroll">
        {state.workspace.artboards.map((board) => {
          const open = state.selection.artboard === board.id;
          const elements = bounds[board.id] ?? [];
          return (
            <div className="layer-artboard" key={board.id}>
              <button
                className={`artboard-row ${open && !state.selection.element ? "selected" : ""}`}
                aria-expanded={open}
                onClick={() => void connection.select({ artboard: board.id, element: null })}
                onDoubleClick={() => onFocus(board.id)}
              >
                <ChevronRight className="layer-chevron" size={13} />
                <Frame size={15} />
                <span>{board.name}</span>
                <span className="layer-count">{elements.length || ""}</span>
              </button>
              <div className={`layer-children ${open ? "expanded" : ""}`} inert={!open} aria-hidden={!open}>
                <div className="element-layers">
                  {elements.map((element) => {
                    const isText = element.kind === "text";
                    const Icon = isText ? Type : Box;
                    return (
                      <button
                        className={`element-row ${state.selection.element === element.id ? "selected" : ""}`}
                        key={element.id}
                        title={element.id}
                        onClick={() => void connection.select({ artboard: board.id, element: element.id })}
                      >
                        <Icon size={14} />
                        <span>{element.id}</span>
                        <code>{element.tag}</code>
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>
          );
        })}
      </div>
      <History history={state.history} connection={connection} pending={pending} />
      <div className="sidebar-footer">
        <span className="project-mark">P</span>
        <span>{state.project?.name ?? "Pi Design"}</span>
        <span className="revision">r{state.revision}</span>
      </div>
    </aside>
  );
}

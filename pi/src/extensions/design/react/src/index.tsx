import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { DesignConnection } from "./client.js";
import { DesignContext, useDesign } from "./context.js";
import { RuntimeBridge } from "./bridge.js";
import { editorEnvironment } from "./environment.js";
import type { DesignData, Mode, ParameterValue } from "./types.js";

export { Editable } from "./editable.js";
export type { DesignData, Parameter, ParameterValue } from "./types.js";

const standalone = { state: null, pending: 0, connected: false, error: null, failed: false } as const;
const noSubscribe = () => () => {};
const standaloneSnapshot = () => standalone;

export function DesignProvider({ data, children }: { data: DesignData; children: ReactNode }) {
  const [fileData, setFileData] = useState(data);
  useEffect(() => {
    setFileData(data);
  }, [data]);
  useEffect(() => {
    const update = (event: Event) => setFileData((event as CustomEvent<DesignData>).detail);
    window.addEventListener("pi-design:data", update);
    return () => window.removeEventListener("pi-design:data", update);
  }, []);
  const connection = useMemo(() => (editorEnvironment ? new DesignConnection(editorEnvironment.endpoint) : null), []);
  const snapshot = useSyncExternalStore(
    connection?.subscribe ?? noSubscribe,
    connection?.getSnapshot ?? standaloneSnapshot,
    standaloneSnapshot,
  );
  const [mode, setMode] = useState<Mode>("select");
  const [selected, setSelected] = useState<string | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  useEffect(() => connection?.start(), [connection]);
  const value = useMemo(
    () => ({
      data: snapshot.state?.design ?? fileData,
      connection,
      environment: editorEnvironment,
      mode,
      setMode,
      selected,
      setSelected,
      editing,
      setEditing,
    }),
    [snapshot.state?.design, fileData, connection, mode, selected, editing],
  );

  return (
    <DesignContext.Provider value={value}>
      {children}
      {editorEnvironment && <RuntimeBridge />}
      {snapshot.error && (
        <div
          data-design-overlay
          role="alert"
          style={{
            position: "fixed",
            bottom: 12,
            left: 12,
            right: 12,
            zIndex: 2147483647,
            padding: "10px 14px",
            background: "#fff1f0",
            border: "1px solid #e3aaa5",
            borderRadius: 8,
            color: "#942d26",
            font: "12px/1.5 system-ui",
            display: "flex",
            gap: 12,
            alignItems: "center",
          }}
        >
          <span style={{ flex: 1 }}>{snapshot.error}</span>
          <button onClick={() => connection?.retry()}>Retry</button>
          {snapshot.failed && <button onClick={() => connection?.discard()}>Discard edits</button>}
        </div>
      )}
    </DesignContext.Provider>
  );
}

export function useParameters<T extends Record<string, ParameterValue> = Record<string, ParameterValue>>(): T {
  const { data } = useDesign();
  return useMemo(
    () => Object.fromEntries(Object.entries(data.parameters).map(([name, parameter]) => [name, parameter.value])) as T,
    [data.parameters],
  );
}

import { Check, History as HistoryIcon } from "lucide-react";
import type { DesignConnection } from "@pi-design/react/client";
import type { State } from "@pi-design/react/protocol";

export function History({
  history,
  connection,
  pending,
}: {
  history: State["history"];
  connection: DesignConnection;
  pending: boolean;
}) {
  if (!history) return null;
  return (
    <section className="history-panel" aria-label="Design history">
      <div className="history-heading">
        <HistoryIcon size={14} />
        <h2>History</h2>
      </div>
      <div className="history-scroll">
        {[...history.entries].reverse().map((entry) => {
          const current = entry.id === history.current;
          return (
            <button
              key={entry.id}
              className={`history-row ${current ? "selected" : ""}`}
              aria-current={current ? "step" : undefined}
              title={entry.summary}
              disabled={pending}
              onClick={() => void connection.restoreSnapshot(entry.id)}
            >
              <span className="history-marker">{current ? <Check size={12} /> : <span />}</span>
              <span className="history-summary">{entry.summary}</span>
              <code>r{entry.id}</code>
            </button>
          );
        })}
      </div>
    </section>
  );
}

import { useState } from "react";
import { Check, MapPin, MessageCircle, RotateCcw, Trash2, X } from "lucide-react";
import type { DesignConnection } from "@pi-design/react/client";
import type { CommentAnchor, DesignComment, State } from "@pi-design/react/protocol";
import { IconButton, TextArea } from "./ui";

export function Comments({
  state,
  connection,
  draft,
  onCancel,
  active,
  onActive,
}: {
  state: State;
  connection: DesignConnection;
  draft: CommentAnchor | null;
  onCancel: () => void;
  active: string | null;
  onActive: (comment: DesignComment) => void;
}) {
  const [showResolved, setShowResolved] = useState(false);
  const [text, setText] = useState("");
  const comments = state.workspace.comments.filter((comment) => showResolved || !comment.resolved);
  const update = (comment: DesignComment) => connection.edit([{ type: "comment", comment }]);
  function submit() {
    if (!draft || !text.trim()) return;
    update({ id: crypto.randomUUID(), text: text.trim(), resolved: false, anchor: draft });
    setText("");
    onCancel();
  }
  const anchorName = (anchor: CommentAnchor) => {
    const board = state.workspace.artboards.find((item) => item.id === anchor.artboard)?.name ?? anchor.artboard;
    return `${board} / ${anchor.element ?? `${Math.round(anchor.x ?? 0)}, ${Math.round(anchor.y ?? 0)}`}`;
  };
  return (
    <div className="comments-panel">
      <div className="comments-heading">
        <span>{state.workspace.comments.filter((item) => !item.resolved).length} open</span>
        <button
          className={`text-button ${showResolved ? "selected" : ""}`}
          onClick={() => setShowResolved(!showResolved)}
        >
          {showResolved ? "Hide resolved" : "Show resolved"}
        </button>
      </div>
      {draft && (
        <form
          className="comment-card draft-comment"
          onSubmit={(event) => {
            event.preventDefault();
            submit();
          }}
        >
          <div className="comment-anchor">
            <MapPin size={13} />
            <span>{anchorName(draft)}</span>
            <IconButton icon={X} title="Cancel comment" onClick={onCancel} />
          </div>
          <textarea
            key={JSON.stringify(draft)}
            autoFocus
            aria-label="New comment"
            rows={4}
            placeholder="Add a comment…"
            value={text}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
                event.preventDefault();
                submit();
              }
              if (event.key === "Escape") {
                event.preventDefault();
                onCancel();
              }
            }}
          />
          <div className="comment-footer">
            <button type="submit" className="primary-button" disabled={!text.trim()}>
              Add comment
            </button>
          </div>
        </form>
      )}
      {comments.map((comment) => (
        <article
          key={comment.id}
          className={`comment-card ${comment.resolved ? "resolved" : ""} ${active === comment.id ? "active" : ""}`}
          onClick={() => onActive(comment)}
        >
          <div className="comment-anchor">
            <MessageCircle size={13} />
            <button onClick={() => onActive(comment)}>{anchorName(comment.anchor)}</button>
            <IconButton
              icon={comment.resolved ? RotateCcw : Check}
              title={comment.resolved ? "Reopen comment" : "Resolve comment"}
              onClick={(event) => {
                event.stopPropagation();
                update({ ...comment, resolved: !comment.resolved });
              }}
            />
            <IconButton
              icon={Trash2}
              title="Remove comment"
              onClick={(event) => {
                event.stopPropagation();
                connection.edit([{ type: "remove_comment", id: comment.id }]);
              }}
            />
          </div>
          <TextArea
            aria-label="Comment text"
            rows={Math.max(2, Math.min(8, Math.ceil(comment.text.length / 33)))}
            value={comment.text}
            onCommit={(text) => update({ ...comment, text })}
          />
          {comment.resolved && (
            <div className="resolved-stamp">
              <Check size={12} />
              Resolved
            </div>
          )}
        </article>
      ))}
      {!comments.length && !draft && (
        <div className="comments-empty">
          <MessageCircle size={28} strokeWidth={1.2} />
          <span>No comments</span>
        </div>
      )}
    </div>
  );
}

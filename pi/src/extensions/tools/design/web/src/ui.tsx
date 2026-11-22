import { useEffect, useState } from "react";
import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

export function IconButton({
  icon: Icon,
  title,
  active,
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { icon: LucideIcon; title: string; active?: boolean }) {
  return (
    <button
      type="button"
      aria-label={title}
      title={title}
      aria-pressed={active}
      className={`icon-button ${active ? "active" : ""} ${className}`}
      {...props}
    >
      <Icon size={16} strokeWidth={1.7} />
    </button>
  );
}

export function Section({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <section className="inspector-section">
      <div className="section-heading">
        <h3>{title}</h3>
        {action}
      </div>
      {children}
    </section>
  );
}

export function Field({
  value,
  onCommit,
  prefix,
  suffix,
  className = "",
  ...props
}: Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange" | "prefix"> & {
  value: string | number;
  onCommit: (value: string) => void;
  prefix?: ReactNode;
  suffix?: ReactNode;
}) {
  const [draft, setDraft] = useState(String(value));
  const [focused, setFocused] = useState(false);
  useEffect(() => {
    if (!focused) setDraft(String(value));
  }, [value, focused]);
  return (
    <label className={`field ${className}`}>
      {prefix !== undefined && (
        <span className="field-prefix" aria-hidden>
          {prefix}
        </span>
      )}
      <input
        {...props}
        value={draft}
        onFocus={() => setFocused(true)}
        onBlur={() => {
          setFocused(false);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") event.currentTarget.blur();
          if (event.key === "Escape") {
            setDraft(String(value));
            event.currentTarget.blur();
          }
        }}
        onChange={(event) => {
          setDraft(event.target.value);
          onCommit(event.target.value);
        }}
      />
      {suffix && (
        <span className="field-suffix" aria-hidden>
          {suffix}
        </span>
      )}
    </label>
  );
}

export function TextArea({
  value,
  onCommit,
  ...props
}: Omit<React.TextareaHTMLAttributes<HTMLTextAreaElement>, "value" | "onChange"> & {
  value: string;
  onCommit: (value: string) => void;
}) {
  const [draft, setDraft] = useState(value);
  const [focused, setFocused] = useState(false);
  useEffect(() => {
    if (!focused) setDraft(value);
  }, [value, focused]);
  return (
    <textarea
      {...props}
      value={draft}
      onFocus={() => setFocused(true)}
      onBlur={() => setFocused(false)}
      onChange={(event) => {
        setDraft(event.target.value);
        onCommit(event.target.value);
      }}
    />
  );
}

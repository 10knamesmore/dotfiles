import { useRef } from "react";
import type { DesignConnection } from "@pi-design/react/client";
import type { Parameter, ParameterValue } from "@pi-design/react/protocol";
import { Field, Section } from "./ui";

export function Parameters({
  parameters,
  connection,
}: {
  parameters: Record<string, Parameter>;
  connection: DesignConnection;
}) {
  const entries = Object.entries(parameters);
  if (!entries.length) return null;
  return (
    <Section title="Parameters">
      {entries.map(([name, parameter]) => {
        const update = (value: ParameterValue) => connection.edit([{ type: "parameter", name, value }]);
        const label = parameter.label ?? name;
        return (
          <div className={`parameter parameter-${parameter.control}`} key={name}>
            <div className="parameter-name">{label}</div>
            {parameter.control === "boolean" ? (
              <button
                role="switch"
                aria-checked={Boolean(parameter.value)}
                aria-label={label}
                className={`switch ${parameter.value ? "checked" : ""}`}
                onClick={() => update(!parameter.value)}
              >
                <span />
              </button>
            ) : parameter.control === "select" ? (
              <select
                aria-label={label}
                value={String(parameter.value)}
                onChange={(event) => update(event.target.value)}
              >
                {parameter.options?.map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </select>
            ) : parameter.control === "range" ? (
              <div className="range-control">
                <ParameterSlider name={name} parameter={parameter} connection={connection} />
                <Field
                  aria-label={`${label} value`}
                  type="number"
                  value={Number(parameter.value)}
                  min={parameter.min}
                  max={parameter.max}
                  step={parameter.step ?? 1}
                  onCommit={(value) => {
                    if (value !== "" && Number.isFinite(Number(value))) update(Number(value));
                  }}
                />
              </div>
            ) : parameter.control === "color" ? (
              <div className="color-field">
                <input
                  type="color"
                  aria-label={label}
                  value={String(parameter.value)}
                  onChange={(event) => update(event.target.value)}
                />
                <Field aria-label={`${label} color`} value={String(parameter.value)} onCommit={update} />
              </div>
            ) : (
              <Field
                aria-label={label}
                type={parameter.control === "number" ? "number" : "text"}
                value={String(parameter.value)}
                min={parameter.min}
                max={parameter.max}
                step={parameter.step}
                onCommit={(value) => {
                  if (parameter.control !== "number") update(value);
                  else if (value !== "" && Number.isFinite(Number(value))) update(Number(value));
                }}
              />
            )}
          </div>
        );
      })}
    </Section>
  );
}

function ParameterSlider({
  name,
  parameter,
  connection,
}: {
  name: string;
  parameter: Parameter;
  connection: DesignConnection;
}) {
  const dragging = useRef(false);
  const finish = () => {
    if (!dragging.current) return;
    dragging.current = false;
    connection.finishEdit();
  };
  return (
    <input
      aria-label={parameter.label ?? name}
      type="range"
      min={parameter.min}
      max={parameter.max}
      step={parameter.step ?? 1}
      value={Number(parameter.value)}
      onPointerDown={(event) => {
        if (event.button !== 0) return;
        dragging.current = true;
        event.currentTarget.setPointerCapture(event.pointerId);
      }}
      onPointerUp={finish}
      onPointerCancel={finish}
      onLostPointerCapture={finish}
      onBlur={finish}
      onChange={(event) =>
        connection.edit([{ type: "parameter", name, value: Number(event.target.value) }], {
          snapshot: !dragging.current,
        })
      }
    />
  );
}

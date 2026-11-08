import { ArrowDown, ArrowRight, Box, Circle, Hash, Layers2, Palette, Trash2, Type } from "lucide-react";
import type { DesignConnection } from "@pi-design/react/client";
import type { Artboard, ElementBounds, State } from "@pi-design/react/protocol";
import { Field, IconButton, Section, TextArea } from "./ui";
import { Parameters } from "./Parameters";

interface Props {
  state: State;
  connection: DesignConnection;
  element?: ElementBounds;
}
const numeric = (value: string): string | number | null => value.trim() === "" ? null : /^-?(?:\d+\.?\d*|\.\d+)$/.test(value.trim()) ? Number(value) : value;

export function Inspector({ state, connection, element }: Props) {
  const selected = state.selection.element;
  const artboard = state.workspace.artboards.find(item => item.id === state.selection.artboard);
  const saved = selected ? state.design.elements[selected] : undefined;
  const style = saved?.style ?? {};
  function changeStyle(property: string, value: string | number | null) {
    if (selected) connection.edit([{ type: "style", element: selected, style: { [property]: value } }]);
  }
  const styleField = (property: string, prefix: React.ReactNode, placeholder?: string) =>
    <Field key={property} aria-label={property} prefix={prefix} value={style[property] ?? ""} placeholder={placeholder}
      onCommit={value => changeStyle(property, numeric(value))} />;
  const color = (property: string, title: string, fallback: string) => <div className="color-row">
    <span className="property-name">{title}</span><div className="color-field">
      <input aria-label={title} type="color" value={/^#[0-9a-f]{6}$/i.test(String(style[property])) ? String(style[property]) : fallback}
        onChange={event => changeStyle(property, event.target.value)} />
      <Field aria-label={`${title} value`} value={style[property] ?? ""} placeholder={fallback} onCommit={value => changeStyle(property, value || null)} />
    </div>
  </div>;
  const selectStyle = (property: string, options: string[], placeholder: string) =>
    <select aria-label={property} value={style[property] ?? ""} onChange={event => changeStyle(property, event.target.value || null)}>
      <option value="">{placeholder}</option>{options.map(value => <option key={value}>{value}</option>)}
    </select>;
  return <div className="inspector-scroll">
    {selected ? <>
      <div className="selection-heading"><span className="selection-icon"><Box size={17} /></span>
        <div><strong>{selected}</strong><code>{element?.tag ?? "element"}</code></div>
        <span className="selection-badge">#{selected}</span>
      </div>
      <Section title="Layout">
        <div className="field-grid">
          {styleField("width", "W", element ? String(Math.round(element.rect.width)) : "auto")}
          {styleField("height", "H", element ? String(Math.round(element.rect.height)) : "auto")}
          {styleField("padding", <Box size={13} />, "padding")}
          {styleField("gap", "↔", "gap")}
        </div>
        <div className="field-grid select-row">
          {selectStyle("display", ["block", "flex", "grid", "inline", "inline-block", "none"], "Display")}
          {selectStyle("position", ["static", "relative", "absolute", "fixed", "sticky"], "Position")}
        </div>
        <div className="field-grid select-row">
          {selectStyle("flexDirection", ["row", "column", "row-reverse", "column-reverse"], "Direction")}
          {selectStyle("alignItems", ["start", "center", "end", "stretch", "baseline"], "Align")}
        </div>
        <div className="field-grid select-row">
          {selectStyle("justifyContent", ["start", "center", "end", "space-between", "space-around", "space-evenly"], "Distribute")}
          {styleField("borderRadius", <Circle size={13} />, "radius")}
        </div>
      </Section>
      <Section title="Typography">
        {element?.kind === "text" &&
        <TextArea key={selected} aria-label="Element text" className="element-text" rows={3} value={saved?.text ?? element?.text ?? ""}
          onCommit={text => connection.edit([{ type: "text", element: selected, text }])} />}
        <div className="field-grid">
          {styleField("fontSize", <Type size={13} />, "size")}
          {styleField("fontWeight", "B", "weight")}
          {styleField("lineHeight", <ArrowDown size={13} />, "line height")}
          {styleField("letterSpacing", <ArrowRight size={13} />, "spacing")}
        </div>
        <div className="select-row">{selectStyle("fontFamily", ["sans-serif", "serif", "monospace", "system-ui"], "Font family")}</div>
        <div className="select-row">{selectStyle("textAlign", ["left", "center", "right", "justify"], "Text alignment")}</div>
        {color("color", "Color", "#202027")}
      </Section>
      <Section title="Appearance">
        {color("backgroundColor", "Fill", "#ffffff")}
        {color("borderColor", "Border", "#dddddd")}
        <div className="field-grid">
          {styleField("borderWidth", <Hash size={13} />, "border")}
          {styleField("opacity", <Palette size={13} />, "opacity")}
        </div>
        <div className="select-row">{selectStyle("borderStyle", ["solid", "dashed", "dotted", "none"], "Border style")}</div>
        <Field aria-label="boxShadow" prefix={<Layers2 size={13} />} value={style.boxShadow ?? ""} placeholder="box-shadow"
          onCommit={value => changeStyle("boxShadow", value || null)} />
      </Section>
    </> : artboard ? <ArtboardInspector artboard={artboard} connection={connection} />
    : <Section title="Canvas"><div className="canvas-summary"><Box size={16} />
      <strong>{state.workspace.artboards.length}</strong><span>artboards</span></div></Section>}
    <Parameters parameters={state.design.parameters} connection={connection} />
  </div>;
}

function ArtboardInspector({ artboard, connection }: { artboard: Artboard; connection: DesignConnection }) {
  const change = (patch: Partial<Artboard>) => connection.edit([{ type: "artboard", artboard: { ...artboard, ...patch } }]);
  const numberField = (property: "x" | "y" | "width" | "height", prefix: string) => <Field
    aria-label={`Artboard ${property}`} prefix={prefix} type="number" value={artboard[property]}
    onCommit={value => {
      if (value !== "" && Number.isFinite(Number(value)) && (!["width", "height"].includes(property) || Number(value) > 0)) {
        change({ [property]: Number(value) });
      }
    }} />;
  return <>
    <div className="selection-heading"><span className="selection-icon"><Box size={17} /></span><strong>{artboard.name}</strong>
      <IconButton icon={Trash2} title="Remove artboard" onClick={() => connection.edit([{ type: "remove_artboard", id: artboard.id }])} /></div>
    <Section title="Artboard">
      <Field aria-label="Artboard name" value={artboard.name} onCommit={value => change({ name: value })} />
      <Field aria-label="Artboard route" prefix="/" value={artboard.route} onCommit={value => change({ route: value })} />
      <div className="field-grid">{numberField("x", "X")}{numberField("y", "Y")}{numberField("width", "W")}{numberField("height", "H")}</div>
    </Section>
  </>;
}

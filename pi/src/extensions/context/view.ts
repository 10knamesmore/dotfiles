import type { ExtensionCommandContext } from "@earendil-works/pi-coding-agent";
import { Input, fuzzyFilter, ScrollView, stripTerminalSequences } from "@earendil-works/pi-tui";
import { contextNavigationAction } from "./controls.js";
import { ContextAppearance, ContextReport, type PageContent } from "./presentation.js";
import { field, heading, note } from "./report.js";
import { SourceLayouts } from "./source-layout.js";
import { SourceContent } from "./source-view.js";
import {
  formatBytes,
  overviewReport,
  payloadReport,
  type ContextEntry,
  type ContextSnapshot,
  type PayloadSnapshot,
} from "./snapshot.js";

interface InspectionPage {
  label: string;
  group: string;
  content: PageContent;
}

type DetailView =
  | { kind: "report"; content: ContextReport; scroll: ScrollView }
  | { kind: "source"; content: SourceContent; scroll: ScrollView };

interface FilterEditor {
  input: Input;
  previousQuery: string;
  previousCursor: number;
  previousScroll: number;
}

function entryPages(entries: ContextEntry[], group: string): InspectionPage[] {
  return entries.map((entry) => ({
    label: `${entry.label} · ${formatBytes(entry.bytes)}`,
    group,
    content: { kind: "source", body: entry.body, format: entry.format },
  }));
}

const keyboardPage: InspectionPage = {
  label: "Keyboard reference",
  group: "Help",
  content: {
    kind: "report",
    rows: [
      heading("Normal mode · /agents navigation"),
      field("j / k · ↓ / ↑", "Move one item", "scroll one line in details"),
      field("J / K", "Move seven items", "scroll seven lines in details"),
      field("Enter / l / →", "Open the selected page"),
      field("h / Esc / ←", "Back", "close at the list"),
      field("g / G · Home / End", "First / last", "list item or detail line"),
      field("Ctrl+U / Ctrl+D", "Scroll one line", "keep list selection"),
      field("Ctrl+B / Ctrl+F", "Scroll half a page", "keep list selection"),
      field("PgUp / PgDn", "Move a full page", "list focus or detail viewport"),
      field("q / Ctrl+C", "Close the inspector"),
      field("?", "Open / dismiss this reference"),
      heading("List filtering"),
      field("/ · f", "Enter filter mode"),
      field("Enter", "Keep the filter and return to normal mode"),
      field("Esc / Ctrl+C", "Cancel editing", "restore the previous filter and selection"),
      note("Typing only edits the filter in filter mode. Navigation keys are ordinary text there."),
      note(
        "Returning from details preserves list selection, filter and viewport. Reopening a page restores its scroll position.",
      ),
    ],
  },
};

export async function inspectContext(
  ctx: ExtensionCommandContext,
  current: ContextSnapshot,
  payload: PayloadSnapshot | undefined,
): Promise<void> {
  const pages: InspectionPage[] = [
    {
      label: `Request #${current.number}`,
      group: "Overview",
      content: { kind: "report", rows: overviewReport(current, payload) },
    },
    ...(payload
      ? [
          {
            label: `Full request JSON · observation #${payload.number} · ${formatBytes(payload.bytes)}`,
            group: "Payload",
            content: { kind: "source", body: payload.json, format: "json" },
          } satisfies InspectionPage,
        ]
      : []),
    { label: "Context files and skill evidence", group: "Sources", content: { kind: "report", rows: current.sources } },
    ...entryPages([current.runtimePrompt], "Prompt"),
    {
      label: "Independent timeline",
      group: "Provider",
      content: { kind: "report", rows: payloadReport(payload) },
    },
    ...entryPages(payload?.entries ?? [], "Payload"),
    ...entryPages(current.prompt, "Prompt"),
    ...entryPages(current.tools, "Tool"),
    ...entryPages(
      [...current.messages].sort((a, b) => b.bytes - a.bytes),
      "Message",
    ),
    keyboardPage,
  ];

  await ctx.ui.custom<void>(
    (tui, theme, keybindings, done) => {
      const appearance = new ContextAppearance(theme);
      const sourceLayouts = new SourceLayouts(theme, (error) => {
        ctx.ui.notify(`Context layout failed: ${error.message}`, "error");
        done();
      });
      const items = pages.map((page, index) => ({
        index,
        page,
        searchText: `${page.group} ${stripTerminalSequences(page.label)}`,
      }));
      const details = new Map<number, DetailView>();
      const pageStack: number[] = [];
      const helpIndex = pages.length - 1;
      let filtered = items;
      let query = "";
      let cursor = 0;
      let filterEditor: FilterEditor | undefined;
      let focused = false;
      let followCursor = true;
      // Restore after layout: the cancelled filter may have shortened the scrollable content.
      let restoreListScroll: number | undefined;

      // Unlike SelectList, /agents lets the viewport move independently of the selected item.
      const listScroll = new ScrollView(
        {
          render(width): string[] {
            if (!filtered.length) return [theme.fg("muted", " No matching sections or messages.")];
            const start = listScroll.scrollTop;
            const visibleItems = filtered.slice(start, start + listScroll.viewportHeight);
            return visibleItems.map(({ page }, offset) => {
              const selected = cursor === start + offset;
              const marker = theme.fg(selected ? "accent" : "muted", selected ? "❯ " : "  ");
              const group = theme.fg(selected ? "accent" : "muted", `${page.group.padEnd(9)} `);
              const label = theme.style(stripTerminalSequences(page.label), { fg: "text", bold: selected });
              return appearance.fill(
                marker + group + label,
                width,
                selected ? appearance.selected : appearance.surface,
              );
            });
          },
          invalidate() {},
        },
        { scrollbar: "hidden" },
      );

      const detailView = (index: number): DetailView => {
        let detail = details.get(index);
        if (!detail) {
          const page = pages[index]!.content;
          if (page.kind === "source") {
            const content = new SourceContent(index, page, sourceLayouts, appearance, () => tui.requestRender());
            detail = { kind: "source", content, scroll: content.scroll };
          } else {
            const content = new ContextReport(page.rows, appearance);
            detail = { kind: "report", content, scroll: new ScrollView(content, { scrollbar: "hidden" }) };
          }
          details.set(index, detail);
        }
        return detail;
      };
      const activeScroll = () => {
        const index = pageStack.at(-1);
        return index === undefined ? listScroll : detailView(index).scroll;
      };
      const moveCursor = (next: number) => {
        cursor = Math.max(0, Math.min(next, filtered.length - 1));
        followCursor = true;
        restoreListScroll = undefined;
      };
      const applyFilter = (next: string) => {
        query = next;
        filtered = fuzzyFilter(items, query, (item) => item.searchText);
        moveCursor(0);
      };
      const finishFilter = (accept: boolean) => {
        const editor = filterEditor!;
        if (!accept) {
          applyFilter(editor.previousQuery);
          cursor = editor.previousCursor;
          restoreListScroll = editor.previousScroll;
          followCursor = false;
        }
        filterEditor = undefined;
      };
      const beginFilter = () => {
        const input = new Input({ prompt: "/ " });
        input.setValue(query);
        input.focused = focused;
        input.onSubmit = () => finishFilter(true);
        input.onEscape = () => finishFilter(false);
        filterEditor = { input, previousQuery: query, previousCursor: cursor, previousScroll: listScroll.scrollTop };
      };

      return {
        get focused() {
          return focused;
        },
        set focused(value: boolean) {
          focused = value;
          if (filterEditor) filterEditor.input.focused = value;
        },
        render(width) {
          const innerWidth = Math.max(1, width - 4);
          const index = pageStack.at(-1);
          const isList = index === undefined;
          const scroll = activeScroll();
          const detail = isList ? undefined : detailView(index);
          const maxRows = Math.max(1, Math.floor(tui.terminal.rows * 0.9) - 5);
          let visible: string[];
          let position: string;
          if (detail?.kind === "source") {
            visible = detail.content.renderViewport(innerWidth, maxRows);
            position = detail.content.position;
          } else {
            const detailLines = isList ? [] : scroll.render(innerWidth);
            const contentHeight = isList ? Math.max(1, filtered.length) : detailLines.length;
            const height = Math.min(Math.max(1, contentHeight), maxRows);
            scroll.updateLayout(contentHeight, height, () => tui.requestRender());
            if (isList) {
              if (restoreListScroll !== undefined) {
                listScroll.scrollTo(restoreListScroll);
                restoreListScroll = undefined;
              } else if (followCursor) {
                listScroll.scrollTo(cursor - Math.floor(height / 2));
                followCursor = false;
              }
            }
            visible = isList
              ? listScroll.render(innerWidth)
              : detailLines.slice(scroll.scrollTop, scroll.scrollTop + height);
            position = `${scroll.scrollTop + 1}–${Math.min(scroll.scrollTop + height, contentHeight)}/${contentHeight}`;
          }
          let status: string[];
          if (filterEditor) {
            status = filterEditor.input.render(innerWidth);
          } else if (isList) {
            status = [
              theme.fg(
                "muted",
                `NORMAL · selected ${filtered.length ? cursor + 1 : 0}/${filtered.length} · rows ${position} · / ${query || "all"}`,
              ),
            ];
          } else {
            status = [
              theme.fg(
                "muted",
                `NORMAL · rows ${position} · h back to ${pageStack.length > 1 ? "previous page" : "list"}`,
              ),
            ];
          }
          const title = isList ? "Context · sections and messages" : `${pages[index]!.group} · ${pages[index]!.label}`;
          const help = filterEditor
            ? "FILTER · Enter apply · Esc cancel"
            : isList
              ? "j/k · J/K ±7 · enter/l open · esc/h close · / filter · ? keys"
              : "j/k · J/K ±7 · esc/h back · g/G ends · ? keys";
          return appearance.panel(
            [...status.map((line) => appearance.fill(line, innerWidth, appearance.section)), ...visible],
            width,
            title,
            help,
          );
        },
        handleInput(data) {
          if (filterEditor) {
            if (keybindings.matches(data, "tui.select.cancel")) finishFilter(false);
            else {
              filterEditor.input.handleInput(data);
              if (filterEditor && filterEditor.input.getValue() !== query) applyFilter(filterEditor.input.getValue());
            }
            tui.requestRender();
            return;
          }

          const action = contextNavigationAction(data);
          const index = pageStack.at(-1);
          const isList = index === undefined;
          const scroll = activeScroll();
          switch (action.type) {
            case "move":
              if (isList) moveCursor(cursor + action.rows);
              else scroll.scrollBy(action.rows);
              break;
            case "page":
              if (isList) moveCursor(cursor + action.direction * scroll.viewportHeight);
              else scroll.scrollBy(action.direction * scroll.viewportHeight);
              break;
            case "scroll":
              if (isList) followCursor = false;
              scroll.scrollBy(action.lines);
              break;
            case "scrollPages":
              if (isList) followCursor = false;
              scroll.scrollBy(
                Math.sign(action.pages) * Math.max(1, Math.floor(scroll.viewportHeight * Math.abs(action.pages))),
              );
              break;
            case "first":
              if (isList) moveCursor(0);
              else scroll.scrollToStart();
              break;
            case "last":
              if (isList) moveCursor(filtered.length - 1);
              else scroll.scrollToEnd();
              break;
            case "open": {
              const selected = filtered[cursor];
              if (isList && selected) pageStack.push(selected.index);
              break;
            }
            case "back":
              if (isList) return done();
              pageStack.pop();
              break;
            case "close":
              return done();
            case "filter":
              if (isList) beginFilter();
              break;
            case "help":
              if (index === helpIndex) pageStack.pop();
              else pageStack.push(helpIndex);
              break;
            case "none":
              return;
          }
          tui.requestRender();
        },
        invalidate() {
          appearance.refresh();
          const themeChanged = sourceLayouts.updateTheme(theme);
          filterEditor?.input.invalidate();
          for (const detail of details.values()) {
            if (detail.kind === "source" && !themeChanged) detail.content.invalidateAppearance();
            else detail.content.invalidate();
          }
        },
        dispose() {
          sourceLayouts.dispose();
        },
      };
    },
  );
}

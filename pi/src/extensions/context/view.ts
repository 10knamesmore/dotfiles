import type { ExtensionCommandContext } from "@earendil-works/pi-coding-agent";
import { Input, fuzzyMatch, ScrollView, stripTerminalSequences } from "@earendil-works/pi-tui";
import { contextNavigationAction } from "./controls.js";
import { ContextAppearance, ContextReport, tableLayout, type PageContent } from "./presentation.js";
import { numberCell, type TableCell, type TableColumn } from "./report.js";
import { viewSourceInEditor } from "./source-editor.js";
import { SourceLayouts } from "./source-layout.js";
import { SourceContent } from "./source-view.js";
import { formatBytes, overviewReport, prepareContext, preparePayload, type ContextEntry } from "./inspection.js";
import type { ContextSnapshot, PayloadSnapshot } from "./snapshot.js";

interface InspectionPage {
  label: string;
  group: string;
  number?: number;
  detail?: string;
  bytes?: number;
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
    label: entry.label,
    group,
    bytes: entry.bytes,
    content: { kind: "source", body: entry.body, format: entry.format },
  }));
}

function pageTable(width: number, showDetail = width >= 60) {
  const textWidth = width - 39;
  const nameWidth = showDetail ? Math.max(17, Math.floor(textWidth * 0.45)) : Math.max(12, width - 36);
  const columns: TableColumn[] = [
    { weight: 10 },
    { weight: 5, align: "right" },
    { weight: nameWidth },
    { weight: 12, align: "right" },
  ];
  if (showDetail) columns.splice(3, 0, { weight: textWidth - nameWidth });
  return { layout: tableLayout(width, columns), showDetail };
}

function pageCells(page: InspectionPage, showDetail: boolean, highlighted: boolean): TableCell[] {
  return [
    { text: page.group, color: highlighted ? "accent" : "muted", bold: highlighted },
    page.number === undefined ? "" : numberCell(`#${page.number}`),
    { text: page.label, bold: highlighted },
    ...(showDetail ? [page.detail ?? ""] : []),
    page.bytes === undefined ? "" : numberCell(formatBytes(page.bytes)),
  ];
}

const keyboardPage: InspectionPage = {
  label: "Keyboard reference",
  group: "Help",
  content: {
    kind: "report",
    sections: [
      {
        title: "Navigation",
        columns: [
          { heading: "Keys", weight: 4 },
          { heading: "List", weight: 5 },
          { heading: "Details", weight: 5 },
        ],
        rows: [
          ["j/k or ↓/↑", "Select one item", "Scroll one line"],
          ["J/K", "Select seven items away", "Scroll seven lines"],
          ["Enter, l, or →", "Open selection", ""],
          ["Ctrl+G", "", "View in external editor"],
          ["h, Esc, or ←", "Close inspector", "Back"],
          ["g/G or Home/End", "First/last item", "Top/bottom"],
          ["Ctrl+U/D", "Scroll one line", "Scroll one line"],
          ["Ctrl+B/F", "Scroll half a page", "Scroll half a page"],
          ["PgUp/PgDn", "Select a page away", "Scroll one page"],
          ["q or Ctrl+C", "Close inspector", "Close inspector"],
          ["?", "Open help", "Open or dismiss help"],
        ],
        note: "Ctrl+U/D and Ctrl+B/F keep the selection. Reopening a page restores its scroll position.",
      },
      {
        title: "List filtering",
        columns: [
          { heading: "Keys", weight: 2 },
          { heading: "Action", weight: 5 },
        ],
        rows: [
          ["/ or f", "Edit filter"],
          ["Enter", "Apply filter"],
          ["Esc or Ctrl+C", "Restore previous filter, selection, and scroll position"],
        ],
        note: "Navigation keys enter text while you edit the filter.",
      },
    ],
  },
};

export async function inspectContext(
  ctx: ExtensionCommandContext,
  snapshot: ContextSnapshot,
  payloadSnapshot: PayloadSnapshot | undefined,
  externalEditor?: string,
): Promise<void> {
  const current = prepareContext(snapshot);
  const payload = payloadSnapshot && preparePayload(payloadSnapshot);
  const pages: InspectionPage[] = [
    {
      label: `Request #${current.number}`,
      group: "Overview",
      content: { kind: "report", sections: overviewReport(current) },
    },
    ...(payload
      ? [
          {
            label: "Full request JSON",
            group: "Payload",
            detail: `Observation #${payload.number} (${new Date(payload.capturedAt).toLocaleTimeString()})`,
            bytes: payload.bytes,
            content: { kind: "source", body: payload.json, format: "json" },
          } satisfies InspectionPage,
        ]
      : []),
    ...entryPages([current.runtimePrompt], "Prompt"),
    ...entryPages(payload?.entries ?? [], "Payload"),
    ...entryPages(current.prompt, "Prompt"),
    ...entryPages(current.tools, "Tool"),
    ...current.messages.map((message): InspectionPage => ({
      label: message.label,
      group: "Message",
      number: message.number,
      detail: message.path,
      bytes: message.bytes,
      content: { kind: "source", body: message.body, format: message.format },
    })),
    keyboardPage,
  ];

  await ctx.ui.custom<void>((tui, theme, keybindings, done) => {
    const appearance = new ContextAppearance(theme);
    const sourceLayouts = new SourceLayouts(theme, (error) => {
      ctx.ui.notify(`Context layout failed: ${error.message}`, "error");
      done();
    });
    const items = pages.map((page, index) => ({
      index,
      page,
      searchText: stripTerminalSequences(
        [
          page.group,
          page.number === undefined ? "" : `#${page.number}`,
          page.label,
          page.detail,
          page.bytes === undefined ? "" : formatBytes(page.bytes),
        ].join(" "),
      ),
    }));
    const details = new Map<number, DetailView>();
    const pageStack: number[] = [];
    const helpIndex = pages.length - 1;
    let filtered = items;
    let query = "";
    let cursor = 0;
    let filterEditor: FilterEditor | undefined;
    let openingEditor = false;
    let focused = false;
    let followCursor = true;
    // Restore after layout: the cancelled filter may have shortened the scrollable content.
    let restoreListScroll: number | undefined;
    let listLayout: (ReturnType<typeof pageTable> & { width: number }) | undefined;
    let listRows = new Map<string, string>();

    // Unlike SelectList, /agents lets the viewport move independently of the selected item.
    const listScroll = new ScrollView(
      {
        render(width): string[] {
          if (!filtered.length) return [theme.fg("muted", "No matching sections or messages.")];
          if (listLayout?.width !== width) {
            listLayout = { width, ...pageTable(width) };
            listRows.clear();
          }
          const { layout, showDetail } = listLayout;
          const start = listScroll.scrollTop;
          const visibleItems = filtered.slice(start, start + listScroll.viewportHeight);
          const nextRows = new Map<string, string>();
          const lines = visibleItems.map(({ page, index }, offset) => {
            const selected = cursor === start + offset;
            const style = selected ? "selected" : (start + offset) % 2 === 0 ? "stripe" : "surface";
            const key = `${index}:${style}`;
            const line =
              listRows.get(key) ??
              appearance.tableRow(layout, pageCells(page, showDetail, selected), width, appearance[style])[0]!;
            nextRows.set(key, line);
            return line;
          });
          listRows = nextRows;
          return lines;
        },
        invalidate() {
          listRows.clear();
        },
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
          const content = new ContextReport(page.sections, appearance);
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
      const tokens = query
        .trim()
        .split(/[\s/]+/)
        .filter(Boolean);
      filtered = items.filter((item) => tokens.every((token) => fuzzyMatch(token, item.searchText).matches));
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
        const page = isList ? undefined : pages[index]!;
        const title = page
          ? appearance.tableRow(
              pageTable(innerWidth, false).layout,
              pageCells(page, false, true),
              innerWidth,
              appearance.selected,
            )
          : [];
        const pageDetail = page?.detail
          ? appearance.tableRow(
              tableLayout(innerWidth, [{ weight: 1 }]),
              [page.detail],
              innerWidth,
              appearance.selected,
              true,
            )
          : [];
        const scroll = activeScroll();
        const detail = isList ? undefined : detailView(index);
        const filterLines =
          filterEditor?.input.render(innerWidth).map((line) => appearance.fill(line, innerWidth, appearance.section)) ??
          [];
        const maxRows = Math.max(
          1,
          Math.floor(tui.terminal.rows * 0.9) - 2 - title.length - pageDetail.length - filterLines.length,
        );
        let visible: readonly string[];
        if (detail?.kind === "source") {
          visible = detail.content.renderViewport(innerWidth, maxRows);
        } else {
          const detailLines = isList ? [] : scroll.render(innerWidth);
          const contentHeight = isList ? Math.max(1, filtered.length) : detailLines.length;
          const height = Math.min(Math.max(1, contentHeight), maxRows);
          // Input already requests a render, and any adjustment to scrollTop here is visible in this frame.
          scroll.updateLayout(contentHeight, height, () => {});
          if (isList) {
            if (restoreListScroll !== undefined) {
              listScroll.scrollTo(restoreListScroll);
              restoreListScroll = undefined;
            } else if (followCursor) {
              const scrolloff = Math.min(4, Math.floor((height - 1) / 2));
              const top = listScroll.scrollTop;
              if (cursor < top + scrolloff) {
                listScroll.scrollTo(cursor - scrolloff);
              } else if (cursor >= top + height - scrolloff) {
                listScroll.scrollTo(cursor - height + scrolloff + 1);
              }
              followCursor = false;
            }
          }
          visible = isList
            ? listScroll.render(innerWidth)
            : detailLines.slice(scroll.scrollTop, scroll.scrollTop + height);
        }
        const selection = isList ? `${filtered.length ? cursor + 1 : 0}/${filtered.length}` : undefined;
        return appearance.panel([...title, ...pageDetail, ...filterLines, ...visible], width, selection);
      },
      handleInput(data) {
        if (openingEditor) return;
        if (filterEditor) {
          if (keybindings.matches(data, "tui.select.cancel")) finishFilter(false);
          else {
            filterEditor.input.handleInput(data);
            if (filterEditor && filterEditor.input.getValue() !== query) applyFilter(filterEditor.input.getValue());
          }
          tui.requestRender();
          return;
        }

        const index = pageStack.at(-1);
        if (keybindings.matches(data, "app.editor.external")) {
          const page = index === undefined ? undefined : pages[index]?.content;
          if (page?.kind !== "source") return;
          openingEditor = true;
          void viewSourceInEditor(page, tui, externalEditor)
            .catch((error: unknown) => {
              ctx.ui.notify(
                `Could not open context in editor: ${error instanceof Error ? error.message : String(error)}`,
                "error",
              );
            })
            .finally(() => {
              openingEditor = false;
            });
          return;
        }
        const action = contextNavigationAction(data);
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
        listScroll.invalidate();
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
  });
}

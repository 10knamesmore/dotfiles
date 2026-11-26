import { DynamicBorder, type ExtensionAPI, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { Input, SelectList, truncateToWidth, type SelectItem } from "@earendil-works/pi-tui";
import { openBottomPanel } from "@dotfiles/pi-ui/bottom-panel";
import { PROMPT_EDITOR_CONFIGURE, type PromptEditorApi } from "@dotfiles/pi-ui/prompt-editor";
import { sanitizeFooterText } from "@dotfiles/pi-ui/status-format";
import type { DesignProject } from "./types.js";

const STATUS_KEY = "design";

export function registerDesignStatus(
  pi: ExtensionAPI,
  open: (ctx: ExtensionContext) => Promise<void>,
): (ctx: ExtensionContext, project: DesignProject | null) => void {
  let connected: { ctx: ExtensionContext; name: string } | undefined;
  let selected = false;

  const publish = () => {
    connected?.ctx.ui.setStatus(STATUS_KEY, `${selected ? "❯" : "●"} Design: ${connected.name} ↵`);
  };
  const clearSelection = () => {
    if (!selected) return;
    selected = false;
    publish();
  };

  pi.events.on(PROMPT_EDITOR_CONFIGURE, (requested) => {
    (requested as PromptEditorApi).useFooterNavigation(STATUS_KEY, {
      selectNext() {
        if (!connected) return false;
        selected = true;
        publish();
        return true;
      },
      selectPrevious: clearSelection,
      hasSelection: () => selected,
      openSelection() {
        if (!selected || !connected) return false;
        clearSelection();
        void open(connected.ctx);
        return true;
      },
      clearSelection,
    });
  });

  return (ctx, project) => {
    if (!project) {
      if (!connected) return;
      connected = undefined;
      selected = false;
      ctx.ui.setStatus(STATUS_KEY, undefined);
      return;
    }
    const name = truncateToWidth(sanitizeFooterText(project.name), 32, "…");
    if (connected?.name === name) return;
    connected = { ctx, name };
    publish();
  };
}

export function selectDesign(
  ctx: ExtensionContext,
  projects: DesignProject[],
  connected: DesignProject | null,
): Promise<string | undefined> {
  return openBottomPanel<string | undefined>(
    ctx.ui,
    (tui, theme, keys, done) => {
      const border = new DynamicBorder((line) => theme.fg("accent", line));
      const search = new Input({ prompt: "  ", placeholder: "Search projects" });
      const projectItems: SelectItem[] = projects
        .filter((project) => project.id !== connected?.id)
        .map((project) => ({
          value: project.id,
          label: sanitizeFooterText(project.name),
          description: sanitizeFooterText(project.path),
        }));
      const actions: SelectItem[] = [
        ...(connected
          ? [
              { value: "workspace", label: "Open workspace" },
              { value: "disconnect", label: "Disconnect" },
            ]
          : []),
        { value: "new", label: "＋ New project" },
      ];
      const listTheme = {
        selectedPrefix: (text: string) => theme.fg("accent", text),
        selectedText: (text: string) => theme.fg("accent", text),
        description: (text: string) => theme.fg("muted", text),
        scrollInfo: (text: string) => theme.fg("dim", text),
        noMatch: () => theme.fg("dim", "  No matching projects"),
      };
      const createList = (rows: number, selected?: string) => {
        const query = search.getValue().trim().toLowerCase();
        const items = query
          ? projectItems.filter((item) => `${item.label} ${item.description}`.toLowerCase().includes(query))
          : [...actions, ...projectItems];
        const list = new SelectList(items, rows, listTheme);
        list.onSelect = (item) => done(item.value);
        list.onCancel = () => done(undefined);
        const index = items.findIndex((item) => item.value === selected);
        if (index >= 0) list.setSelectedIndex(index);
        return list;
      };
      let rows = 1;
      let list = createList(rows);
      let listTop = 0;
      let listHeight = 0;
      const status = connected
        ? `Connected: ${sanitizeFooterText(connected.name)} · ${sanitizeFooterText(connected.path)}`
        : "No project connected";

      return {
        get focused() {
          return search.focused;
        },
        set focused(value: boolean) {
          search.focused = value;
        },
        render(width, height) {
          const top = [
            ...border.render(width),
            truncateToWidth(`  ${theme.bold("Design")}  ${theme.fg("muted", status)}`, width),
            ...search.render(width),
          ];
          const bottom = border.render(width);
          const visible = Math.max(1, Math.min(10, height - top.length - bottom.length - 1));
          if (visible !== rows) {
            rows = visible;
            list = createList(rows, list.getSelectedItem()?.value);
          }
          const items = list.render(width);
          listTop = top.length;
          listHeight = items.length;
          return [...top, ...items, ...bottom];
        },
        invalidate() {
          border.invalidate();
          search.invalidate();
          list.invalidate();
        },
        handleInput(data) {
          if (
            (
              [
                "tui.select.up",
                "tui.select.down",
                "tui.select.pageUp",
                "tui.select.pageDown",
                "tui.select.confirm",
                "tui.select.cancel",
              ] as const
            ).some((key) => keys.matches(data, key))
          ) {
            list.handleInput(data);
          } else {
            const previous = search.getValue();
            search.handleInput(data);
            if (previous !== search.getValue()) list = createList(rows);
          }
          tui.requestRender();
        },
        handleMouse(event) {
          const y = event.y - listTop;
          if (y < 0 || y >= listHeight) return;
          return list.handleMouse({ ...event, y, height: listHeight });
        },
      };
    },
    { heightPercent: 50 },
  );
}

import { DynamicBorder, type ExtensionContext } from "@earendil-works/pi-coding-agent";
import { Container, Input, SelectList, Spacer, Text, type SelectItem } from "@earendil-works/pi-tui";
import type { DesignProject } from "./types.js";

export async function selectDesign(
  ctx: ExtensionContext,
  projects: DesignProject[],
): Promise<string | undefined> {
  return ctx.ui.custom<string | undefined>((tui, theme, keys, done) => {
    const container = new Container();
    container.addChild(new DynamicBorder((line) => theme.fg("accent", line)));
    container.addChild(new Text(theme.bold("Design"), 1, 0));
    const search = new Input({ prompt: "  ", placeholder: "Search" });
    container.addChild(search);
    container.addChild(new Spacer(1));
    const items: SelectItem[] = [
      { value: "new", label: "＋ New" },
      ...projects.map((project) => ({
        value: project.id,
        label: project.name,
        description: project.path,
      })),
    ];
    const listTheme = {
      selectedPrefix: (text) => theme.fg("accent", text),
      selectedText: (text) => theme.fg("accent", text),
      description: (text) => theme.fg("muted", text),
      scrollInfo: (text) => theme.fg("dim", text),
      noMatch: () => theme.fg("dim", "No matching projects"),
    } satisfies ConstructorParameters<typeof SelectList>[2];
    const createList = (query: string) => {
      const matching = items.filter((item) =>
        item.value === "new" || `${item.label} ${item.description ?? ""}`.toLowerCase().includes(query.toLowerCase()),
      );
      const list = new SelectList(matching, 10, listTheme);
      list.onSelect = (item) => done(item.value);
      list.onCancel = () => done(undefined);
      if (query && matching.length > 1) list.setSelectedIndex(1);
      return list;
    };
    let list = createList("");
    container.addChild({
      render: (width) => list.render(width),
      invalidate: () => list.invalidate(),
    });
    container.addChild(new DynamicBorder((line) => theme.fg("accent", line)));

    return {
      get focused() { return search.focused; },
      set focused(value: boolean) { search.focused = value; },
      capturesViewportInput: true,
      render: (width: number) => container.render(width),
      invalidate: () => container.invalidate(),
      handleInput(data: string): void {
        if (([
          "tui.select.up", "tui.select.down", "tui.select.pageUp", "tui.select.pageDown",
          "tui.select.confirm", "tui.select.cancel",
        ] as const).some((key) => keys.matches(data, key))) {
          list.handleInput(data);
        } else {
          search.handleInput(data);
          list = createList(search.getValue());
        }
        tui.requestRender();
      },
    };
  }, { blocked: { kind: "question", message: "Choose a design" } });
}

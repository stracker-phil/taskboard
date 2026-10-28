import { describe, expect, it, vi } from "vitest";
import type { GroupBy, MenuEntry, Sort } from "../../api";
import { groupMenu } from "./groupMenu";

type Entry = Exclude<MenuEntry, "separator">;
const entriesOf = (entries: MenuEntry[]) => entries.filter((e): e is Entry => e !== "separator");

function build(groupBy: GroupBy = "status", sort: Sort = { by: "updated", descending: true }) {
  const setGroup = vi.fn();
  const setSort = vi.fn();
  const menu = entriesOf(groupMenu(groupBy, sort, setGroup, setSort));
  const sortMenu = entriesOf(menu.find((e) => e.text === "Sort by")!.items!);
  return { menu, sortMenu, setGroup, setSort };
}

describe("groupMenu", () => {
  it("lists the groupings and checks only the current one", () => {
    const { menu } = build("epic");

    const groupings = menu.filter((e) => e.text !== "Sort by");
    expect(groupings.map((e) => e.text)).toEqual(["Status", "Space", "Client", "Assignee", "Epic", "Release", "Priority", "Due date", "None"]);
    expect(groupings.filter((e) => e.checked).map((e) => e.text)).toEqual(["Epic"]);
  });

  it("groups by the chosen entry", () => {
    const { menu, setGroup } = build();

    menu.find((e) => e.text === "Priority")!.run!();

    expect(setGroup).toHaveBeenCalledWith("priority");
  });

  it("checks the current sort key and direction in the Sort by submenu", () => {
    const { sortMenu } = build("status", { by: "due", descending: false });

    expect(sortMenu.filter((e) => e.checked).map((e) => e.text)).toEqual(["Due date", "Ascending"]);
  });

  it("sorts by a chosen key and keeps the direction", () => {
    const { sortMenu, setSort } = build("status", { by: "updated", descending: true });

    sortMenu.find((e) => e.text === "Key")!.run!();

    expect(setSort).toHaveBeenCalledWith({ by: "key", descending: true });
  });

  it("flips the direction and keeps the sort key", () => {
    const { sortMenu, setSort } = build("status", { by: "summary", descending: true });

    sortMenu.find((e) => e.text === "Ascending")!.run!();

    expect(setSort).toHaveBeenCalledWith({ by: "summary", descending: false });
  });
});

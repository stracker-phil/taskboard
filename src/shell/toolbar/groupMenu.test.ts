import { describe, expect, it, vi } from "vitest";
import type { GroupBy, Sort } from "../../api";
import type { MenuEntry, MenuItem } from "../../components/menu";
import { groupMenu } from "./groupMenu";

const entriesOf = (entries: MenuEntry[]) => entries.filter((e): e is MenuItem => e !== "separator" && !("caption" in e));

function build(groupBy: GroupBy = "status", sort: Sort = { by: "updated", descending: true }, acrossSpaces = true) {
  const setGroup = vi.fn();
  const setSort = vi.fn();
  const menu = entriesOf(groupMenu(groupBy, sort, acrossSpaces, setGroup, setSort));
  const sortMenu = entriesOf(menu.find((e) => e.text === "Sort by")!.items!);
  return { menu, sortMenu, setGroup, setSort };
}

const groupingsOf = (menu: MenuItem[]) => menu.filter((e) => e.text !== "Sort by");

describe("groupMenu", () => {
  it("lists the groupings and checks only the current one", () => {
    const { menu } = build("epic");

    const groupings = groupingsOf(menu);
    expect(groupings.map((e) => e.text)).toEqual(["Status", "Status type", "Space", "Client", "Assignee", "Epic", "Release", "Priority", "Due date", "None"]);
    expect(groupings.filter((e) => e.checked).map((e) => e.text)).toEqual(["Epic"]);
  });

  describe("Space and Client groupings", () => {
    it("are left out when the issues come from a single Space", () => {
      const { menu } = build("status", undefined, false);

      const names = groupingsOf(menu).map((e) => e.text);
      expect(names).not.toContain("Space");
      expect(names).not.toContain("Client");
      expect(names).toContain("Assignee");
    });

    it("are both offered across Spaces", () => {
      const { menu } = build("status", undefined, true);

      expect(groupingsOf(menu).map((e) => e.text)).toEqual(expect.arrayContaining(["Space", "Client"]));
    });

    it("keep the one the view already groups by, checked, but not the other", () => {
      const { menu } = build("project", undefined, false);

      const groupings = groupingsOf(menu);
      expect(groupings.find((e) => e.text === "Space")?.checked).toBe(true);
      expect(groupings.map((e) => e.text)).not.toContain("Client");
    });

    it("keep Client, checked, when the view groups by it outside Everything", () => {
      const { menu } = build("client", undefined, false);

      const groupings = groupingsOf(menu);
      expect(groupings.find((e) => e.text === "Client")?.checked).toBe(true);
      expect(groupings.map((e) => e.text)).not.toContain("Space");
    });
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

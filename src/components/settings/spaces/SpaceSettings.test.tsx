import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type Accent, type SpaceConfig, type SpaceLayouts } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { settings } from "../../../test/settings";
import { fieldSetting, space, spaceConfig } from "../../../test/spaces";
import { teamCell, teamMatrix } from "../../../test/team";
import { MenuHost } from "../../menu";
import { Toasts } from "../../Toasts";
import { ScrollTarget } from "../controls";
import { SpaceSettings } from "./SpaceSettings";

const mocked = vi.mocked(api, true);

const layouts = (patch: Partial<SpaceLayouts> = {}): SpaceLayouts => ({ layouts: ["list", "board", "table"], defaultLayout: "list", defaultGroup: "status", ...patch });

const tabButton = (name: string) => within(screen.getByRole("group", { name: "Space settings" })).getByRole("button", { name });

async function renderSettings(config: SpaceConfig = spaceConfig(), tab?: string) {
  const onBack = vi.fn();
  mocked.spaceConfig.mockResolvedValue(config);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SpaceSettings space={space()} onBack={onBack} />
      <Toasts />
      <MenuHost />
    </QueryClientProvider>,
  );
  await screen.findByRole("region", { name: "Layouts" });
  if (tab) await userEvent.click(tabButton(tab));
  return onBack;
}

const client = createQueryClient();
const atTarget = (target: string | undefined) => (
  <QueryClientProvider client={client}>
    <ScrollTarget.Provider value={target ? { id: target } : undefined}>
      <SpaceSettings space={space()} />
    </ScrollTarget.Provider>
  </QueryClientProvider>
);
function renderAtTarget(target: string | undefined) {
  mocked.spaceConfig.mockResolvedValue(spaceConfig());
  client.clear();
  return render(atTarget(target));
}

const GROUPS = ["General", "Layouts", "Team", "Statuses", "Custom fields"];
const expectOnlyGroups = (shown: string[]) => {
  for (const name of GROUPS) {
    if (shown.includes(name)) expect(screen.getByRole("region", { name })).toBeInTheDocument();
    else expect(screen.queryByRole("region", { name })).not.toBeInTheDocument();
  }
};

const openMenuOf = async (buttonName: string) => {
  await userEvent.click(screen.getByRole("button", { name: buttonName }));
  return screen.getByRole("menu", { name: buttonName });
};
const rowsOf = (menu: HTMLElement) => within(menu).getAllByRole("menuitemradio");
const choose = (menu: HTMLElement, name: string) => userEvent.click(within(menu).getByRole("menuitemradio", { name }));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("SpaceSettings", () => {
  describe("tabs", () => {
    it("opens on General showing the General and Layouts groups only", async () => {
      await renderSettings();

      expect(screen.getByRole("heading", { name: "Website relaunch" })).toBeInTheDocument();
      expect(tabButton("General")).toHaveAttribute("aria-pressed", "true");
      expect(mocked.spaceConfig).toHaveBeenCalledWith("10001");
      expectOnlyGroups(["General", "Layouts"]);
    });

    it.each([
      ["Team", "Team"],
      ["Statuses", "Statuses"],
      ["Custom Fields", "Custom fields"],
    ])("shows only its group when the %s tab is chosen", async (tab, group) => {
      await renderSettings();

      await userEvent.click(tabButton(tab));

      expect(await screen.findByRole("region", { name: group })).toBeInTheDocument();
      expect(tabButton(tab)).toHaveAttribute("aria-pressed", "true");
      expect(tabButton("General")).toHaveAttribute("aria-pressed", "false");
      expectOnlyGroups([group]);
    });

    it("loads the Space's statuses when the Statuses tab is chosen", async () => {
      await renderSettings(spaceConfig(), "Statuses");

      expect(await screen.findByRole("region", { name: "Statuses" })).toBeInTheDocument();
      expect(mocked.statusStylesList).toHaveBeenCalledWith("10001");
    });

    it("returns to the General groups when the General tab is chosen again", async () => {
      await renderSettings(spaceConfig(), "Team");

      await userEvent.click(tabButton("General"));

      expectOnlyGroups(["General", "Layouts"]);
    });

    it("keeps the tab when the pressed tab is clicked again", async () => {
      await renderSettings(spaceConfig(), "Team");

      await userEvent.click(tabButton("Team"));

      expect(tabButton("Team")).toHaveAttribute("aria-pressed", "true");
      expect(screen.getByRole("region", { name: "Team" })).toBeInTheDocument();
    });

    describe("requested section", () => {
      it.each([
        ["team", "Team", "Team"],
        ["statuses", "Statuses", "Statuses"],
        ["fields", "Custom Fields", "Custom fields"],
      ])("opens the %s section on its tab", async (target, tab, group) => {
        renderAtTarget(target);

        expect(await screen.findByRole("region", { name: group })).toBeInTheDocument();
        expect(tabButton(tab)).toHaveAttribute("aria-pressed", "true");
        expectOnlyGroups([group]);
      });

      it.each(["layouts", "unknown"])("opens General for the %s section", async (target) => {
        renderAtTarget(target);

        expect(await screen.findByRole("region", { name: "Layouts" })).toBeInTheDocument();
        expect(tabButton("General")).toHaveAttribute("aria-pressed", "true");
        expectOnlyGroups(["General", "Layouts"]);
      });

      it("follows a new target", async () => {
        const { rerender } = renderAtTarget("team");
        await screen.findByRole("region", { name: "Team" });

        rerender(atTarget("fields"));

        expect(await screen.findByRole("region", { name: "Custom fields" })).toBeInTheDocument();
        expect(tabButton("Custom Fields")).toHaveAttribute("aria-pressed", "true");
      });
    });
  });

  it("opens the look popover from the header tile and saves the chosen color", async () => {
    await renderSettings();

    await userEvent.click(screen.getByRole("button", { name: "Icon and color of Website relaunch" }));
    const dialog = screen.getByRole("dialog", { name: "Icon and color" });
    await userEvent.click(within(dialog).getByRole("radio", { name: "Teal" }));

    expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", expect.objectContaining({ color: "4" }));
  });

  it("goes back when the Spaces button is clicked", async () => {
    const onBack = await renderSettings();

    await userEvent.click(screen.getByRole("button", { name: "Spaces" }));

    expect(onBack).toHaveBeenCalled();
  });

  it("has no Spaces button without onBack", async () => {
    mocked.spaceConfig.mockResolvedValue(spaceConfig());
    render(
      <QueryClientProvider client={createQueryClient()}>
        <SpaceSettings space={space()} />
      </QueryClientProvider>,
    );
    await screen.findByRole("region", { name: "Layouts" });

    expect(screen.queryByRole("button", { name: "Spaces" })).not.toBeInTheDocument();
  });

  describe("accent color", () => {
    async function renderWith(spaceAccent: Accent | null, appAccent: Accent = "blue") {
      mocked.getSettings.mockResolvedValue(settings({ accent: appAccent }));
      mocked.spaceConfig.mockResolvedValue(spaceConfig());
      render(
        <QueryClientProvider client={createQueryClient()}>
          <SpaceSettings space={space({ accent: spaceAccent })} />
        </QueryClientProvider>,
      );
      await screen.findByRole("region", { name: "Layouts" });
      return screen.getByRole("radiogroup", { name: "Accent color" });
    }

    it("checks the Space's own accent", async () => {
      const group = await renderWith("purple");

      expect(within(group).getByRole("radio", { name: "Purple" })).toHaveAttribute("aria-checked", "true");
      expect(within(group).getAllByRole("radio", { checked: true })).toHaveLength(1);
    });

    it("checks the swatch that follows the app when the Space has none", async () => {
      const group = await renderWith(null, "orange");

      expect(await within(group).findByRole("radio", { name: "Same as the app (Orange)", checked: true })).toBeInTheDocument();
    });

    it("sends the clicked accent for the Space to Rust", async () => {
      const group = await renderWith(null);

      await userEvent.click(within(group).getByRole("radio", { name: "Teal" }));

      expect(mocked.spaceSetAccent).toHaveBeenCalledWith("10001", "teal");
    });

    it("follows System while the settings have not loaded", async () => {
      mocked.getSettings.mockReturnValueOnce(new Promise(() => {}));
      mocked.spaceConfig.mockResolvedValue(spaceConfig());
      render(
        <QueryClientProvider client={createQueryClient()}>
          <SpaceSettings space={space({ accent: null })} />
        </QueryClientProvider>,
      );
      await screen.findByRole("region", { name: "Layouts" });
      const group = screen.getByRole("radiogroup", { name: "Accent color" });

      expect(within(group).getByRole("radio", { name: "Same as the app (System)", checked: true })).toBeInTheDocument();
    });

    it("sends null when the swatch that follows the app is clicked", async () => {
      const group = await renderWith("teal");

      await userEvent.click(within(group).getByRole("radio", { name: "Same as the app (Blue)" }));

      expect(mocked.spaceSetAccent).toHaveBeenCalledWith("10001", null);
    });
  });

  describe("layouts", () => {
    it("shows a switch per layout, on for the offered ones", async () => {
      await renderSettings(spaceConfig({ layouts: layouts({ layouts: ["list", "table"] }) }));

      expect(screen.getByRole("switch", { name: "Offer List" })).toBeChecked();
      expect(screen.getByRole("switch", { name: "Offer Board" })).not.toBeChecked();
      expect(screen.getByRole("switch", { name: "Offer Table" })).toBeChecked();
    });

    it("switches a layout off and shows the config Rust returns", async () => {
      mocked.spaceSetLayouts.mockResolvedValue(spaceConfig({ layouts: layouts({ layouts: ["list", "table"] }) }));
      await renderSettings();

      await userEvent.click(screen.getByRole("switch", { name: "Offer Board" }));

      expect(mocked.spaceSetLayouts).toHaveBeenCalledWith("10001", layouts({ layouts: ["list", "table"] }));
      expect(await screen.findByRole("switch", { name: "Offer Board" })).not.toBeChecked();
    });

    it("switches a layout on in the fixed order", async () => {
      mocked.spaceSetLayouts.mockResolvedValue(spaceConfig());
      await renderSettings(spaceConfig({ layouts: layouts({ layouts: ["list", "table"] }) }));

      await userEvent.click(screen.getByRole("switch", { name: "Offer Board" }));

      expect(mocked.spaceSetLayouts).toHaveBeenCalledWith("10001", layouts());
    });

    it("makes the first remaining layout the default when the default is switched off", async () => {
      mocked.spaceSetLayouts.mockResolvedValue(spaceConfig());
      await renderSettings();

      await userEvent.click(screen.getByRole("switch", { name: "Offer List" }));

      expect(mocked.spaceSetLayouts).toHaveBeenCalledWith("10001", layouts({ layouts: ["board", "table"], defaultLayout: "board" }));
    });

    it("keeps the last layout on", async () => {
      await renderSettings(spaceConfig({ layouts: layouts({ layouts: ["table"], defaultLayout: "table" }) }));

      const last = screen.getByRole("switch", { name: "Offer Table" });
      expect(last).toBeDisabled();
      expect(screen.getByText("A Space keeps at least one layout")).toBeInTheDocument();
      expect(screen.getByRole("switch", { name: "Offer List" })).toBeEnabled();
    });

    it("limits the default layout to the offered layouts and presses the current one", async () => {
      await renderSettings(spaceConfig({ layouts: layouts({ layouts: ["list", "table"], defaultLayout: "table" }) }));

      const group = screen.getByRole("group", { name: "Default layout" });
      expect(within(group).getAllByRole("button").map((b) => b.textContent)).toEqual(["List", "Table"]);
      expect(within(group).getByRole("button", { name: "Table" })).toHaveAttribute("aria-pressed", "true");
    });

    it("saves a new default layout", async () => {
      mocked.spaceSetLayouts.mockResolvedValue(spaceConfig());
      await renderSettings();

      await userEvent.click(within(screen.getByRole("group", { name: "Default layout" })).getByRole("button", { name: "Board" }));

      expect(mocked.spaceSetLayouts).toHaveBeenCalledWith("10001", layouts({ defaultLayout: "board" }));
    });

    it("shows the grouping of new views and saves another one chosen from the menu", async () => {
      mocked.spaceSetLayouts.mockResolvedValue(spaceConfig({ layouts: layouts({ defaultGroup: "assignee" }) }));
      await renderSettings();
      const button = screen.getByRole("button", { name: "Grouping of new views" });
      expect(button).toHaveTextContent("Status");

      const menu = await openMenuOf("Grouping of new views");
      expect(within(menu).getByRole("menuitemradio", { name: "Status" })).toHaveAttribute("aria-checked", "true");
      expect(within(menu).getByRole("menuitemradio", { name: "Assignee" })).toHaveAttribute("aria-checked", "false");
      await choose(menu, "Assignee");

      expect(mocked.spaceSetLayouts).toHaveBeenCalledWith("10001", layouts({ defaultGroup: "assignee" }));
      expect(await screen.findByRole("button", { name: "Grouping of new views" })).toHaveTextContent("Assignee");
    });

    it("offers Status type right after Status in the grouping menu and saves it", async () => {
      mocked.spaceSetLayouts.mockResolvedValue(spaceConfig({ layouts: layouts({ defaultGroup: "statusType" }) }));
      await renderSettings();

      const menu = await openMenuOf("Grouping of new views");
      expect(rowsOf(menu).map((r) => r.textContent).slice(0, 2)).toEqual(["Status", "Status type"]);
      await choose(menu, "Status type");

      expect(mocked.spaceSetLayouts).toHaveBeenCalledWith("10001", layouts({ defaultGroup: "statusType" }));
      expect(await screen.findByRole("button", { name: "Grouping of new views" })).toHaveTextContent("Status type");
    });

    it("shows an error toast when the layouts cannot be saved", async () => {
      mocked.spaceSetLayouts.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderSettings();

      await userEvent.click(screen.getByRole("switch", { name: "Offer Board" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not change the layouts: Disk full");
      expect(screen.getByRole("switch", { name: "Offer Board" })).toBeChecked();
    });
  });

  describe("team", () => {
    it("counts the people on the team and those with activity in this Space", async () => {
      mocked.teamMatrix.mockResolvedValue(teamMatrix());
      await renderSettings(undefined, "Team");

      expect(await screen.findByText("1 person on the team")).toBeInTheDocument();
      expect(screen.getByText("2 with activity in this Space")).toBeInTheDocument();
    });

    it("counts people in the plural and reads the column of this Space only", async () => {
      const matrix = teamMatrix();
      matrix.people[1].cells[0] = teamCell({ active: true, involved: true });
      mocked.teamMatrix.mockResolvedValue(matrix);
      await renderSettings(undefined, "Team");

      expect(await screen.findByText("2 people on the team")).toBeInTheDocument();
      expect(screen.getByText("2 with activity in this Space")).toBeInTheDocument();
    });

    it("shows nobody on the team before the matrix has this Space", async () => {
      mocked.teamMatrix.mockResolvedValue({ spaces: [], people: [] });
      await renderSettings(undefined, "Team");

      expect(await screen.findByText("0 people on the team")).toBeInTheDocument();
      expect(screen.getByText("0 with activity in this Space")).toBeInTheDocument();
    });

    it("opens the Team settings when Edit Team… is clicked", async () => {
      await renderSettings(undefined, "Team");

      await userEvent.click(screen.getByRole("button", { name: "Edit Team…" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:team");
    });
  });

  describe("custom fields", () => {
    const reviewer = fieldSetting();
    const points = fieldSetting({ id: "customfield_10072", name: "Story points", appDetail: false, inDetail: false });

    it("points to the app-wide Custom fields settings when no field is picked", async () => {
      await renderSettings(spaceConfig({ fields: [] }), "Custom Fields");

      expect(screen.getByText("No fields are shown. Pick fields in Settings → Custom fields.")).toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "Custom Fields…" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:fields");
    });

    it("shows a box per place the app shows the field, ticked as the Space has it", async () => {
      await renderSettings(spaceConfig({ fields: [reviewer, { ...points, inList: false }] }), "Custom Fields");

      expect(screen.getByRole("checkbox", { name: "Reviewer in this Space's lists" })).toBeChecked();
      expect(screen.getByRole("checkbox", { name: "Reviewer in this Space's Issue view" })).toBeChecked();
      expect(screen.getByRole("checkbox", { name: "Story points in this Space's lists" })).not.toBeChecked();
    });

    it("shows no box for a place the app does not show the field", async () => {
      await renderSettings(spaceConfig({ fields: [points] }), "Custom Fields");

      expect(screen.queryByRole("checkbox", { name: "Story points in this Space's Issue view" })).not.toBeInTheDocument();
    });

    it.each([
      ["lists", "Reviewer in this Space's lists", false, true],
      ["Issue view", "Reviewer in this Space's Issue view", true, false],
    ])("unticking the %s box sends it off and keeps the other place", async (_place, name, inList, inDetail) => {
      mocked.spaceSetField.mockResolvedValue(spaceConfig({ fields: [fieldSetting({ inList, inDetail })] }));
      await renderSettings(spaceConfig({ fields: [reviewer] }), "Custom Fields");

      await userEvent.click(screen.getByRole("checkbox", { name }));

      expect(mocked.spaceSetField).toHaveBeenCalledWith("10001", "customfield_10071", inList, inDetail);
      await waitFor(() => expect(screen.getByRole("checkbox", { name })).not.toBeChecked());
    });

    it("shows an error toast when the field cannot be changed", async () => {
      mocked.spaceSetField.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderSettings(spaceConfig({ fields: [reviewer] }), "Custom Fields");

      await userEvent.click(screen.getByRole("checkbox", { name: "Reviewer in this Space's lists" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not change Reviewer: Disk full");
      expect(screen.getByRole("checkbox", { name: "Reviewer in this Space's lists" })).toBeChecked();
    });
  });
});

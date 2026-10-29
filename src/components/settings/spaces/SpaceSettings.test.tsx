import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type MenuEntry, type SpaceConfig, type SpaceLayouts } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { space, spaceConfig } from "../../../test/spaces";
import { Toasts } from "../../Toasts";
import { SpaceSettings } from "./SpaceSettings";

const mocked = vi.mocked(api, true);

const layouts = (patch: Partial<SpaceLayouts> = {}): SpaceLayouts => ({ layouts: ["list", "board", "table"], defaultLayout: "list", defaultGroup: "status", ...patch });

async function renderSettings(config: SpaceConfig = spaceConfig(), onBack = vi.fn()) {
  mocked.spaceConfig.mockResolvedValue(config);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SpaceSettings space={space()} onBack={onBack} />
      <Toasts />
    </QueryClientProvider>,
  );
  await screen.findByRole("region", { name: "Layouts" });
  return onBack;
}

const lastMenu = (): Exclude<MenuEntry, "separator">[] => mocked.popupMenu.mock.lastCall![0].filter((e): e is Exclude<MenuEntry, "separator"> => e !== "separator");
const runEntry = (text: string) => {
  const entry = lastMenu().find((e) => e.text === text);
  if (!entry) throw new Error(`No menu entry "${text}"`);
  return act(async () => entry.run?.());
};

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("SpaceSettings", () => {
  it("shows the Space's name and the General, Layouts, Statuses and Custom fields groups", async () => {
    await renderSettings();

    expect(screen.getByRole("heading", { name: "Website relaunch" })).toBeInTheDocument();
    for (const name of ["General", "Layouts", "Statuses", "Custom fields"]) expect(await screen.findByRole("region", { name })).toBeInTheDocument();
    expect(mocked.spaceConfig).toHaveBeenCalledWith("10001");
    expect(mocked.statusStylesList).toHaveBeenCalledWith("10001");
  });

  it("goes back when the Spaces button is clicked", async () => {
    const onBack = await renderSettings();

    await userEvent.click(screen.getByRole("button", { name: "Spaces" }));

    expect(onBack).toHaveBeenCalled();
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

      await userEvent.click(button);
      expect(lastMenu().find((e) => e.text === "Status")?.enabled).toBe(false);
      await runEntry("Assignee");

      expect(mocked.spaceSetLayouts).toHaveBeenCalledWith("10001", layouts({ defaultGroup: "assignee" }));
      expect(await screen.findByRole("button", { name: "Grouping of new views" })).toHaveTextContent("Assignee");
    });

    it("shows an error toast when the layouts cannot be saved", async () => {
      mocked.spaceSetLayouts.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderSettings();

      await userEvent.click(screen.getByRole("switch", { name: "Offer Board" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not change the layouts: Disk full");
      expect(screen.getByRole("switch", { name: "Offer Board" })).toBeChecked();
    });
  });

  describe("custom fields", () => {
    const fields = [
      { id: "customfield_10071", name: "Reviewer", visibility: "visible" as const },
      { id: "customfield_10072", name: "Story points", visibility: "hidden" as const },
    ];

    it("says fields appear after the first sync when there are none", async () => {
      await renderSettings(spaceConfig({ fields: [] }));

      expect(screen.getByText("Custom fields appear after the first sync.")).toBeInTheDocument();
    });

    it("shows a row per field with its visibility", async () => {
      await renderSettings(spaceConfig({ fields }));

      expect(screen.getByRole("button", { name: "Visibility of Reviewer" })).toHaveTextContent("Visible");
      expect(screen.getByRole("button", { name: "Visibility of Story points" })).toHaveTextContent("Hidden everywhere");
    });

    it("offers the four visibilities with the current one disabled", async () => {
      await renderSettings(spaceConfig({ fields }));

      await userEvent.click(screen.getByRole("button", { name: "Visibility of Reviewer" }));

      expect(lastMenu().map((e) => [e.text, e.enabled])).toEqual([
        ["Visible", false],
        ["Hidden in list", true],
        ["Hidden in detail", true],
        ["Hidden everywhere", true],
      ]);
    });

    it("saves the chosen visibility and shows the config Rust returns", async () => {
      mocked.spaceSetField.mockResolvedValue(spaceConfig({ fields: [{ ...fields[0], visibility: "hiddenInList" }, fields[1]] }));
      await renderSettings(spaceConfig({ fields }));
      await userEvent.click(screen.getByRole("button", { name: "Visibility of Reviewer" }));

      await runEntry("Hidden in list");

      expect(mocked.spaceSetField).toHaveBeenCalledWith("10001", "customfield_10071", "hiddenInList");
      expect(await screen.findByRole("button", { name: "Visibility of Reviewer" })).toHaveTextContent("Hidden in list");
    });

    it("shows an error toast when the visibility cannot be saved", async () => {
      mocked.spaceSetField.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderSettings(spaceConfig({ fields }));
      await userEvent.click(screen.getByRole("button", { name: "Visibility of Reviewer" }));

      await runEntry("Hidden in detail");

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not change Reviewer: Disk full");
      expect(screen.getByRole("button", { name: "Visibility of Reviewer" })).toHaveTextContent("Visible");
    });
  });
});

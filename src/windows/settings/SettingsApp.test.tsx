import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Puzzle } from "lucide-react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { registerSettingsPage } from "../../registry/settingsPages";
import { resetStore } from "../../state/store";
import { emitEvent } from "../../test/apiMock";
import { settings } from "../../test/settings";
import { SettingsApp } from "./SettingsApp";

const mocked = vi.mocked(api, true);

async function renderSettings() {
  render(<SettingsApp client={createQueryClient()} />);
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("open_settings", expect.any(Function)));
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.appInfo.mockResolvedValue({ name: "Taskboard", version: "4.5.6" });
  mocked.getSettings.mockResolvedValue(settings());
  mocked.appearanceSet.mockImplementation(async (appearance) => settings({ appearance }));
});

afterEach(() => {
  window.history.replaceState({}, "", "/");
});

describe("SettingsApp", () => {
  it("lists the Jira, Workspace and System pages in order and opens on Connection", async () => {
    await renderSettings();

    const nav = screen.getByRole("navigation", { name: "Settings pages" });
    const entries = within(nav).getAllByRole("button").map((b) => b.textContent?.trim());
    expect(entries).toEqual([
      "Connection",
      "Privacy & Data",
      "Projects",
      "Sync",
      "Spaces",
      "Team",
      "Statuses",
      "Custom fields",
      "Behavior",
      "Appearance",
      "Shortcuts",
      "Status",
      "About",
    ]);
    expect(nav.textContent).toMatch(
      /Jira.*Connection.*Privacy & Data.*Projects.*Sync.*Workspace.*Spaces.*Team.*Statuses.*Custom fields.*Behavior.*System.*Appearance.*Shortcuts.*Status.*About/,
    );
    expect(screen.getByRole("heading", { name: "Connection" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Connection/ })).toHaveAttribute("aria-current", "page");
  });

  it("shows the Sync page when chosen in the page list", async () => {
    await renderSettings();

    await userEvent.click(screen.getByRole("button", { name: /Sync/ }));

    expect(screen.getByRole("heading", { name: "Sync" })).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "Sync Now" })).toBeInTheDocument();
  });

  it("shows the Privacy & Data page with the cache reset when chosen in the page list", async () => {
    await renderSettings();

    await userEvent.click(screen.getByRole("button", { name: /Privacy & Data/ }));

    expect(screen.getByRole("heading", { name: "Privacy & Data" })).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "Reset Cache" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "What Taskboard reads" })).not.toBeInTheDocument();
  });

  it("shows the Token scopes and What Taskboard reads groups on the Connection page", async () => {
    await renderSettings();

    expect(await screen.findByRole("heading", { name: "Token scopes" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "What Taskboard reads" })).toBeInTheDocument();
  });

  describe("search", () => {
    it("finds the Spaces page by its Layouts section", async () => {
      await renderSettings();

      await userEvent.type(screen.getByRole("searchbox", { name: "Search Settings" }), "Layouts");

      const nav = screen.getByRole("navigation", { name: "Settings pages" });
      expect(within(nav).getAllByRole("button").map((b) => b.textContent?.trim())).toEqual([expect.stringContaining("Spaces")]);
    });

    it("finds both the Spaces page by its Custom fields section and the Custom fields page", async () => {
      await renderSettings();

      await userEvent.type(screen.getByRole("searchbox", { name: "Search Settings" }), "Custom fields");

      const nav = screen.getByRole("navigation", { name: "Settings pages" });
      const entries = within(nav).getAllByRole("button").map((b) => b.textContent?.trim());
      expect(entries).toEqual([expect.stringContaining("Spaces"), "Custom fields"]);
      expect(entries[0]).toContain("Custom fields");
    });

    it("finds both the Spaces page by its Statuses section and the Statuses page", async () => {
      await renderSettings();

      await userEvent.type(screen.getByRole("searchbox", { name: "Search Settings" }), "Statuses");

      const nav = screen.getByRole("navigation", { name: "Settings pages" });
      const entries = within(nav).getAllByRole("button").map((b) => b.textContent?.trim());
      expect(entries).toEqual([expect.stringContaining("Spaces"), "Statuses"]);
      expect(entries[0]).toContain("Statuses");
    });

    it("leaves no page when the text matches nothing", async () => {
      await renderSettings();

      await userEvent.type(screen.getByRole("searchbox", { name: "Search Settings" }), "zzz");

      expect(screen.getByText("No Results")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Behavior/ })).not.toBeInTheDocument();
    });

    it("keeps Behavior when the text matches part of its label", async () => {
      await renderSettings();

      await userEvent.type(screen.getByRole("searchbox", { name: "Search Settings" }), "beh");

      expect(screen.getByRole("button", { name: /Behavior/ })).toBeInTheDocument();
      expect(screen.queryByText("No Results")).not.toBeInTheDocument();
    });

    it.each(["What Taskboard reads", "Token scopes"])("finds the Connection page by its %s section", async (section) => {
      await renderSettings();

      await userEvent.type(screen.getByRole("searchbox", { name: "Search Settings" }), section);

      const nav = screen.getByRole("navigation", { name: "Settings pages" });
      expect(within(nav).getAllByRole("button").map((b) => b.textContent?.trim())).toEqual([expect.stringContaining("Connection")]);
      expect(within(nav).getByRole("button")).toHaveTextContent(section);
    });
  });

  describe("Behavior page", () => {
    beforeEach(() => {
      window.history.replaceState({}, "", "/?page=behavior");
    });

    it("shows the behavior settings without an About group", async () => {
      await renderSettings();

      expect(await screen.findByText("Count an issue as read after")).toBeInTheDocument();
      expect(screen.queryByText("Version 4.5.6")).not.toBeInTheDocument();
    });
  });

  describe("About page", () => {
    beforeEach(() => {
      window.history.replaceState({}, "", "/?page=about");
    });

    it("shows the app version", async () => {
      await renderSettings();

      expect(await screen.findByText("Version 4.5.6")).toBeInTheDocument();
    });

    it("opens the log folder from its button", async () => {
      await renderSettings();

      await userEvent.click(screen.getByRole("button", { name: "Open Log Folder" }));

      expect(mocked.openLogFolder).toHaveBeenCalled();
    });

    it("shows an error toast when the log folder cannot be opened", async () => {
      mocked.openLogFolder.mockRejectedValue({ kind: "io", message: "No such folder", code: null });
      await renderSettings();

      await userEvent.click(screen.getByRole("button", { name: "Open Log Folder" }));

      expect(await screen.findByText("Could not open the log folder: No such folder")).toBeInTheDocument();
    });
  });

  describe("Appearance page", () => {
    beforeEach(() => {
      window.history.replaceState({}, "", "/?page=appearance");
    });

    it("shows the current appearance under System", async () => {
      mocked.getSettings.mockResolvedValue(settings({ appearance: "dark" }));
      await renderSettings();

      const group = screen.getByRole("group", { name: "Appearance" });
      await waitFor(() => expect(within(group).getByRole("button", { name: "Dark" })).toHaveAttribute("aria-pressed", "true"));
    });

    it("shows an error toast when the appearance cannot be changed", async () => {
      mocked.appearanceSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderSettings();

      await userEvent.click(within(screen.getByRole("group", { name: "Appearance" })).getByRole("button", { name: "Light" }));

      expect(await screen.findByText("Could not change the appearance: Disk full")).toBeInTheDocument();
    });
  });

  describe("page selection", () => {
    beforeEach(() => {
      registerSettingsPage({ id: "extras", group: "System", label: "Demo Extras", icon: Puzzle, order: 5, Body: () => <p>Extras body</p> });
    });

    it("shows the page chosen in the page list", async () => {
      await renderSettings();

      await userEvent.click(screen.getByRole("button", { name: /Demo Extras/ }));

      expect(screen.getByRole("heading", { name: "Demo Extras" })).toBeInTheDocument();
      expect(screen.getByText("Extras body")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Demo Extras/ })).toHaveAttribute("aria-current", "page");
    });

    it("opens at the page named in the window address", async () => {
      window.history.replaceState({}, "", "/?page=extras");

      await renderSettings();

      expect(screen.getByRole("heading", { name: "Demo Extras" })).toBeInTheDocument();
    });

    it("shows Behavior on the open_settings event with behavior", async () => {
      window.history.replaceState({}, "", "/?page=extras");
      await renderSettings();

      act(() => emitEvent(api, "open_settings", "behavior"));

      expect(screen.getByRole("heading", { name: "Behavior", level: 2 })).toBeInTheDocument();
      expect(screen.queryByText("Extras body")).not.toBeInTheDocument();
    });

    it.each([
      ["cache", "Privacy & Data"],
      ["scopes", "Connection"],
      ["reads", "Connection"],
    ])("shows the page of the %s section on the open_settings event: %s", async (section, heading) => {
      window.history.replaceState({}, "", "/?page=extras");
      await renderSettings();

      act(() => emitEvent(api, "open_settings", section));

      expect(screen.getByRole("heading", { name: heading, level: 2 })).toBeInTheDocument();
      expect(screen.queryByText("Extras body")).not.toBeInTheDocument();
    });
  });
});

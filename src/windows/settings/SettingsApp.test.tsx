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
      "Notifications",
      "Appearance",
      "Shortcuts",
      "Status",
      "About",
    ]);
    expect(nav.textContent).toMatch(
      /Jira.*Connection.*Privacy & Data.*Projects.*Sync.*Workspace.*Spaces.*Team.*Statuses.*Custom fields.*Behavior.*Notifications.*System.*Appearance.*Shortcuts.*Status.*About/,
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

  it("shows the Privacy & Data page with the Refetch from Jira button when chosen in the page list", async () => {
    await renderSettings();

    await userEvent.click(screen.getByRole("button", { name: /Privacy & Data/ }));

    expect(screen.getByRole("heading", { name: "Privacy & Data" })).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "Refetch from Jira" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "What Taskboard reads" })).not.toBeInTheDocument();
  });

  it("shows the Token scopes and What Taskboard reads groups on the Connection page", async () => {
    await renderSettings();

    expect(await screen.findByRole("heading", { name: "Token scopes" })).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "What Taskboard reads" })).toBeInTheDocument();
  });

  describe("search", () => {
    const search = (text: string) => userEvent.type(screen.getByRole("searchbox", { name: "Search Settings" }), text);
    const hits = () => within(screen.getByRole("group", { name: "Results" })).getAllByRole("button");

    it("replaces the page list with one Results group of hits", async () => {
      await renderSettings();

      await search("Appearance");

      expect(screen.queryByRole("group", { name: "Jira" })).not.toBeInTheDocument();
      expect(hits()[0]).toHaveTextContent("Appearance");
    });

    it("finds the Spaces page by its Layouts keyword", async () => {
      await renderSettings();

      await search("Layouts");

      expect(hits()[0]).toHaveAccessibleName("Spaces");
    });

    it("lists the Custom fields page first when its label is typed", async () => {
      await renderSettings();

      await search("Custom fields");

      expect(hits()[0]).toHaveAccessibleName("Custom fields");
    });

    it("lists a row as a two-line hit showing its page and its title", async () => {
      await renderSettings();

      await search("Which issues");

      const hit = screen.getByRole("button", { name: "Which issues, Notifications" });
      expect(within(hit).getByText("Notifications")).toBeInTheDocument();
      expect(within(hit).getByText("Which issues")).toBeInTheDocument();
    });

    it("finds a row by one of its keywords", async () => {
      await renderSettings();

      await search("keychain");

      expect(screen.getByRole("button", { name: "API token, Connection" })).toBeInTheDocument();
    });

    it("opens the page of a row hit and marks the row as found when the hit is clicked", async () => {
      await renderSettings();
      await search("Jira data");

      await userEvent.click(screen.getByRole("button", { name: "Jira data, Privacy & Data" }));

      expect(screen.getByRole("heading", { name: "Privacy & Data", level: 2 })).toBeInTheDocument();
      await waitFor(() => expect(screen.getByRole("region", { name: "Jira data" })).toHaveClass("found"));
    });

    it("opens the first hit when Enter is pressed", async () => {
      await renderSettings();

      await search("Which issues{Enter}");

      expect(screen.getByRole("heading", { name: "Notifications", level: 2 })).toBeInTheDocument();
      await waitFor(() => expect(screen.getByRole("region", { name: "Which issues" })).toHaveClass("found"));
    });

    it("leaves no hit when the text matches nothing", async () => {
      await renderSettings();

      await search("zzz");

      expect(screen.getByText("No Results")).toBeInTheDocument();
      expect(screen.queryByRole("group", { name: "Results" })).not.toBeInTheDocument();
    });

    it("keeps Behavior when the text matches part of its label", async () => {
      await renderSettings();

      await search("beh");

      expect(screen.getByRole("button", { name: "Behavior" })).toBeInTheDocument();
      expect(screen.queryByText("No Results")).not.toBeInTheDocument();
    });

    it("shows the page list again when the text is cleared", async () => {
      await renderSettings();
      await search("zzz");

      await userEvent.clear(screen.getByRole("searchbox", { name: "Search Settings" }));

      expect(screen.getByRole("group", { name: "Workspace" })).toBeInTheDocument();
      expect(screen.queryByText("No Results")).not.toBeInTheDocument();
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

      expect(await screen.findByText("Taskboard 4.5.6")).toBeInTheDocument();
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

      const group = screen.getByRole("group", { name: "Mode" });
      await waitFor(() => expect(within(group).getByRole("button", { name: "Dark" })).toHaveAttribute("aria-pressed", "true"));
    });

    it("shows an error toast when the appearance cannot be changed", async () => {
      mocked.appearanceSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderSettings();

      await userEvent.click(within(screen.getByRole("group", { name: "Mode" })).getByRole("button", { name: "Light" }));

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

    it("opens the page of a row on the open_settings event and marks the row as found", async () => {
      window.history.replaceState({}, "", "/?page=extras");
      await renderSettings();

      act(() => emitEvent(api, "open_settings", "cache"));

      expect(screen.getByRole("heading", { name: "Privacy & Data", level: 2 })).toBeInTheDocument();
      await waitFor(() => expect(screen.getByRole("region", { name: "Jira data" })).toHaveClass("found"));
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

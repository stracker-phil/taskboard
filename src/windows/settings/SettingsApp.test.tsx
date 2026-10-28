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
});

afterEach(() => {
  window.history.replaceState({}, "", "/");
});

describe("SettingsApp", () => {
  it("lists the Jira pages before General and opens on Connection", async () => {
    await renderSettings();

    const nav = screen.getByRole("navigation", { name: "Settings pages" });
    const entries = within(nav).getAllByRole("button").map((b) => b.textContent?.trim());
    expect(entries).toEqual(["Connection", "Projects", "Spaces", "Statuses", "Sync", "General", "Shortcuts"]);
    expect(nav.textContent).toMatch(/Jira.*Connection.*Projects.*Spaces.*Statuses.*Sync.*System.*General.*Shortcuts/);
    expect(screen.getByRole("heading", { name: "Connection" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Connection/ })).toHaveAttribute("aria-current", "page");
  });

  it("shows the Sync page when chosen in the page list", async () => {
    await renderSettings();

    await userEvent.click(screen.getByRole("button", { name: /Sync/ }));

    expect(screen.getByRole("heading", { name: "Sync" })).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: "Reset Cache" })).toBeInTheDocument();
  });

  it("shows the Statuses page when chosen in the page list", async () => {
    await renderSettings();

    await userEvent.click(screen.getByRole("button", { name: /Statuses/ }));

    expect(screen.getByRole("button", { name: /Statuses/ })).toHaveAttribute("aria-current", "page");
    expect(await screen.findByText("Statuses appear after the first sync.")).toBeInTheDocument();
  });

  describe("search", () => {
    it("leaves no page when the text matches nothing", async () => {
      await renderSettings();

      await userEvent.type(screen.getByRole("searchbox", { name: "Search Settings" }), "zzz");

      expect(screen.getByText("No Results")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /General/ })).not.toBeInTheDocument();
    });

    it("keeps General when the text matches part of its label", async () => {
      await renderSettings();

      await userEvent.type(screen.getByRole("searchbox", { name: "Search Settings" }), "gen");

      expect(screen.getByRole("button", { name: /General/ })).toBeInTheDocument();
      expect(screen.queryByText("No Results")).not.toBeInTheDocument();
    });
  });

  describe("General page", () => {
    beforeEach(() => {
      window.history.replaceState({}, "", "/?page=general");
    });

    it("shows the app version in the About group", async () => {
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

    it("shows General on the open_settings event with general", async () => {
      window.history.replaceState({}, "", "/?page=extras");
      await renderSettings();

      act(() => emitEvent(api, "open_settings", "general"));

      expect(screen.getByRole("heading", { name: "General" })).toBeInTheDocument();
      expect(screen.queryByText("Extras body")).not.toBeInTheDocument();
    });
  });
});

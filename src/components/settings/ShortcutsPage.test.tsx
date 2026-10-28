import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { registerCoreScopes } from "../../commands";
import { clearCommands, registerCommands, type Command } from "../../commands/registry";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
import { ShortcutsPage } from "./ShortcutsPage";

const mocked = vi.mocked(api, true);

const command = (id: string, title: string, scope: string, defaultBinding?: string): Command => ({ id, title, scope, defaultBinding, run: () => {} });

async function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ShortcutsPage />
    </QueryClientProvider>,
  );
  await screen.findByText("Open Search");
}

const field = (title: string) => screen.getByRole("button", { name: `Record shortcut for ${title}` });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.getKeybindings.mockResolvedValue({});
  clearCommands();
  registerCoreScopes();
  registerCommands([
    command("search.open", "Open Search", "main", "Mod+K"),
    command("sidebar.toggle", "Toggle sidebar", "global", "Mod+B"),
    command("issue.copy", "Copy Link", "main"),
  ]);
});

describe("ShortcutsPage", () => {
  describe("listing", () => {
    it("lists each command with its scope label and its key", async () => {
      await renderPage();

      expect(field("Open Search")).toHaveTextContent("⌘K");
      expect(field("Toggle sidebar")).toHaveTextContent("⌘B");
      expect(field("Copy Link")).toHaveTextContent("unbound");
      expect(screen.getByText("Every window")).toBeInTheDocument();
      expect(screen.getAllByText("Taskboard window").length).toBeGreaterThan(0);
    });

    it("shows a saved override instead of the default", async () => {
      mocked.getKeybindings.mockResolvedValue({ "search.open": "Mod+Shift+P" });
      await renderPage();

      expect(await screen.findByRole("button", { name: "Record shortcut for Open Search" })).toHaveTextContent("⌘⇧P");
    });

    it("lists the app menu's fixed keys", async () => {
      await renderPage();

      const menu = screen.getByRole("region", { name: "In the menu" });
      expect(within(menu).getByText("Settings")).toBeInTheDocument();
      expect(within(menu).getByText("⌘,")).toBeInTheDocument();
      expect(within(menu).getByText("Refresh")).toBeInTheDocument();
      expect(within(menu).getByText("⌘R")).toBeInTheDocument();
      expect(within(menu).getByText("Full Sync")).toBeInTheDocument();
      expect(within(menu).getByText("⌘⇧R")).toBeInTheDocument();
    });
  });

  describe("search", () => {
    it("filters by title", async () => {
      await renderPage();

      await userEvent.type(screen.getByRole("textbox", { name: "Search shortcuts" }), "sidebar");

      expect(screen.getByText("Toggle sidebar")).toBeInTheDocument();
      expect(screen.queryByText("Open Search")).not.toBeInTheDocument();
    });

    it("filters by command id", async () => {
      await renderPage();

      await userEvent.type(screen.getByRole("textbox", { name: "Search shortcuts" }), "issue.copy");

      expect(screen.getByText("Copy Link")).toBeInTheDocument();
      expect(screen.queryByText("Open Search")).not.toBeInTheDocument();
    });

    it("says so when nothing matches", async () => {
      await renderPage();

      await userEvent.type(screen.getByRole("textbox", { name: "Search shortcuts" }), "zzz");

      expect(screen.getByText("No matching commands.")).toBeInTheDocument();
    });
  });

  describe("editing", () => {
    it("saves a recorded key as the command's override", async () => {
      await renderPage();

      await userEvent.click(field("Copy Link"));
      await userEvent.keyboard("{Meta>}{Shift>}l{/Shift}{/Meta}");

      expect(mocked.setKeybindings).toHaveBeenCalledWith({ "issue.copy": "Mod+Shift+L" });
    });

    it("keeps the other overrides when saving a new one", async () => {
      mocked.getKeybindings.mockResolvedValue({ "sidebar.toggle": "Mod+J" });
      await renderPage();
      await screen.findByText("⌘J");

      await userEvent.click(field("Copy Link"));
      await userEvent.keyboard("{Meta>}l{/Meta}");

      expect(mocked.setKeybindings).toHaveBeenCalledWith({ "sidebar.toggle": "Mod+J", "issue.copy": "Mod+L" });
    });

    it("saves null when the shortcut is removed", async () => {
      await renderPage();

      await userEvent.click(screen.getByRole("button", { name: "Remove shortcut for Open Search" }));

      expect(mocked.setKeybindings).toHaveBeenCalledWith({ "search.open": null });
    });

    it("offers no remove button for an unbound command", async () => {
      await renderPage();

      expect(screen.queryByRole("button", { name: "Remove shortcut for Copy Link" })).not.toBeInTheDocument();
    });

    it("shows Reset only for an overridden command and saves without its override", async () => {
      mocked.getKeybindings.mockResolvedValue({ "search.open": "Mod+Shift+P", "sidebar.toggle": "Mod+J" });
      await renderPage();
      await screen.findByRole("button", { name: "Reset Open Search" });

      expect(screen.queryByRole("button", { name: "Reset Copy Link" })).not.toBeInTheDocument();
      await userEvent.click(screen.getByRole("button", { name: "Reset Open Search" }));

      expect(mocked.setKeybindings).toHaveBeenCalledWith({ "sidebar.toggle": "Mod+J" });
    });

    it("shows no Reset button without overrides", async () => {
      await renderPage();

      expect(screen.queryByRole("button", { name: /^Reset/ })).not.toBeInTheDocument();
    });
  });

  describe("conflicts", () => {
    it("shows no alert while every key is unique", async () => {
      await renderPage();

      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    });

    it("names both commands that share a key", async () => {
      mocked.getKeybindings.mockResolvedValue({ "issue.copy": "Mod+K" });
      await renderPage();

      expect(await screen.findByRole("alert")).toHaveTextContent("⌘K is used by both “Open Search” and “Copy Link”.");
    });
  });
});

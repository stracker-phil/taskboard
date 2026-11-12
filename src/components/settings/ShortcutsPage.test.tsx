import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { registerCoreScopes } from "../../commands";
import { clearCommands, registerCommands, type Command } from "../../commands/registry";
import { ConfirmHost } from "../ConfirmSheet";
import { Toasts } from "../Toasts";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
import { ShortcutsPage } from "./ShortcutsPage";

const mocked = vi.mocked(api, true);

const command = (id: string, title: string, scope: string, defaultBinding?: string): Command => ({ id, title, scope, defaultBinding, run: () => {} });

async function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ShortcutsPage />
      <ConfirmHost />
      <Toasts />
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

    it.each([
      ["Close Tab or Window", "⌘W"],
      ["New Tab", "⌘⇧T"],
      ["Previous Tab", "⌘⇧["],
      ["Next Tab", "⌘⇧]"],
      ["Reopen Closed Tab", "⌘⌥T"],
    ])("lists the menu's tab key %s as %s", async (title, key) => {
      await renderPage();

      const menu = screen.getByRole("region", { name: "In the menu" });
      expect(within(menu).getByText(title)).toBeInTheDocument();
      expect(within(menu).getByText(key)).toBeInTheDocument();
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

  describe("taking a key another command holds", () => {
    const record = async (title: string, keys: string) => {
      await userEvent.click(field(title));
      await userEvent.keyboard(keys);
    };

    it("asks before taking the key and saves once, with both changes, after Reassign", async () => {
      mocked.getKeybindings.mockResolvedValue({ "sidebar.toggle": "Mod+J" });
      await renderPage();
      await screen.findByText("⌘J");

      await record("Copy Link", "{Meta>}k{/Meta}");

      const sheet = await screen.findByRole("alertdialog");
      expect(within(sheet).getByRole("heading")).toHaveTextContent("⌘K is used by “Open Search”");
      expect(sheet).toHaveTextContent("Reassign it to “Copy Link”? “Open Search” will have no shortcut.");
      expect(mocked.setKeybindings).not.toHaveBeenCalled();

      await userEvent.click(within(sheet).getByRole("button", { name: "Reassign" }));

      expect(mocked.setKeybindings).toHaveBeenCalledTimes(1);
      expect(mocked.setKeybindings).toHaveBeenCalledWith({ "sidebar.toggle": "Mod+J", "search.open": null, "issue.copy": "Mod+K" });
    });

    it("asks about a key held in an overlapping ancestor scope", async () => {
      await renderPage();

      await record("Copy Link", "{Meta>}b{/Meta}");

      expect(await screen.findByRole("alertdialog")).toHaveTextContent("⌘B is used by “Toggle sidebar”");
    });

    it("saves nothing when Cancel is chosen", async () => {
      await renderPage();
      await record("Copy Link", "{Meta>}k{/Meta}");

      await userEvent.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      expect(mocked.setKeybindings).not.toHaveBeenCalled();
      expect(field("Open Search")).toHaveTextContent("⌘K");
      expect(field("Copy Link")).toHaveTextContent("unbound");
    });

    it("saves nothing when the sheet is dismissed with Escape", async () => {
      await renderPage();
      await record("Copy Link", "{Meta>}k{/Meta}");
      await screen.findByRole("alertdialog");

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      expect(mocked.setKeybindings).not.toHaveBeenCalled();
    });

    it("saves at once, without asking, when no command holds the key", async () => {
      await renderPage();

      await record("Copy Link", "{Meta>}l{/Meta}");

      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      expect(mocked.setKeybindings).toHaveBeenCalledWith({ "issue.copy": "Mod+L" });
    });

    it("asks when Reset would restore a default another command holds now", async () => {
      mocked.getKeybindings.mockResolvedValue({ "search.open": "Mod+J", "issue.copy": "Mod+K" });
      await renderPage();
      await screen.findByRole("button", { name: "Reset Open Search" });

      await userEvent.click(screen.getByRole("button", { name: "Reset Open Search" }));

      const sheet = await screen.findByRole("alertdialog");
      expect(sheet).toHaveTextContent("⌘K is used by “Copy Link”");
      await userEvent.click(within(sheet).getByRole("button", { name: "Reassign" }));
      expect(mocked.setKeybindings).toHaveBeenCalledWith({ "issue.copy": null });
    });
  });

  describe("taking a key that belongs to the menu", () => {
    it("refuses with a toast and saves nothing", async () => {
      await renderPage();

      await userEvent.click(field("Copy Link"));
      await userEvent.keyboard("{Meta>}r{/Meta}");

      expect(await screen.findByText("⌘R belongs to Refresh in the menu.")).toBeInTheDocument();
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      expect(mocked.setKeybindings).not.toHaveBeenCalled();
    });
  });

  describe("Reset and the default key", () => {
    it("shows Reset while a command's key differs from its default", async () => {
      mocked.getKeybindings.mockResolvedValue({ "search.open": "Mod+J" });
      await renderPage();

      expect(await screen.findByRole("button", { name: "Reset Open Search" })).toBeInTheDocument();
    });

    it("shows no Reset for a stored override equal to the default", async () => {
      mocked.getKeybindings.mockResolvedValue({ "search.open": "Mod+K" });
      await renderPage();
      await screen.findByRole("button", { name: "Remove shortcut for Open Search" });

      expect(screen.queryByRole("button", { name: "Reset Open Search" })).not.toBeInTheDocument();
    });

    it("saves without an override when the default key is recorded", async () => {
      mocked.getKeybindings.mockResolvedValue({ "search.open": "Mod+J", "sidebar.toggle": "Mod+H" });
      await renderPage();
      await screen.findByText("⌘J");

      await userEvent.click(field("Open Search"));
      await userEvent.keyboard("{Meta>}k{/Meta}");

      expect(mocked.setKeybindings).toHaveBeenCalledWith({ "sidebar.toggle": "Mod+H" });
    });
  });

  describe("fixed slots for Reset and Remove", () => {
    const slots = (title: string) => Array.from(field(title).closest(".shortcut-control")!.children).slice(0, 2);
    const isPlaceholder = (slot: Element) => slot.getAttribute("aria-hidden") === "true" && slot.tagName !== "BUTTON";

    it("puts a placeholder before Remove while nothing is overridden", async () => {
      await renderPage();

      const [first, second] = slots("Open Search");
      expect(isPlaceholder(first)).toBe(true);
      expect(second).toHaveAccessibleName("Remove shortcut for Open Search");
    });

    it("keeps Remove in the second slot when Reset shows in the first", async () => {
      mocked.getKeybindings.mockResolvedValue({ "search.open": "Mod+Shift+P" });
      await renderPage();
      await screen.findByRole("button", { name: "Reset Open Search" });

      const [first, second] = slots("Open Search");
      expect(first).toHaveAccessibleName("Reset Open Search");
      expect(second).toHaveAccessibleName("Remove shortcut for Open Search");
    });

    it("shows Reset in the first slot and a placeholder in the second once the shortcut is removed", async () => {
      mocked.getKeybindings.mockResolvedValue({ "search.open": null });
      await renderPage();
      await screen.findByRole("button", { name: "Reset Open Search" });

      const [first, second] = slots("Open Search");
      expect(first).toHaveAccessibleName("Reset Open Search");
      expect(isPlaceholder(second)).toBe(true);
    });

    it("holds two placeholders for an unbound command", async () => {
      await renderPage();

      expect(slots("Copy Link").map(isPlaceholder)).toEqual([true, true]);
    });
  });

  describe("conflicts", () => {
    it("shows the alert after the window's shortcuts and before the menu's", async () => {
      mocked.getKeybindings.mockResolvedValue({ "issue.copy": "Mod+K" });
      await renderPage();

      const alert = await screen.findByRole("alert");

      const windowGroup = screen.getByRole("region", { name: "In the Taskboard window" });
      const menuGroup = screen.getByRole("region", { name: "In the menu" });
      expect(windowGroup.compareDocumentPosition(alert) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(alert.compareDocumentPosition(menuGroup) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

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

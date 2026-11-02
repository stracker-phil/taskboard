import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Nav } from "../api";
import { installCommands, useCommandKeys } from "../commands";
import { clearCommands } from "../commands/registry";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { nav as navOf } from "../test/nav";
import { NavArrows } from "./NavArrows";

const mocked = vi.mocked(api, true);

const AT_LIST = { listId: 11, viewId: 101 };
const OTHER = { listId: 12, viewId: 5 };

function Keys() {
  useCommandKeys();
  return null;
}

async function renderArrows(current: Nav) {
  mocked.navCurrent.mockResolvedValue(current);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Keys />
      <NavArrows />
    </QueryClientProvider>,
  );
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("keybindings_changed", expect.any(Function)));
  if (current.canBack) await waitFor(() => expect(screen.getByRole("button", { name: "Back" })).toBeEnabled());
  else await waitFor(() => expect(mocked.navCurrent).toHaveBeenCalled());
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("main");
});

afterEach(() => clearCommands());

describe("NavArrows", () => {
  describe("availability", () => {
    it.each([
      ["neither", false, false],
      ["only Back", true, false],
      ["only Forward", false, true],
      ["both", true, true],
    ])("enables %s when Rust reports where it can go", async (_name, canBack, canForward) => {
      await renderArrows(navOf(AT_LIST, { canBack, canForward }));

      await waitFor(() => expect(screen.getByRole("button", { name: "Back" })).toHaveProperty("disabled", !canBack));
      expect(screen.getByRole("button", { name: "Forward" })).toHaveProperty("disabled", !canForward);
    });

    it("names the buttons with their shortcuts", async () => {
      await renderArrows(navOf(AT_LIST, { canBack: true, canForward: true }));

      expect(screen.getByRole("button", { name: "Back" })).toHaveAttribute("title", "Back (⌘[)");
      expect(screen.getByRole("button", { name: "Forward" })).toHaveAttribute("title", "Forward (⌘])");
    });
  });

  describe("clicking", () => {
    it.each([
      ["Back", () => mocked.navBack, () => mocked.navForward],
      ["Forward", () => mocked.navForward, () => mocked.navBack],
    ])("%s opens the Issue view on the issue of the place Rust answers with", async (name, asked, other) => {
      await renderArrows(navOf(AT_LIST, { canBack: true, canForward: true }));
      asked().mockResolvedValue(navOf(OTHER, { issue: "10042", canBack: true, canForward: true }));
      await waitFor(() => expect(screen.getByRole("button", { name })).toBeEnabled());

      await userEvent.click(screen.getByRole("button", { name }));

      await waitFor(() => expect(useStore.getState().issueView).toEqual({ origin: OTHER, stack: ["10042"] }));
      expect(other()).not.toHaveBeenCalled();
    });

    it.each([
      ["Back", () => mocked.navBack],
      ["Forward", () => mocked.navForward],
    ])("%s closes the Issue view when the place Rust answers with has no issue", async (name, asked) => {
      resetStore({ issueView: { origin: AT_LIST, stack: ["10042"] } });
      await renderArrows(navOf(AT_LIST, { issue: "10042", canBack: true, canForward: true }));
      asked().mockResolvedValue(navOf(AT_LIST, { issue: null, canBack: true, canForward: true }));

      await userEvent.click(screen.getByRole("button", { name }));

      await waitFor(() => expect(useStore.getState().issueView).toBeNull());
    });

    it("does nothing when a disabled arrow is clicked", async () => {
      await renderArrows(navOf(AT_LIST));

      await userEvent.click(screen.getByRole("button", { name: "Back" }));
      await userEvent.click(screen.getByRole("button", { name: "Forward" }));

      expect(mocked.navBack).not.toHaveBeenCalled();
      expect(mocked.navForward).not.toHaveBeenCalled();
    });
  });

  describe("shortcuts", () => {
    it.each([
      ["⌘[", "{Meta>}[[{/Meta}", () => mocked.navBack],
      ["⌘]", "{Meta>}]{/Meta}", () => mocked.navForward],
    ])("%s goes there when it is possible", async (_key, keys, asked) => {
      await renderArrows(navOf(AT_LIST, { canBack: true, canForward: true }));
      asked().mockResolvedValue(navOf(OTHER, { issue: "10042", canBack: true, canForward: true }));

      await userEvent.keyboard(keys);

      await waitFor(() => expect(useStore.getState().issueView?.stack).toEqual(["10042"]));
    });

    it.each([
      ["⌘[", "{Meta>}[[{/Meta}", () => mocked.navBack],
      ["⌘]", "{Meta>}]{/Meta}", () => mocked.navForward],
    ])("%s does nothing when there is nowhere to go", async (_key, keys, asked) => {
      await renderArrows(navOf(AT_LIST));

      await userEvent.keyboard(keys);

      expect(asked()).not.toHaveBeenCalled();
      expect(useStore.getState().issueView).toBeNull();
    });
  });
});

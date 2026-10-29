import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { installCommands, useCommandKeys } from ".";

const mocked = vi.mocked(api, true);

function Keys() {
  useCommandKeys();
  return (
    <>
      <input className="input settings-nav-search" aria-label="Search Settings" />
      <input aria-label="Field" />
      <button type="button">Other</button>
    </>
  );
}

async function renderKeys() {
  render(<Keys />);
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("keybindings_changed", expect.any(Function)));
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  installCommands("main");
});

describe("Settings window keys", () => {
  describe("in the Settings window", () => {
    beforeEach(() => {
      installCommands("settings");
    });

    it("closes the window on Escape", async () => {
      await renderKeys();

      await userEvent.keyboard("{Escape}");

      expect(mocked.runCommand).toHaveBeenCalledWith("window:close");
    });

    it("keeps the window open on Escape while a text field has focus", async () => {
      await renderKeys();
      await userEvent.click(screen.getByRole("textbox", { name: "Field" }));

      await userEvent.keyboard("{Escape}");

      expect(mocked.runCommand).not.toHaveBeenCalled();
    });

    it("keeps the window open on Escape while a dialog is open", async () => {
      await renderKeys();
      const dialog = document.createElement("div");
      dialog.setAttribute("role", "dialog");
      document.body.append(dialog);

      await userEvent.keyboard("{Escape}");
      dialog.remove();

      expect(mocked.runCommand).not.toHaveBeenCalled();
    });

    it("focuses the search field on ⌘F", async () => {
      await renderKeys();

      await userEvent.keyboard("{Meta>}f{/Meta}");

      expect(screen.getByRole("textbox", { name: "Search Settings" })).toHaveFocus();
    });
  });

  describe("in the Taskboard window", () => {
    beforeEach(() => {
      installCommands("main");
    });

    it("leaves Escape alone", async () => {
      await renderKeys();

      await userEvent.keyboard("{Escape}");

      expect(mocked.runCommand).not.toHaveBeenCalled();
    });

    it("leaves ⌘F alone", async () => {
      await renderKeys();

      await userEvent.keyboard("{Meta>}f{/Meta}");

      expect(screen.getByRole("textbox", { name: "Search Settings" })).not.toHaveFocus();
    });
  });
});

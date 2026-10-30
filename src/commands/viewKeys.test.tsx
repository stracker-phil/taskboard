import { render, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { installCommands, useCommandKeys } from ".";
import { allCommands } from "./registry";
import { onSelectView } from "./viewKeys";

const mocked = vi.mocked(api, true);

function Keys() {
  useCommandKeys();
  return null;
}

async function renderKeys() {
  render(<Keys />);
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("keybindings_changed", expect.any(Function)));
}

beforeEach(() => {
  vi.clearAllMocks();
  installCommands("main");
});

describe("page commands", () => {
  it("registers view.select.1 to view.select.9 on ⌘1 to ⌘9", () => {
    const select = allCommands().filter((c) => c.id.startsWith("view.select."));

    expect(select.map((c) => [c.id, c.defaultBinding])).toEqual(
      Array.from({ length: 9 }, (_, i) => [`view.select.${i + 1}`, `Mod+${i + 1}`]),
    );
  });

  describe("Space keys", () => {
    it("runs nav:previousSpace on ⌥⌘↑", async () => {
      await renderKeys();

      await userEvent.keyboard("{Meta>}{Alt>}{ArrowUp}{/Alt}{/Meta}");

      expect(mocked.runCommand).toHaveBeenCalledWith("nav:previousSpace");
    });

    it("runs nav:nextSpace on ⌥⌘↓", async () => {
      await renderKeys();

      await userEvent.keyboard("{Meta>}{Alt>}{ArrowDown}{/Alt}{/Meta}");

      expect(mocked.runCommand).toHaveBeenCalledWith("nav:nextSpace");
    });
  });

  describe("list keys", () => {
    it("binds list.previous to ⌥⌘[ and list.next to ⌥⌘]", () => {
      const lists = allCommands().filter((c) => c.id === "list.previous" || c.id === "list.next");

      expect(lists.map((c) => [c.id, c.defaultBinding, c.bus])).toEqual([
        ["list.previous", "Mod+Alt+[", "nav:previousList"],
        ["list.next", "Mod+Alt+]", "nav:nextList"],
      ]);
    });

    it("runs nav:previousList on ⌥⌘[", async () => {
      await renderKeys();

      await userEvent.keyboard("{Meta>}{Alt>}[BracketLeft]{/Alt}{/Meta}");

      expect(mocked.runCommand).toHaveBeenCalledWith("nav:previousList");
    });

    it("runs nav:nextList on ⌥⌘]", async () => {
      await renderKeys();

      await userEvent.keyboard("{Meta>}{Alt>}[BracketRight]{/Alt}{/Meta}");

      expect(mocked.runCommand).toHaveBeenCalledWith("nav:nextList");
    });
  });

  describe("view keys", () => {
    it.each([1, 2, 9])("passes the zero-based index of ⌘%i to the shown tabs", async (n) => {
      const handler = vi.fn();
      const release = onSelectView(handler);
      await renderKeys();

      await userEvent.keyboard(`{Meta>}${n}{/Meta}`);

      expect(handler).toHaveBeenCalledWith(n - 1);
      release();
    });

    it("ignores ⌘1 while no tabs are shown", async () => {
      const handler = vi.fn();
      onSelectView(handler)();
      await renderKeys();

      await userEvent.keyboard("{Meta>}1{/Meta}");

      expect(handler).not.toHaveBeenCalled();
    });

    it("keeps the newer tabs' handler when an older one is released late", async () => {
      const older = vi.fn();
      const newer = vi.fn();
      const releaseOlder = onSelectView(older);
      const releaseNewer = onSelectView(newer);
      releaseOlder();
      await renderKeys();

      await userEvent.keyboard("{Meta>}1{/Meta}");

      expect(newer).toHaveBeenCalledWith(0);
      expect(older).not.toHaveBeenCalled();
      releaseNewer();
    });
  });
});

import { render, renderHook, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { installCommands, useCommandKeys } from ".";
import { PAGE_ACTION_COMMANDS, runPageAction, usePageAction } from "./pageActions";
import { allCommands, bindingFor, conflicts, getCommand, setOverrides } from "./registry";

const mocked = vi.mocked(api, true);

function Keys() {
  useCommandKeys();
  return null;
}

beforeEach(() => {
  vi.clearAllMocks();
  installCommands("main");
  setOverrides({});
});

describe("usePageAction", () => {
  it("runs the action when its id is run", () => {
    const run = vi.fn();
    renderHook(() => usePageAction("test.action", run));

    expect(runPageAction("test.action")).toBe(true);

    expect(run).toHaveBeenCalled();
  });

  it("is not offered while nothing registers the id", () => {
    expect(runPageAction("test.action")).toBe(false);
  });

  it("is not offered while the action is null, and is once it is a function", () => {
    const run = vi.fn();
    const { rerender } = renderHook(({ action }) => usePageAction("test.action", action), { initialProps: { action: null as (() => void) | null } });
    expect(runPageAction("test.action")).toBe(false);

    rerender({ action: run });
    expect(runPageAction("test.action")).toBe(true);
    expect(run).toHaveBeenCalled();

    rerender({ action: null });
    expect(runPageAction("test.action")).toBe(false);
  });

  it("runs the latest action after a re-render", () => {
    const first = vi.fn();
    const second = vi.fn();
    const { rerender } = renderHook(({ action }) => usePageAction("test.action", action), { initialProps: { action: first } });

    rerender({ action: second });
    runPageAction("test.action");

    expect(second).toHaveBeenCalled();
    expect(first).not.toHaveBeenCalled();
  });

  it("is gone after the component unmounts", () => {
    const { unmount } = renderHook(() => usePageAction("test.action", () => {}));

    unmount();

    expect(runPageAction("test.action")).toBe(false);
  });

  describe("two components offering the same id", () => {
    it("runs the one that registered last", () => {
      const older = vi.fn();
      const newer = vi.fn();
      renderHook(() => usePageAction("test.action", older));
      renderHook(() => usePageAction("test.action", newer));

      runPageAction("test.action");

      expect(newer).toHaveBeenCalled();
      expect(older).not.toHaveBeenCalled();
    });

    it("keeps the newer action when the older component unmounts", () => {
      const newer = vi.fn();
      const older = renderHook(() => usePageAction("test.action", () => {}));
      renderHook(() => usePageAction("test.action", newer));

      older.unmount();
      runPageAction("test.action");

      expect(newer).toHaveBeenCalled();
    });
  });
});

describe("page action commands", () => {
  it.each(PAGE_ACTION_COMMANDS.map((c) => [c.id, c] as const))("%s is not offered while nothing on screen registers it", (_id, command) => {
    expect(command.when?.()).toBe(false);
  });

  it("is offered while a component registers its id, and runs that component's action", () => {
    const run = vi.fn();
    const { unmount } = renderHook(() => usePageAction("toolbar.me", run));
    const command = getCommand("toolbar.me")!;

    expect(command.when?.()).toBe(true);
    void command.run();
    expect(run).toHaveBeenCalled();

    unmount();
    expect(command.when?.()).toBe(false);
  });

  it("runs the registered action when its default key is pressed", async () => {
    const run = vi.fn();
    renderHook(() => usePageAction("toolbar.me", run));
    render(<Keys />);
    await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("keybindings_changed", expect.any(Function)));

    await userEvent.keyboard("{Meta>}{Shift>}m{/Shift}{/Meta}");

    expect(run).toHaveBeenCalled();
  });

});

describe("page action titles", () => {
  it.each([
    ["go.recent", "Go to Unread"],
    ["go.pinned", "Go to Bookmarks"],
    ["recent.markAllSeen", "Mark All as Read"],
  ])("name %s %s", (id, title) => {
    expect(getCommand(id)?.title).toBe(title);
  });
});

describe("default bindings of the main window", () => {
  it("parse for every command that has one", () => {
    const unparsable = allCommands().filter((c) => c.defaultBinding && bindingFor(c.id) === null);

    expect(unparsable.map((c) => c.id)).toEqual([]);
  });

  it("are unique per scope, except Escape, which the peek and the Issue view share and never offer together", () => {
    expect(conflicts().map((c) => [c.binding, c.a.id, c.b.id])).toEqual([["Escape", "peek.close", "issue.back"]]);
  });

  it("include the page actions, the issue keys, the view keys and the palette", () => {
    const ids = allCommands().map((c) => c.id);

    expect(ids).toEqual(expect.arrayContaining(["palette.open", "view.select.1", "issue.markSeen", "issue.togglePeek", ...PAGE_ACTION_COMMANDS.map((c) => c.id)]));
  });
});

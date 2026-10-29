import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { installCommands } from "../commands";
import { usePageAction } from "../commands/pageActions";
import { clearCommands, registerCommands } from "../commands/registry";
import { emitEvent } from "../test/apiMock";
import { narrowingKey } from "./slices/narrowing";
import { resetStore, useStore } from "./store";
import { useAppEvents } from "./useAppEvents";

const mocked = vi.mocked(api, true);

async function mountEvents() {
  const view = renderHook(() => useAppEvents());
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("page_command", expect.any(Function)));
  return view;
}

const pageCommand = (id: string) => act(() => emitEvent(api, "page_command", id));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  clearCommands();
});

const origin = { listId: 3, viewId: 7 };
const key = narrowingKey(origin.listId, origin.viewId);

describe("useAppEvents launch events", () => {
  it("selects the issue in its view with the peek closed and opens the Issue view over that origin", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "openIssue", id: "DEMO-12", origin }]);

    await mountEvents();

    await waitFor(() => expect(useStore.getState().issueView).toEqual({ origin, stack: ["DEMO-12"] }));
    expect(useStore.getState().places[key]).toMatchObject({ selectedId: "DEMO-12", peekOpen: false });
  });

  it("takes launch events again when Rust signals a launch", async () => {
    await mountEvents();
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "openIssue", id: "DEMO-5", origin }]);

    act(() => emitEvent(api, "launch", null));

    await waitFor(() => expect(useStore.getState().issueView?.stack).toEqual(["DEMO-5"]));
  });

  describe("onlyMine", () => {
    beforeEach(() => {
      mocked.navCurrent.mockResolvedValue({ location: origin, expanded: [] });
    });

    it.each([
      { on: true, before: false, after: true },
      { on: false, before: true, after: false },
      { on: true, before: true, after: true },
    ])("sets the current view's Me chip to $after when on is $on and it was $before", async ({ on, before, after }) => {
      useStore.getState().setNarrowing(key, { me: before });
      mocked.takeLaunchEvents.mockResolvedValue([{ type: "onlyMine", on }]);

      await mountEvents();

      await waitFor(() => expect(useStore.getState().narrowings[key].me).toBe(after));
    });

    it.each([
      { before: false, after: true },
      { before: true, after: false },
    ])("flips the Me chip from $before to $after when on is null", async ({ before, after }) => {
      useStore.getState().setNarrowing(key, { me: before });
      mocked.takeLaunchEvents.mockResolvedValue([{ type: "onlyMine", on: null }]);

      await mountEvents();

      await waitFor(() => expect(useStore.getState().narrowings[key].me).toBe(after));
    });

    it("turns the Me chip on when on is null and the view has no narrowing yet", async () => {
      mocked.takeLaunchEvents.mockResolvedValue([{ type: "onlyMine", on: null }]);

      await mountEvents();

      await waitFor(() => expect(useStore.getState().narrowings[key].me).toBe(true));
    });

    it("changes nothing when navigation has no location", async () => {
      mocked.navCurrent.mockResolvedValue({ location: null, expanded: [] });
      mocked.takeLaunchEvents.mockResolvedValue([{ type: "onlyMine", on: true }]);

      await mountEvents();
      await waitFor(() => expect(mocked.navCurrent).toHaveBeenCalled());
      await act(async () => {});

      expect(useStore.getState().narrowings).toEqual({});
    });
  });

  it("shows an error event as a toast", async () => {
    mocked.takeLaunchEvents.mockResolvedValue([{ type: "error", message: "No such issue DEMO-404" }]);

    await mountEvents();

    await waitFor(() => expect(useStore.getState().toasts).toMatchObject([{ kind: "error", message: "No such issue DEMO-404" }]));
  });
});

describe("useAppEvents page_command", () => {
  it("runs the command with that id", async () => {
    const run = vi.fn();
    registerCommands([{ id: "test.menu", title: "Menu item", scope: "main", run }]);
    await mountEvents();

    pageCommand("test.menu");

    expect(run).toHaveBeenCalled();
  });

  it("runs the command while its condition allows it", async () => {
    const run = vi.fn();
    registerCommands([{ id: "test.menu", title: "Menu item", scope: "main", when: () => true, run }]);
    await mountEvents();

    pageCommand("test.menu");

    expect(run).toHaveBeenCalled();
  });

  it("skips the command while its condition does not allow it", async () => {
    const run = vi.fn();
    registerCommands([{ id: "test.menu", title: "Menu item", scope: "main", when: () => false, run }]);
    await mountEvents();

    pageCommand("test.menu");

    expect(run).not.toHaveBeenCalled();
  });

  it("ignores an id no command has", async () => {
    await mountEvents();

    expect(() => pageCommand("test.unknown")).not.toThrow();
  });

  it("runs a page action only while a component on screen offers it", async () => {
    installCommands("main");
    const run = vi.fn();
    await mountEvents();

    pageCommand("view.next");
    expect(run).not.toHaveBeenCalled();

    const offered = renderHook(() => usePageAction("view.next", run));
    pageCommand("view.next");
    expect(run).toHaveBeenCalled();

    offered.unmount();
    run.mockClear();
    pageCommand("view.next");
    expect(run).not.toHaveBeenCalled();
  });

  it("stops listening when the window's hook unmounts", async () => {
    const run = vi.fn();
    registerCommands([{ id: "test.menu", title: "Menu item", scope: "main", run }]);
    const view = await mountEvents();

    view.unmount();
    await act(async () => {});
    pageCommand("test.menu");

    expect(run).not.toHaveBeenCalled();
  });
});

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { installCommands } from "../commands";
import { usePageAction } from "../commands/pageActions";
import { clearCommands, registerCommands } from "../commands/registry";
import { emitEvent } from "../test/apiMock";
import { resetStore } from "./store";
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

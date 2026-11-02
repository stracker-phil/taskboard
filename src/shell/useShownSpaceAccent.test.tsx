import { QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type ListNode, type SidebarTree } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { nav } from "../test/nav";
import { lumenSpace, sidebarTree, space } from "../test/spaces";
import { useShownSpaceAccent } from "./useShownSpaceAccent";

const mocked = vi.mocked(api, true);

const filterList = (id: number): ListNode => ({ id, kind: "list", name: "Open bugs", count: null, ownConditions: 1, description: "", error: null, icon: null, color: null });

/** Mounts the hook on a window showing `listId`. */
function shownAccent(tree: SidebarTree, listId: number | null) {
  mocked.sidebarTree.mockResolvedValue(tree);
  mocked.navCurrent.mockResolvedValue(nav(listId === null ? null : { listId, viewId: 1 }));
  const wrapper = ({ children }: { children: ReactNode }) => createElement(QueryClientProvider, { client: createQueryClient() }, children);
  const { result } = renderHook(() => useShownSpaceAccent(), { wrapper });
  return result;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("useShownSpaceAccent", () => {
  it("is the accent of the Space that is shown", async () => {
    const result = shownAccent(sidebarTree([space({ accent: "teal" })]), 11);

    await waitFor(() => expect(result.current).toBe("teal"));
  });

  it("is the accent of the Space a shown filter list belongs to", async () => {
    const result = shownAccent(sidebarTree([space({ accent: "purple", lists: [filterList(31)] })]), 31);

    await waitFor(() => expect(result.current).toBe("purple"));
  });

  it("is the accent of the shown Space, not another one's", async () => {
    const tree = sidebarTree([space({ accent: "teal" }), lumenSpace({ accent: "red" })]);

    const result = shownAccent(tree, 12);

    await waitFor(() => expect(result.current).toBe("red"));
  });

  it("is null for a Space without its own accent", async () => {
    const tree = sidebarTree([space({ accent: null })]);
    const result = shownAccent(tree, 11);

    await waitFor(() => expect(mocked.navCurrent).toHaveBeenCalled());
    expect(result.current).toBeNull();
  });

  it("is null for Everything, which belongs to no Space", async () => {
    const tree = sidebarTree([space({ accent: "teal" })]);

    const result = shownAccent(tree, tree.everything.id);

    await waitFor(() => expect(mocked.navCurrent).toHaveBeenCalled());
    expect(result.current).toBeNull();
  });

  it("is null while nothing is shown", async () => {
    const result = shownAccent(sidebarTree([space({ accent: "teal" })]), null);

    await waitFor(() => expect(mocked.navCurrent).toHaveBeenCalled());
    expect(result.current).toBeNull();
  });
});

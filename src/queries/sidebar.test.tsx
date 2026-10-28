import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { resetStore, useStore } from "../state/store";
import { lumenSpace, sidebarTree, space } from "../test/spaces";
import { view } from "../test/views";
import { createQueryClient } from "./queryClient";
import { useNav, useNavGo, useSetExpanded, useSidebar, useSpaces, useViews } from "./sidebar";

const mocked = vi.mocked(api, true);

function Probe({ listId }: { listId: number | null }) {
  const { data: nav } = useNav();
  const { data: tree } = useSidebar();
  const { data: spaces } = useSpaces();
  const { data: views } = useViews(listId);
  const open = useNavGo();
  const setExpanded = useSetExpanded();
  return (
    <>
      <p>Place: {nav?.location ? `${nav.location.listId}/${nav.location.viewId}` : "none"}</p>
      <p>Expanded: {nav?.expanded.join(",") ?? "unknown"}</p>
      <p>Everything: {tree?.everything.name ?? "loading"}</p>
      <p>Spaces: {spaces?.map((s) => s.name).join(", ") ?? "loading"}</p>
      <p>Views: {views?.map((v) => v.name).join(", ") ?? "none"}</p>
      <button onClick={() => void open(12, 5)}>Open</button>
      <button onClick={() => void setExpanded([11, 12])}>Expand</button>
    </>
  );
}

function renderProbe(listId: number | null = null) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Probe listId={listId} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space(), lumenSpace()]));
  mocked.navCurrent.mockResolvedValue({ location: null, expanded: [] });
  mocked.navGo.mockImplementation(async (listId, viewId) => ({ location: { listId, viewId: viewId ?? 0 }, expanded: [] }));
  mocked.navSetExpanded.mockImplementation(async (expanded) => ({ location: null, expanded }));
  mocked.viewsOf.mockResolvedValue([]);
});

describe("sidebar queries", () => {
  it("loads the tree, its Spaces in order and Rust's current place", async () => {
    mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [11] });

    renderProbe();

    expect(await screen.findByText("Spaces: Website relaunch, Lumen iOS app")).toBeInTheDocument();
    expect(screen.getByText("Everything: Everything")).toBeInTheDocument();
    expect(await screen.findByText("Place: 11/101")).toBeInTheDocument();
    expect(screen.getByText("Expanded: 11")).toBeInTheDocument();
  });

  describe("views", () => {
    it("loads the views of a level", async () => {
      mocked.viewsOf.mockResolvedValue([view({ name: "List" }), view({ id: 102, name: "Sprint board", layout: "board" })]);

      renderProbe(11);

      expect(await screen.findByText("Views: List, Sprint board")).toBeInTheDocument();
      expect(mocked.viewsOf).toHaveBeenCalledWith(11);
    });

    it("asks for nothing while there is no level", async () => {
      renderProbe(null);
      await screen.findByText("Spaces: Website relaunch, Lumen iOS app");

      expect(screen.getByText("Views: none")).toBeInTheDocument();
      expect(mocked.viewsOf).not.toHaveBeenCalled();
    });
  });

  describe("opening a level", () => {
    it("shows the place Rust answers with", async () => {
      renderProbe();
      await screen.findByText("Place: none");

      await userEvent.click(screen.getByRole("button", { name: "Open" }));

      expect(mocked.navGo).toHaveBeenCalledWith(12, 5);
      expect(await screen.findByText("Place: 12/5")).toBeInTheDocument();
    });

    it("keeps the place and shows an error toast when Rust refuses", async () => {
      mocked.navGo.mockRejectedValue({ kind: "notFound", message: "No such list", code: null });
      renderProbe();
      await screen.findByText("Place: none");

      await userEvent.click(screen.getByRole("button", { name: "Open" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not open it: No such list"));
      expect(screen.getByText("Place: none")).toBeInTheDocument();
    });
  });

  describe("expanding Spaces", () => {
    it("replaces the expanded Spaces with what Rust answers", async () => {
      mocked.navSetExpanded.mockResolvedValue({ location: null, expanded: [12] });
      renderProbe();
      await screen.findByText("Expanded:");

      await userEvent.click(screen.getByRole("button", { name: "Expand" }));

      expect(mocked.navSetExpanded).toHaveBeenCalledWith([11, 12]);
      expect(await screen.findByText("Expanded: 12")).toBeInTheDocument();
    });

    it("shows an error toast when the sidebar cannot be saved", async () => {
      mocked.navSetExpanded.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderProbe();
      await screen.findByText("Expanded:");

      await userEvent.click(screen.getByRole("button", { name: "Expand" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not save the sidebar: Disk full"));
    });
  });
});

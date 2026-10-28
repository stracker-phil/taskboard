import { act, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Nav } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { NAV_KEY } from "../queries/sidebar";
import { resetStore, useStore } from "../state/store";
import { issueDetail, related, viewData } from "../test/issues";
import { sidebarTree, space } from "../test/spaces";
import { listNode, view } from "../test/views";
import { Window } from "./Window";

const mocked = vi.mocked(api, true);

const nav = (listId: number, viewId: number): Nav => ({ location: { listId, viewId }, expanded: [] });

let client: QueryClient;

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  client = createQueryClient();
  mocked.navCurrent.mockResolvedValue(nav(11, 101));
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space({ lists: [listNode({ id: 21, name: "My open issues" })] })]));
  mocked.viewsOf.mockResolvedValue([view()]);
  mocked.issueGet.mockResolvedValue(issueDetail({ id: "10042" }));
  mocked.issueRelated.mockResolvedValue(related());
  mocked.viewRows.mockResolvedValue(viewData([], []));
});

async function renderWindow(open: boolean) {
  resetStore(open ? { issueView: { origin: { listId: 11, viewId: 101 }, stack: ["10042"] } } : {});
  render(
    <QueryClientProvider client={client}>
      <Window sidebarHidden>
        <p>View body</p>
      </Window>
    </QueryClientProvider>,
  );
  await vi.waitFor(() => expect(client.getQueryData(NAV_KEY)).toEqual(nav(11, 101)));
}

const goTo = (location: Nav) => act(async () => void client.setQueryData(NAV_KEY, location));

describe("Window", () => {
  describe("with the Issue view open", () => {
    it("keeps it while the place stays on the view it was opened from", async () => {
      await renderWindow(true);

      expect(await screen.findByRole("button", { name: /^Back to / })).toBeInTheDocument();
      expect(screen.queryByText("View body")).not.toBeInTheDocument();
      expect(useStore.getState().issueView).not.toBeNull();
    });

    it("keeps it when the same place is announced again", async () => {
      await renderWindow(true);
      await screen.findByRole("button", { name: /^Back to / });

      await goTo(nav(11, 101));

      expect(screen.getByRole("button", { name: /^Back to / })).toBeInTheDocument();
    });

    it.each([
      ["another list", nav(21, 201)],
      ["another view of the same list", nav(11, 102)],
    ])("closes it and shows the view body when the place moves to %s", async (_name, target) => {
      await renderWindow(true);
      await screen.findByRole("button", { name: /^Back to / });

      await goTo(target);

      expect(await screen.findByText("View body")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^Back to / })).not.toBeInTheDocument();
      expect(useStore.getState().issueView).toBeNull();
    });
  });

  describe("level chrome", () => {
    const tree = () => ({ ...sidebarTree([space()]), home: [listNode({ id: 3, kind: "pinned", name: "Pinned", count: null })] });

    it("shows the view tabs on a Space", async () => {
      mocked.sidebarTree.mockResolvedValue(tree());
      await renderWindow(false);

      expect(await screen.findByRole("tablist", { name: "Views" })).toBeInTheDocument();
    });

    it("shows no view tabs and no toolbar on the Pinned level", async () => {
      mocked.sidebarTree.mockResolvedValue(tree());
      mocked.navCurrent.mockResolvedValue(nav(3, 0));
      mocked.viewsOf.mockResolvedValue([view({ id: 0, listId: 3 })]);
      render(
        <QueryClientProvider client={client}>
          <Window sidebarHidden>
            <p>View body</p>
          </Window>
        </QueryClientProvider>,
      );

      expect(await screen.findByRole("heading", { name: "Pinned" })).toBeInTheDocument();
      expect(screen.queryByRole("tablist", { name: "Views" })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^Filter/ })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "More toolbar items" })).not.toBeInTheDocument();
    });
  });

  describe("without an Issue view", () => {
    it("shows the view body and stays that way when the place changes", async () => {
      await renderWindow(false);
      expect(await screen.findByText("View body")).toBeInTheDocument();

      await goTo(nav(21, 201));

      expect(screen.getByText("View body")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^Back to / })).not.toBeInTheDocument();
    });
  });
});

import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { narrowingKey } from "../state/slices/narrowing";
import { resetStore, useStore } from "../state/store";
import { nav as navAt, tab, tabbedNav } from "../test/nav";
import { lumenSpace, sidebarTree, space } from "../test/spaces";
import { view } from "../test/views";
import { createQueryClient } from "./queryClient";
import { useNav, useNavGo, useReportShownIssue, useSetExpanded, useSidebar, useSpaces, useTabs, useTravel, useViews } from "./sidebar";

const mocked = vi.mocked(api, true);

function Probe({ listId }: { listId: number | null }) {
  const { data: nav } = useNav();
  const { data: tree } = useSidebar();
  const { data: spaces } = useSpaces();
  const { data: views } = useViews(listId);
  const open = useNavGo();
  const setExpanded = useSetExpanded();
  const tabs = useTabs();
  return (
    <>
      <p>Place: {nav?.location ? `${nav.location.listId}/${nav.location.viewId}` : "none"}</p>
      <p>Tabs: {nav ? `${nav.tabs.map((t) => t.id).join(",")} (showing ${nav.activeTab})` : "unknown"}</p>
      <button onClick={() => void tabs.open(12, 5, true)}>Open tab</button>
      <button onClick={() => void tabs.activate(1)}>Show tab 1</button>
      <button onClick={() => void tabs.close(2)}>Close tab 2</button>
      <button onClick={() => void tabs.move(1, null)}>Move tab 1 last</button>
      <p>Expanded: {nav?.expanded.join(",") ?? "unknown"}</p>
      <p>Everything: {tree?.everything.name ?? "loading"}</p>
      <p>Spaces: {spaces?.map((s) => s.name).join(", ") ?? "loading"}</p>
      <p>Views: {views?.map((v) => v.name).join(", ") ?? "none"}</p>
      <button onClick={() => void open(12, 5)}>Open</button>
      <button onClick={() => void setExpanded([11, 12])}>Expand</button>
    </>
  );
}

function Traveller() {
  const travel = useTravel();
  return (
    <>
      <button onClick={() => void travel("back")}>Travel back</button>
      <button onClick={() => void travel("forward")}>Travel forward</button>
    </>
  );
}

function Reporter({ issue }: { issue: string | null }) {
  useReportShownIssue(issue);
  const { data: nav } = useNav();
  return <p>Can go back: {nav?.canBack ? "yes" : "no"}</p>;
}

function renderWith(ui: ReactNode) {
  render(<QueryClientProvider client={createQueryClient()}>{ui}</QueryClientProvider>);
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
  mocked.navCurrent.mockResolvedValue(navAt(null));
  mocked.navGo.mockImplementation(async (listId, viewId) => navAt({ listId, viewId: viewId ?? 0 }));
  mocked.navSetExpanded.mockImplementation(async (expanded) => navAt(null, { expanded }));
  mocked.navSetIssue.mockResolvedValue(navAt(null));
  mocked.viewsOf.mockResolvedValue([]);
});

describe("sidebar queries", () => {
  it("loads the tree, its Spaces in order and Rust's current place", async () => {
    mocked.navCurrent.mockResolvedValue(navAt({ listId: 11, viewId: 101 }, { expanded: [11] }));

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

  describe("opening the place already shown", () => {
    const peekOpen = (listId: number, viewId: number) => useStore.getState().places[narrowingKey(listId, viewId)]?.peekOpen;

    async function renderAt(listId: number, viewId: number) {
      mocked.navCurrent.mockResolvedValue(navAt({ listId, viewId }));
      resetStore({ issueView: { origin: { listId, viewId }, stack: ["10042"] } });
      act(() => useStore.getState().updatePlace(12, 5, { peekOpen: true }));
      renderProbe();
      await screen.findByText(`Place: ${listId}/${viewId}`);
    }

    it("closes the Issue view and the peek of that place", async () => {
      await renderAt(12, 5);

      await userEvent.click(screen.getByRole("button", { name: "Open" }));

      await waitFor(() => expect(useStore.getState().issueView).toBeNull());
      expect(peekOpen(12, 5)).toBe(false);
    });

    it("leaves the Issue view and the target's peek alone when going elsewhere", async () => {
      await renderAt(11, 101);

      await userEvent.click(screen.getByRole("button", { name: "Open" }));

      expect(await screen.findByText("Place: 12/5")).toBeInTheDocument();
      expect(useStore.getState().issueView).not.toBeNull();
      expect(peekOpen(12, 5)).toBe(true);
    });
  });

  describe("tabs", () => {
    const AT_LIST = { listId: 11, viewId: 101 };
    const TWO = tabbedNav([tab(1, AT_LIST), tab(2, { listId: 12, viewId: 5 })], 2);

    it("shows the tabs Rust answers with when one is opened", async () => {
      mocked.navCurrent.mockResolvedValue(navAt(AT_LIST));
      mocked.navTabNew.mockResolvedValue(TWO);
      renderProbe();
      await screen.findByText("Tabs: 1 (showing 1)");

      await userEvent.click(screen.getByRole("button", { name: "Open tab" }));

      expect(mocked.navTabNew).toHaveBeenCalledWith(12, 5, true);
      expect(await screen.findByText("Tabs: 1,2 (showing 2)")).toBeInTheDocument();
      expect(screen.getByText("Place: 12/5")).toBeInTheDocument();
    });

    it("shows the tab Rust answers with when one is shown", async () => {
      mocked.navCurrent.mockResolvedValue(tabbedNav(TWO.tabs, 2));
      mocked.navTabActivate.mockResolvedValue(tabbedNav(TWO.tabs, 1));
      renderProbe();
      await screen.findByText("Tabs: 1,2 (showing 2)");

      await userEvent.click(screen.getByRole("button", { name: "Show tab 1" }));

      expect(mocked.navTabActivate).toHaveBeenCalledWith(1);
      expect(await screen.findByText("Tabs: 1,2 (showing 1)")).toBeInTheDocument();
    });

    it("drops the tab Rust answers without when one is closed", async () => {
      mocked.navCurrent.mockResolvedValue(TWO);
      mocked.navTabClose.mockResolvedValue(navAt(AT_LIST));
      renderProbe();
      await screen.findByText("Tabs: 1,2 (showing 2)");

      await userEvent.click(screen.getByRole("button", { name: "Close tab 2" }));

      expect(mocked.navTabClose).toHaveBeenCalledWith(2);
      expect(await screen.findByText("Tabs: 1 (showing 1)")).toBeInTheDocument();
    });

    it("shows the order Rust answers with when one is moved", async () => {
      mocked.navCurrent.mockResolvedValue(TWO);
      mocked.navTabMove.mockResolvedValue(tabbedNav([TWO.tabs[1], TWO.tabs[0]], 2));
      renderProbe();
      await screen.findByText("Tabs: 1,2 (showing 2)");

      await userEvent.click(screen.getByRole("button", { name: "Move tab 1 last" }));

      expect(mocked.navTabMove).toHaveBeenCalledWith(1, null);
      expect(await screen.findByText("Tabs: 2,1 (showing 2)")).toBeInTheDocument();
    });

    it("keeps the tabs and shows an error toast when Rust refuses", async () => {
      mocked.navCurrent.mockResolvedValue(TWO);
      mocked.navTabClose.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderProbe();
      await screen.findByText("Tabs: 1,2 (showing 2)");

      await userEvent.click(screen.getByRole("button", { name: "Close tab 2" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not close the tab: Disk full"));
      expect(screen.getByText("Tabs: 1,2 (showing 2)")).toBeInTheDocument();
    });
  });

  describe("travelling back and forward", () => {
    const AT_LIST = { listId: 11, viewId: 101 };
    const OTHER = { listId: 12, viewId: 5 };

    function renderTraveller() {
      renderWith(
        <>
          <Probe listId={null} />
          <Traveller />
        </>,
      );
    }

    it.each([
      ["back", "Travel back", mocked.navBack],
      ["forward", "Travel forward", mocked.navForward],
    ])("opens the Issue view on the issue Rust answers with when going %s", async (_direction, button, call) => {
      call.mockResolvedValue(navAt(OTHER, { issue: "10042" }));
      mocked.navCurrent.mockResolvedValue(navAt(AT_LIST));
      renderTraveller();
      await screen.findByText("Place: 11/101");

      await userEvent.click(screen.getByRole("button", { name: button }));

      expect(await screen.findByText("Place: 12/5")).toBeInTheDocument();
      expect(useStore.getState().issueView).toEqual({ origin: OTHER, stack: ["10042"] });
    });

    it.each([
      ["back", "Travel back", mocked.navBack],
      ["forward", "Travel forward", mocked.navForward],
    ])("closes the Issue view when going %s lands on a place without an issue", async (_direction, button, call) => {
      call.mockResolvedValue(navAt(AT_LIST, { issue: null }));
      mocked.navCurrent.mockResolvedValue(navAt(AT_LIST, { issue: "10042" }));
      resetStore({ issueView: { origin: AT_LIST, stack: ["10042"] } });
      renderTraveller();
      await screen.findByText("Place: 11/101");

      await userEvent.click(screen.getByRole("button", { name: button }));

      await waitFor(() => expect(useStore.getState().issueView).toBeNull());
    });

    it("keeps the Issue view and shows an error toast when Rust refuses", async () => {
      mocked.navBack.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      mocked.navCurrent.mockResolvedValue(navAt(AT_LIST));
      resetStore({ issueView: { origin: AT_LIST, stack: ["10042"] } });
      renderTraveller();
      await screen.findByText("Place: 11/101");

      await userEvent.click(screen.getByRole("button", { name: "Travel back" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not go there: Disk full"));
      expect(useStore.getState().issueView).not.toBeNull();
    });
  });

  describe("reporting the shown issue", () => {
    it("tells Rust the issue when it changes, and null when it closes", async () => {
      const client = createQueryClient();
      const { rerender } = render(
        <QueryClientProvider client={client}>
          <Reporter issue={null} />
        </QueryClientProvider>,
      );
      await waitFor(() => expect(mocked.navSetIssue).toHaveBeenLastCalledWith(null));

      rerender(
        <QueryClientProvider client={client}>
          <Reporter issue="10042" />
        </QueryClientProvider>,
      );
      await waitFor(() => expect(mocked.navSetIssue).toHaveBeenLastCalledWith("10042"));

      rerender(
        <QueryClientProvider client={client}>
          <Reporter issue={null} />
        </QueryClientProvider>,
      );
      await waitFor(() => expect(mocked.navSetIssue).toHaveBeenLastCalledWith(null));
    });

    it("asks for the place again afterwards, so the arrows follow Rust's history", async () => {
      let reported = false;
      mocked.navSetIssue.mockImplementation(async () => {
        reported = true;
        return navAt(null);
      });
      mocked.navCurrent.mockImplementation(async () => navAt(null, { canBack: reported }));

      renderWith(<Reporter issue="10042" />);

      expect(await screen.findByText("Can go back: yes")).toBeInTheDocument();
    });

    it("stays quiet when Rust refuses the report", async () => {
      mocked.navSetIssue.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      renderWith(<Reporter issue="10042" />);
      await waitFor(() => expect(mocked.navSetIssue).toHaveBeenCalledWith("10042"));

      expect(useStore.getState().toasts).toEqual([]);
    });
  });

  describe("expanding Spaces", () => {
    it("replaces the expanded Spaces with what Rust answers", async () => {
      mocked.navSetExpanded.mockResolvedValue(navAt(null, { expanded: [12] }));
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

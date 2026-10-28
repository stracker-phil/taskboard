import { act, render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { ConnectionPage } from "../components/settings/jira/ConnectionPage";
import { ProjectsPage } from "../components/settings/jira/ProjectsPage";
import { SyncPage } from "../components/settings/jira/SyncPage";
import { emitEvent } from "../test/apiMock";
import { connectionView, PROJECTS, projectCounts, syncOverview } from "../test/jira";
import { row, statusStyle, viewData } from "../test/issues";
import { settings } from "../test/settings";
import { lumenSpace, sidebarTree, space } from "../test/spaces";
import { createQueryClient } from "./queryClient";
import { useNav, useSpaces } from "./sidebar";
import { useEvents } from "./useEvents";
import { useStatusStyles, useViewRows } from "./views";

const mocked = vi.mocked(api, true);

function Harness() {
  useEvents();
  return (
    <>
      <ConnectionPage />
      <ProjectsPage />
      <SyncPage />
    </>
  );
}

async function renderHarness() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness />
    </QueryClientProvider>,
  );
  await screen.findByText("Refresh to load your projects.");
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("local_changed", expect.any(Function)));
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.connectionGet.mockResolvedValue(connectionView());
  mocked.projectsList.mockResolvedValue([]);
  mocked.scopeGet.mockResolvedValue({ jql: "", isDefault: true });
  mocked.syncOverview.mockResolvedValue(syncOverview());
});

describe("useEvents", () => {
  it("refetches the projects when local_changed arrives", async () => {
    await renderHarness();
    mocked.projectsList.mockResolvedValue(PROJECTS);

    act(() => emitEvent(api, "local_changed", { parts: [] }));

    expect(await screen.findByText("Website relaunch")).toBeInTheDocument();
  });

  it("refetches the projects and the scope when issues_changed arrives", async () => {
    await renderHarness();
    mocked.projectsList.mockResolvedValue(PROJECTS);
    mocked.scopeGet.mockResolvedValue({ jql: "project = NW", isDefault: false });

    act(() => emitEvent(api, "issues_changed", { ids: [], all: true }));

    expect(await screen.findByText("Website relaunch")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Query" })).toHaveValue("project = NW"));
  });

  it("refetches the sync overview when sync_state arrives", async () => {
    await renderHarness();
    await screen.findByText("Not synced yet");
    mocked.syncOverview.mockResolvedValue(syncOverview({ state: { state: "running", job: "full" } }));

    act(() => emitEvent(api, "sync_state", { state: "running", job: "full" }));

    expect(await screen.findByText("Syncing…")).toBeInTheDocument();
  });

  it("refetches the sync overview when issues_changed arrives", async () => {
    await renderHarness();
    await screen.findByText("Not synced yet");
    mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));

    act(() => emitEvent(api, "issues_changed", { ids: [], all: true }));

    expect(await screen.findByText("33 issues · 21 open")).toBeInTheDocument();
  });

  it("refetches the connection when settings_changed arrives", async () => {
    await renderHarness();
    mocked.connectionGet.mockResolvedValue(connectionView({ email: "sam.lee@example.com" }));

    act(() =>
      emitEvent(api, "settings_changed", settings({ connection: connectionView().connection })),
    );

    await waitFor(() => expect(screen.getByLabelText("Email")).toHaveValue("sam.lee@example.com"));
  });

  describe("view rows and status styles", () => {
    function RowsHarness() {
      useEvents();
      const { data: view } = useViewRows(11, 101);
      const { data: styles } = useStatusStyles();
      return (
        <>
          <p>Rows: {view ? Object.values(view.rows).map((r) => r.key).join(", ") || "none" : "loading"}</p>
          <p>Styles: {styles?.map((s) => `${s.name}=${s.chosen ?? "auto"}`).join(", ") || "none"}</p>
        </>
      );
    }

    async function renderRows() {
      mocked.viewRows.mockResolvedValue(viewData([], [row({ key: "NW-1" })]));
      mocked.statusStylesList.mockResolvedValue([]);
      render(
        <QueryClientProvider client={createQueryClient()}>
          <RowsHarness />
        </QueryClientProvider>,
      );
      await screen.findByText("Rows: NW-1");
      await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("issues_changed", expect.any(Function)));
    }

    it.each([
      ["local_changed", { parts: ["views"] }],
      ["issues_changed", { ids: ["1"], all: false }],
    ] as const)("refetches the view rows when %s arrives", async (event, payload) => {
      await renderRows();
      mocked.viewRows.mockResolvedValue(viewData([], [row({ key: "NW-1" }), row({ id: "10043", key: "NW-2" })]));

      act(() => emitEvent(api, event, payload as never));

      expect(await screen.findByText("Rows: NW-1, NW-2")).toBeInTheDocument();
    });

    it.each([
      ["local_changed", { parts: ["statuses"] }],
      ["issues_changed", { ids: [], all: true }],
    ] as const)("refetches the status styles when %s arrives", async (event, payload) => {
      await renderRows();
      await screen.findByText("Styles: none");
      mocked.statusStylesList.mockResolvedValue([statusStyle({ chosen: "blocked" })]);

      act(() => emitEvent(api, event, payload as never));

      expect(await screen.findByText("Styles: In Progress=blocked")).toBeInTheDocument();
    });
  });

  describe("sidebar and place", () => {
    function PlaceHarness() {
      useEvents();
      const { data: nav } = useNav();
      const { data: spaces } = useSpaces();
      return (
        <>
          <p>Place: {nav?.location ? `${nav.location.listId}/${nav.location.viewId}` : "none"}</p>
          <p>Spaces: {spaces?.map((s) => s.name).join(", ")}</p>
        </>
      );
    }

    async function renderPlace() {
      mocked.sidebarTree.mockResolvedValue(sidebarTree([space()]));
      mocked.navCurrent.mockResolvedValue({ location: null, expanded: [] });
      render(
        <QueryClientProvider client={createQueryClient()}>
          <PlaceHarness />
        </QueryClientProvider>,
      );
      await screen.findByText("Spaces: Website relaunch");
      await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("nav_changed", expect.any(Function)));
    }

    it.each([
      ["local_changed", { parts: ["spaces"] }],
      ["issues_changed", { ids: [], all: true }],
    ] as const)("refetches the Spaces when %s arrives", async (event, payload) => {
      await renderPlace();
      mocked.sidebarTree.mockResolvedValue(sidebarTree([space(), lumenSpace()]));

      act(() => emitEvent(api, event, payload as never));

      expect(await screen.findByText("Spaces: Website relaunch, Lumen iOS app")).toBeInTheDocument();
    });

    it("shows the place nav_changed carries without asking Rust again", async () => {
      await renderPlace();
      await screen.findByText("Place: none");

      act(() => emitEvent(api, "nav_changed", { location: { listId: 12, viewId: 7 }, expanded: [12] }));

      expect(await screen.findByText("Place: 12/7")).toBeInTheDocument();
    });

    it("asks Rust for the place again when local_changed arrives", async () => {
      await renderPlace();
      await screen.findByText("Place: none");
      mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 3 }, expanded: [] });

      act(() => emitEvent(api, "local_changed", { parts: ["views"] }));

      expect(await screen.findByText("Place: 11/3")).toBeInTheDocument();
    });
  });
});

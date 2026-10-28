import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type SyncProgress, type SyncState } from "../api";
import { formatLocalTime } from "../lib/time";
import { createQueryClient } from "../queries/queryClient";
import { useEvents } from "../queries/useEvents";
import { resetStore } from "../state/store";
import { emitEvent } from "../test/apiMock";
import { projectCounts, syncOverview } from "../test/jira";
import { SyncStatusView } from "./SyncStatusView";

const mocked = vi.mocked(api, true);

const NW = projectCounts();
const LUM = projectCounts({ id: "10002", key: "LUM", name: "Lumen iOS app", cached: 1, open: 1 });
const RUNNING: SyncState = { state: "running", job: "full" };

const progress = (patch: Partial<SyncProgress> = {}): SyncProgress => ({
  job: "full",
  projectId: NW.id,
  projectKey: NW.key,
  pagesDone: 1,
  fetched: 120,
  approxTotal: 400,
  ...patch,
});

function Harness() {
  useEvents();
  return <SyncStatusView />;
}

async function renderView() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Harness />
    </QueryClientProvider>,
  );
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("sync_progress", expect.any(Function)));
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.syncOverview.mockResolvedValue(syncOverview());
});

describe("SyncStatusView", () => {
  describe("without tracked projects", () => {
    it("asks to connect Jira in Settings", async () => {
      await renderView();

      expect(await screen.findByText("Connect Jira in Settings")).toBeInTheDocument();
    });

    it("runs the app:settings command from Open Settings", async () => {
      await renderView();

      await userEvent.click(await screen.findByRole("button", { name: "Open Settings" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings");
    });
  });

  describe("while the first sync runs", () => {
    beforeEach(() => {
      mocked.syncOverview.mockResolvedValue(syncOverview({ state: RUNNING, projects: [NW, LUM] }));
    });

    const rows = () => within(screen.getByRole("list")).getAllByRole("listitem");

    it("lists each project as waiting until its first progress arrives", async () => {
      await renderView();

      expect(await screen.findByText("Loading your Jira issues")).toBeInTheDocument();
      expect(screen.getAllByText("waiting")).toHaveLength(2);
      expect(screen.getByText("0 issues so far")).toBeInTheDocument();
    });

    it("shows a running project's next page and the overall fetched of total with a progressbar", async () => {
      await renderView();
      await screen.findByText("Loading your Jira issues");

      act(() => {
        emitEvent(api, "sync_progress", progress());
        emitEvent(api, "sync_progress", progress({ projectId: LUM.id, projectKey: LUM.key, pagesDone: 0, fetched: 1, approxTotal: 1 }));
      });

      expect(await screen.findByText("About 121 of 401 issues")).toBeInTheDocument();
      expect(rows()[0]).toHaveTextContent("page 2");
      expect(rows()[1]).toHaveTextContent("page 1");
      const bar = screen.getByRole("progressbar", { name: "Issues loaded" });
      expect(bar).toHaveAttribute("aria-valuenow", "121");
      expect(bar).toHaveAttribute("aria-valuemax", "401");
    });

    it("counts the issues so far, without a bar value, while Jira gave no estimate", async () => {
      await renderView();
      await screen.findByText("Loading your Jira issues");

      act(() => emitEvent(api, "sync_progress", progress({ approxTotal: null })));

      expect(await screen.findByText("120 issues so far")).toBeInTheDocument();
      expect(screen.getByRole("progressbar", { name: "Issues loaded" })).not.toHaveAttribute("aria-valuenow");
    });

    it("counts the issues so far while a project has not started", async () => {
      await renderView();
      await screen.findByText("Loading your Jira issues");

      act(() => emitEvent(api, "sync_progress", progress()));

      expect(await screen.findByText("120 issues so far")).toBeInTheDocument();
      expect(rows()[1]).toHaveTextContent("waiting");
    });

    it("keeps the latest progress per project", async () => {
      await renderView();
      await screen.findByText("Loading your Jira issues");

      act(() => {
        emitEvent(api, "sync_progress", progress({ fetched: 100 }));
        emitEvent(api, "sync_progress", progress({ fetched: 200 }));
        emitEvent(api, "sync_progress", progress({ projectId: LUM.id, projectKey: LUM.key, fetched: 1, approxTotal: 1 }));
      });

      expect(await screen.findByText("About 201 of 401 issues")).toBeInTheDocument();
    });

    describe("with a finished Space", () => {
      const DONE = projectCounts({ lastFullAt: "2026-10-27T09:00:00Z" });

      beforeEach(() => {
        mocked.syncOverview.mockResolvedValue(syncOverview({ state: RUNNING, projects: [DONE, LUM] }));
      });

      it("marks it done with its cached count, counts it into the overall figure and points out it can be browsed", async () => {
        await renderView();
        await screen.findByText("Loading your Jira issues");

        act(() => emitEvent(api, "sync_progress", progress({ projectId: LUM.id, projectKey: LUM.key, pagesDone: 2, fetched: 40, approxTotal: 100 })));

        expect(await screen.findByText("About 73 of 133 issues")).toBeInTheDocument();
        expect(rows()[0]).toHaveTextContent("✓");
        expect(rows()[0]).toHaveTextContent("33");
        expect(rows()[1]).toHaveTextContent("page 3");
        expect(screen.getByText("You can already browse finished Spaces.")).toBeInTheDocument();
      });

      it("ignores a late progress event of the finished Space", async () => {
        await renderView();
        await screen.findByText("Loading your Jira issues");

        act(() => emitEvent(api, "sync_progress", progress({ fetched: 999, approxTotal: 999 })));

        expect(await screen.findByText("33 issues so far")).toBeInTheDocument();
        expect(rows()[0]).toHaveTextContent("✓");
      });
    });

    it("leaves out the note on finished Spaces while none is done", async () => {
      await renderView();
      await screen.findByText("Loading your Jira issues");

      expect(screen.queryByText("You can already browse finished Spaces.")).not.toBeInTheDocument();
    });

    it("shows the issue count once a sync_state idle event arrives", async () => {
      await renderView();
      await screen.findByText("Loading your Jira issues");
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [NW, LUM] }));

      act(() => emitEvent(api, "sync_state", { state: "idle", lastOk: "2026-03-04T09:30:00Z" }));

      expect(await screen.findByText("34 issues in 2 Spaces")).toBeInTheDocument();
      expect(screen.queryByText("Loading your Jira issues")).not.toBeInTheDocument();
    });
  });

  describe("when idle", () => {
    it("uses singular for one issue in one Space", async () => {
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts({ cached: 1, open: 1 })] }));

      await renderView();

      expect(await screen.findByText("1 issue in 1 Space")).toBeInTheDocument();
    });

    it("shows when the last full sync finished", async () => {
      const lastFullAt = "2026-03-04T09:30:00Z";
      mocked.syncOverview.mockResolvedValue(syncOverview({ lastFullAt, projects: [NW] }));

      await renderView();

      expect(await screen.findByText(`Synced at ${formatLocalTime(lastFullAt)}`)).toBeInTheDocument();
    });
  });

  describe("when the sync cannot run", () => {
    it.each<[string, SyncState, string]>([
      ["an error", { state: "error", kind: "internal", message: "Jira answered with an unexpected response." }, "Jira answered with an unexpected response."],
      ["a pause", { state: "paused", message: "Reconnect Jira to continue." }, "Reconnect Jira to continue."],
      ["an offline state", { state: "offline", since: "2026-03-04T09:30:00Z" }, `Offline since ${formatLocalTime("2026-03-04T09:30:00Z")}`],
    ])("shows the message for %s", async (_name, state, text) => {
      mocked.syncOverview.mockResolvedValue(syncOverview({ state, projects: [NW] }));

      await renderView();

      expect(await screen.findByText(text)).toBeInTheDocument();
    });
  });
});

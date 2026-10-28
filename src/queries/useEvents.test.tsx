import { act, render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { ConnectionPage } from "../components/settings/jira/ConnectionPage";
import { SyncPage } from "../components/settings/jira/SyncPage";
import { emitEvent } from "../test/apiMock";
import { connectionView, PROJECTS, projectCounts, syncOverview } from "../test/jira";
import { createQueryClient } from "./queryClient";
import { useEvents } from "./useEvents";

const mocked = vi.mocked(api, true);

function Harness() {
  useEvents();
  return (
    <>
      <ConnectionPage />
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
      emitEvent(api, "settings_changed", {
        connection: connectionView().connection,
        sync: { scopeOverride: null, focusedMinutes: 5, backgroundMinutes: 15, keepDoneDays: 90 },
      }),
    );

    await waitFor(() => expect(screen.getByLabelText("Email")).toHaveValue("sam.lee@example.com"));
  });
});

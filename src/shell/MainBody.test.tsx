import { render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore } from "../state/store";
import { groupOf, row, viewData } from "../test/issues";
import { projectCounts, syncOverview } from "../test/jira";
import { MainBody } from "./MainBody";

const mocked = vi.mocked(api, true);

type Restorable = { mockRestore: () => void };
let spies: Restorable[] = [];

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  spies = [
    vi.spyOn(Element.prototype, "clientHeight", "get").mockReturnValue(1000),
    vi.spyOn(Element.prototype, "clientWidth", "get").mockReturnValue(800),
  ];
});

afterEach(() => {
  spies.forEach((s) => s.mockRestore());
});

function renderBody() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <MainBody />
    </QueryClientProvider>,
  );
}

describe("MainBody", () => {
  it("asks to connect Jira while no project is tracked", async () => {
    mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });

    renderBody();

    expect(await screen.findByText("Connect Jira in Settings")).toBeInTheDocument();
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });

  it("shows the sync status while the tracked project has no cached issues", async () => {
    mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
    mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts({ cached: 0, open: 0 })] }));

    renderBody();

    expect(await screen.findByText("0 issues in 1 Space")).toBeInTheDocument();
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });

  it("shows the list of the current place once a project has cached issues", async () => {
    const issue = row({ summary: "Redesign the pricing page" });
    mocked.navCurrent.mockResolvedValue({ location: { listId: 11, viewId: 101 }, expanded: [] });
    mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));
    mocked.viewRows.mockResolvedValue(viewData([groupOf([issue])], [issue]));

    renderBody();

    expect(await screen.findByText("Redesign the pricing page")).toBeInTheDocument();
    expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, expect.anything());
  });

  it("keeps the sync status when there are issues but no place to show", async () => {
    mocked.navCurrent.mockResolvedValue({ location: null, expanded: [] });
    mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts()] }));

    renderBody();

    expect(await screen.findByText("33 issues in 1 Space")).toBeInTheDocument();
    expect(screen.queryByRole("grid")).not.toBeInTheDocument();
  });
});

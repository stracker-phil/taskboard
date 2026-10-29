import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type IssueDetail, type Nav } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { connectionView } from "../test/jira";
import { view } from "../test/views";
import { CurrentPeek } from "./CurrentPeek";

const mocked = vi.mocked(api, true);

const AT_LIST: Nav = { location: { listId: 11, viewId: 101 }, expanded: [] };
const PLACE = { scrollTop: 0, toggled: {}, collapsedParents: [], selectedId: "10042", peekOpen: true, onlyMine: false, text: "" };

const detail = (): IssueDetail => ({
  id: "10042",
  key: "NW-1042",
  issueType: "Story",
  summary: "Redesign the pricing page",
  style: "progress",
  statusName: "In Progress",
  epic: null,
  releases: [],
  priority: null,
  assignees: [],
  reporter: null,
  sprint: null,
  labels: [],
  due: null,
  createdAt: "2026-10-01T09:00:00Z",
  updatedAt: "2026-10-02T09:00:00Z",
  syncedAt: "2026-10-27T09:00:00.000Z",
  descriptionHtml: null,
  commentCount: 0,
  isEpic: false,
  gone: false,
  custom: [],
  since: null,
  unread: false,
  attachments: [],
});

function renderPeek() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <CurrentPeek />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.navCurrent.mockResolvedValue(AT_LIST);
  mocked.issueGet.mockResolvedValue(detail());
  mocked.viewsOf.mockResolvedValue([view()]);
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "" }));
});

describe("CurrentPeek", () => {
  it("renders nothing without a location", async () => {
    mocked.navCurrent.mockResolvedValue({ location: null, expanded: [] });
    resetStore({ places: { "11:101": PLACE } });

    const { container } = renderPeek();

    await waitFor(() => expect(mocked.navCurrent).toHaveBeenCalled());
    expect(container).toBeEmptyDOMElement();
  });

  it.each([
    ["has its peek closed", { ...PLACE, peekOpen: false }],
    ["has no selected issue", { ...PLACE, selectedId: null }],
  ])("renders nothing when the view's place %s", async (_, place) => {
    resetStore({ places: { "11:101": place } });

    const { container } = renderPeek();

    await waitFor(() => expect(mocked.navCurrent).toHaveBeenCalled());
    expect(container).toBeEmptyDOMElement();
  });

  it("renders nothing when only another view has its peek open", async () => {
    resetStore({ places: { "11:102": PLACE } });

    const { container } = renderPeek();

    await waitFor(() => expect(mocked.navCurrent).toHaveBeenCalled());
    expect(container).toBeEmptyDOMElement();
  });

  it("renders nothing in a Board view even when its place has a peek open", async () => {
    mocked.viewsOf.mockResolvedValue([view({ layout: "board" })]);
    resetStore({ places: { "11:101": PLACE } });

    const { container } = renderPeek();

    await waitFor(() => expect(mocked.viewsOf).toHaveBeenCalledWith(11));
    await waitFor(() => expect(container).toBeEmptyDOMElement());
    expect(mocked.issueGet).not.toHaveBeenCalled();
  });

  it.each(["list", "table"] as const)("shows the peek in a %s view", async (layout) => {
    mocked.viewsOf.mockResolvedValue([view({ layout })]);
    resetStore({ places: { "11:101": PLACE } });

    renderPeek();

    expect(await screen.findByRole("heading", { name: "Redesign the pricing page" })).toBeInTheDocument();
  });

  it("shows the selected issue of the current view when its peek is open", async () => {
    resetStore({ places: { "11:101": PLACE } });

    renderPeek();

    expect(await screen.findByRole("heading", { name: "Redesign the pricing page" })).toBeInTheDocument();
    expect(mocked.issueGet).toHaveBeenCalledWith("10042");
  });

  it("opens the Issue view on the selected issue with the view as origin from Full view", async () => {
    resetStore({ places: { "11:101": PLACE } });
    renderPeek();

    await userEvent.click(await screen.findByRole("button", { name: "Full view" }));

    expect(useStore.getState().issueView).toEqual({ origin: { listId: 11, viewId: 101 }, stack: ["10042"] });
  });

  it("closes the peek in the view's place when its close button is clicked", async () => {
    resetStore({ places: { "11:101": PLACE } });
    renderPeek();

    await userEvent.click(await screen.findByRole("button", { name: "Close (esc)" }));

    expect(useStore.getState().places["11:101"]).toMatchObject({ selectedId: "10042", peekOpen: false });
    expect(screen.queryByRole("complementary", { name: "Issue preview" })).not.toBeInTheDocument();
  });
});

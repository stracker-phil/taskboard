import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type IssueDetail } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { connectionView } from "../test/jira";
import { ALEX } from "../test/issues";
import { Peek } from "./Peek";

const mocked = vi.mocked(api, true);

function detail(patch: Partial<IssueDetail> = {}): IssueDetail {
  return {
    id: "10042",
    key: "NW-1042",
    issueType: "Story",
    summary: "Redesign the pricing page",
    style: "progress",
    statusName: "In Progress",
    epic: { id: "9", name: "Launch Q4", color: 2 },
    releases: ["Release 2.4"],
    priority: { name: "High", level: 2 },
    assignees: [ALEX],
    reporter: null,
    sprint: null,
    labels: [],
    due: null,
    createdAt: "2026-10-01T09:00:00Z",
    updatedAt: "2026-10-02T09:00:00Z",
    descriptionHtml: "<p>Update the <strong>pricing</strong> table.</p>",
    commentCount: 0,
    isEpic: false,
    gone: false,
    ...patch,
  };
}

function renderPeek(onClose = vi.fn()) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Peek id="10042" onClose={onClose} />
    </QueryClientProvider>,
  );
  return onClose;
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.issueGet.mockResolvedValue(detail());
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "" }));
});

describe("Peek", () => {
  it("shows the issue's key, type, title, status, epic, release, priority and assignee", async () => {
    renderPeek();

    expect(await screen.findByRole("heading", { name: "Redesign the pricing page" })).toBeInTheDocument();
    expect(screen.getByText("NW-1042")).toBeInTheDocument();
    expect(screen.getByText("Story")).toBeInTheDocument();
    expect(screen.getByText("In Progress")).toBeInTheDocument();
    expect(screen.getByText("Launch Q4")).toBeInTheDocument();
    expect(screen.getByText("Release 2.4")).toBeInTheDocument();
    expect(screen.getByText("High")).toBeInTheDocument();
    expect(screen.getByText("Alex Morgan")).toBeInTheDocument();
  });

  it("asks Rust for the issue by its id", async () => {
    renderPeek();

    await screen.findByRole("heading", { name: "Redesign the pricing page" });
    expect(mocked.issueGet).toHaveBeenCalledWith("10042");
  });

  it("shows the description as plain text without its tags", async () => {
    renderPeek();

    const description = await screen.findByText("Update the pricing table.");
    expect(description.textContent).not.toContain("<");
  });

  it.each([null, "", "<p> </p>"])("says there is no description when it is %j", async (descriptionHtml) => {
    mocked.issueGet.mockResolvedValue(detail({ descriptionHtml }));
    renderPeek();

    expect(await screen.findByText("No description.")).toBeInTheDocument();
  });

  it("leaves out the epic, release and priority an issue does not have", async () => {
    mocked.issueGet.mockResolvedValue(detail({ epic: null, releases: [], priority: null, assignees: [] }));
    renderPeek();

    await screen.findByRole("heading", { name: "Redesign the pricing page" });
    for (const text of ["Launch Q4", "Release 2.4", "High", "Alex Morgan"]) expect(screen.queryByText(text)).not.toBeInTheDocument();
  });

  it("calls onClose when the close button is clicked", async () => {
    const onClose = renderPeek();

    await userEvent.click(await screen.findByRole("button", { name: "Close (esc)" }));

    expect(onClose).toHaveBeenCalled();
  });

  it("shows Pin and Full view disabled", async () => {
    renderPeek();

    await screen.findByRole("heading", { name: "Redesign the pricing page" });
    expect(screen.getByRole("button", { name: "Pin" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Full view" })).toBeDisabled();
  });

  describe("Open in Jira", () => {
    it("opens the issue's page on the Jira site", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "https://example.atlassian.net" }));
      renderPeek();

      await userEvent.click(await screen.findByRole("button", { name: "Open in Jira" }));

      expect(mocked.openUrl).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW-1042");
    });

    it("is not offered without a Jira site", async () => {
      renderPeek();

      await screen.findByRole("heading", { name: "Redesign the pricing page" });
      await vi.waitFor(() => expect(mocked.connectionGet).toHaveBeenCalled());
      expect(screen.queryByRole("button", { name: "Open in Jira" })).not.toBeInTheDocument();
    });
  });

  it("says the issue is no longer in the cache when Rust cannot find it", async () => {
    mocked.issueGet.mockRejectedValue({ kind: "notFound", message: "No such issue", code: null });
    renderPeek();

    expect(await screen.findByText("This issue is no longer in the cache.")).toBeInTheDocument();
    expect(screen.queryByRole("heading")).not.toBeInTheDocument();
  });
});

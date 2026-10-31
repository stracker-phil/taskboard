import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type IssueDetail } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore } from "../state/store";
import { ALEX, issueDetail, SAM } from "../test/issues";
import { behaviour, settings } from "../test/settings";
import { AssignMeButton } from "./AssignMeButton";

const mocked = vi.mocked(api, true);

function renderButton(issue: IssueDetail = issueDetail({ assignedToMe: true })) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AssignMeButton issue={issue} className="peek__btn" />
    </QueryClientProvider>,
  );
}

const allowChanges = (on = true) => mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: on }) }));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  allowChanges();
});

describe("AssignMeButton", () => {
  it("offers Unassign, pressed, on an issue assigned to me", async () => {
    renderButton(issueDetail({ assignees: [SAM, ALEX], assignedToMe: true }));

    expect(await screen.findByRole("button", { name: "Unassign NW-1042" })).toHaveAttribute("aria-pressed", "true");
  });

  it("offers Assign to me, not pressed, on an issue assigned to someone else", async () => {
    renderButton(issueDetail({ assignees: [SAM], assignedToMe: false }));

    expect(await screen.findByRole("button", { name: "Assign NW-1042 to me" })).toHaveAttribute("aria-pressed", "false");
  });

  it("offers Assign to me on an unassigned issue", async () => {
    renderButton(issueDetail({ assignees: [], assignedToMe: false }));

    expect(await screen.findByRole("button", { name: "Assign NW-1042 to me" })).toHaveAttribute("aria-pressed", "false");
  });

  it("toggles the assignment of its issue when clicked", async () => {
    renderButton(issueDetail({ id: "10042", assignedToMe: false }));

    await userEvent.click(await screen.findByRole("button", { name: "Assign NW-1042 to me" }));

    expect(mocked.issueToggleMine).toHaveBeenCalledWith("10042");
  });

  it("is absent while assigning is switched off, though changes to Jira are on", async () => {
    mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, disabledChanges: ["assign"] }) }));
    renderButton();
    await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

    expect(screen.queryByRole("button", { name: /NW-1042/ })).not.toBeInTheDocument();
  });

  it("stays when only another kind of change is switched off", async () => {
    mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, disabledChanges: ["changeStatus"] }) }));
    renderButton(issueDetail({ assignedToMe: false }));

    expect(await screen.findByRole("button", { name: "Assign NW-1042 to me" })).toBeInTheDocument();
  });

  it("is absent while changes to Jira are off", async () => {
    allowChanges(false);
    renderButton();
    await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

    expect(screen.queryByRole("button", { name: /NW-1042/ })).not.toBeInTheDocument();
  });
});

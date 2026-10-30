import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type IssueDetail } from "../../api";
import { installCommands } from "../../commands";
import { getCommand } from "../../commands/registry";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import { issueDetail, related, viewData } from "../../test/issues";
import { connectionView } from "../../test/jira";
import { pins } from "../../test/pins";
import { settings } from "../../test/settings";
import { sidebarTree, space } from "../../test/spaces";
import { listNode, view } from "../../test/views";
import { IssueView } from "../IssueView";

const mocked = vi.mocked(api, true);

const ORIGIN = { listId: 11, viewId: 101 };
const ISSUE: IssueDetail = issueDetail({
  summary: "Plain summary",
  descriptionHtml: "<p>Zebra crossing, a second zebra and a third ZEBRA.</p>",
});

function renderIssue() {
  resetStore({ issueView: { origin: ORIGIN, stack: [ISSUE.id] } });
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <IssueView id={ISSUE.id} origin={ORIGIN} />
    </QueryClientProvider>,
  );
}

const findField = () => screen.findByRole("textbox", { name: "Find in this issue" });

async function findFor(query: string) {
  const field = await findField();
  await screen.findByText("Plain summary");
  await userEvent.type(field, query);
  return field;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("main");
  mocked.pinsList.mockResolvedValue(pins());
  mocked.getSettings.mockResolvedValue(settings());
  mocked.issueGet.mockResolvedValue(ISSUE);
  mocked.issueRelated.mockResolvedValue(related());
  mocked.issueActivity.mockResolvedValue([]);
  mocked.issueSyncComments.mockResolvedValue(false);
  mocked.viewRows.mockResolvedValue(viewData([], []));
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "" }));
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space({ lists: [listNode({ id: 21, name: "My open issues" })] })]));
  mocked.viewsOf.mockResolvedValue([view()]);
});

describe("Find in this issue", () => {
  it("shows no status before anything is typed", async () => {
    renderIssue();

    expect(await findField()).toHaveValue("");
    expect(screen.queryByText(/^\d+\/\d+$/)).not.toBeInTheDocument();
  });

  it("counts the matches of the typed words, the first one current", async () => {
    renderIssue();

    await findFor("zebra");

    expect(await screen.findByText("1/3")).toBeInTheDocument();
  });

  it("matches words of the summary too", async () => {
    renderIssue();

    await findFor("PLAIN");

    expect(await screen.findByText("1/1")).toBeInTheDocument();
  });

  it("shows 0/0 when nothing matches", async () => {
    renderIssue();

    await findFor("invoice");

    expect(await screen.findByText("0/0")).toBeInTheDocument();
  });

  describe("Enter", () => {
    it("keeps the first match on the first press, then advances and wraps after the last", async () => {
      renderIssue();
      await findFor("zebra");
      await screen.findByText("1/3");

      await userEvent.keyboard("{Enter}");
      expect(screen.getByText("1/3")).toBeInTheDocument();

      await userEvent.keyboard("{Enter}");
      expect(screen.getByText("2/3")).toBeInTheDocument();

      await userEvent.keyboard("{Enter}");
      expect(screen.getByText("3/3")).toBeInTheDocument();

      await userEvent.keyboard("{Enter}");
      expect(screen.getByText("1/3")).toBeInTheDocument();
    });

    it("goes back with Shift and wraps to the last match", async () => {
      renderIssue();
      await findFor("zebra");
      await screen.findByText("1/3");
      await userEvent.keyboard("{Enter}");

      await userEvent.keyboard("{Shift>}{Enter}{/Shift}");
      expect(screen.getByText("3/3")).toBeInTheDocument();

      await userEvent.keyboard("{Shift>}{Enter}{/Shift}");
      expect(screen.getByText("2/3")).toBeInTheDocument();
    });

    it("starts over at the first match when the words change", async () => {
      renderIssue();
      await findFor("zebra");
      await userEvent.keyboard("{Enter}{Enter}");
      expect(screen.getByText("2/3")).toBeInTheDocument();

      await userEvent.type(await findField(), " crossing");

      expect(await screen.findByText("1/3")).toBeInTheDocument();
    });
  });

  describe("Escape", () => {
    it("clears the field and its status without leaving the Issue view", async () => {
      renderIssue();
      const field = await findFor("zebra");
      await screen.findByText("1/3");

      await userEvent.keyboard("{Escape}");

      expect(field).toHaveValue("");
      expect(screen.queryByText(/^\d+\/\d+$/)).not.toBeInTheDocument();
      expect(useStore.getState().issueView).not.toBeNull();
    });
  });

  describe("the toolbar.search page action", () => {
    it("focuses the field", async () => {
      renderIssue();
      const field = await findField();
      expect(field).not.toHaveFocus();

      act(() => void getCommand("toolbar.search")?.run());

      expect(field).toHaveFocus();
    });
  });
});

import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { capabilityRows } from "../../../test/capabilities";
import { syncOverview } from "../../../test/jira";
import { settings } from "../../../test/settings";
import { PrivacyPage } from "./PrivacyPage";

const mocked = vi.mocked(api, true);

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <PrivacyPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.getSettings.mockResolvedValue(settings());
  mocked.syncOverview.mockResolvedValue(syncOverview());
});

describe("PrivacyPage", () => {
  it("shows the Jira data group before what Taskboard reads", async () => {
    renderPage();

    const data = await screen.findByText("Jira data");
    const reads = await screen.findByText("What Taskboard reads");

    expect(data.compareDocumentPosition(reads) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  describe("what Taskboard reads", () => {
    it("lists the read rows with their detail and no switch", async () => {
      renderPage();

      expect(await screen.findByText("Your account")).toBeInTheDocument();
      expect(screen.getByText(/Reads the issues of the projects you track\./)).toBeInTheDocument();
      expect(screen.queryByRole("switch", { name: "Your account" })).not.toBeInTheDocument();
      expect(screen.queryByRole("switch", { name: "Issues" })).not.toBeInTheDocument();
    });

    it("is absent while the capabilities load", async () => {
      mocked.jiraCapabilities.mockReturnValueOnce(new Promise(() => {}));

      renderPage();

      await screen.findByText("Jira data");
      expect(screen.queryByText("What Taskboard reads")).not.toBeInTheDocument();
    });

    it("lists no change among the reads", async () => {
      renderPage();

      await screen.findByText("Your account");

      expect(screen.queryByText("Assign issues")).not.toBeInTheDocument();
    });

    it("lists the required scopes of the reads once each, in row order, in the description", async () => {
      renderPage();

      expect(await screen.findByText(/Required for a scoped token:/)).toHaveTextContent(
        "Always on: Taskboard reads only these from Jira. Required for a scoped token: read:jira-user, read:jira-work.",
      );
    });

    it("derives the description from the scopes of the read rows", async () => {
      const rows = capabilityRows();
      mocked.jiraCapabilities.mockResolvedValue([
        { ...rows[0], scope: "read:example-first" },
        { ...rows[1], scope: "read:example-second" },
        { ...rows[5], scope: "write:example-one" },
      ]);
      renderPage();

      const description = await screen.findByText(/Required for a scoped token:/);

      expect(description).toHaveTextContent("read:example-first, read:example-second.");
      expect(description).not.toHaveTextContent("write:example-one");
    });
  });
});

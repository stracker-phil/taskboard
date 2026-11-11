import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
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
  it("shows the Jira data group", async () => {
    renderPage();

    expect(await screen.findByText("Jira data")).toBeInTheDocument();
  });

  it("does not list what Taskboard reads or the token scopes; those live on the Connection page", async () => {
    renderPage();

    await screen.findByText("Jira data");

    expect(screen.queryByText("What Taskboard reads")).not.toBeInTheDocument();
    expect(screen.queryByText("Token scopes")).not.toBeInTheDocument();
    expect(screen.queryByText("Your account")).not.toBeInTheDocument();
  });
});

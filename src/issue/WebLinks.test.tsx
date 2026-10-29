import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore } from "../state/store";
import { pullRequestLink, webLink } from "../test/webLinks";
import { WebLinks } from "./WebLinks";

const mocked = vi.mocked(api, true);

function renderLinks(variant: "issue" | "peek" = "issue") {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <WebLinks id="10042" variant={variant} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("WebLinks", () => {
  it("lists each link under Web links with its title and its reference or host", async () => {
    mocked.issueWebLinks.mockResolvedValue([pullRequestLink(), webLink()]);
    renderLinks();

    const section = within(await screen.findByRole("region", { name: "Web links" }));
    expect(section.getByRole("heading", { name: "Web links" })).toBeInTheDocument();
    const pullRequest = section.getByRole("button", { name: /Validate DACH postcodes/ });
    expect(pullRequest).toHaveTextContent("shop#481");
    expect(section.getByRole("button", { name: /Postcode guide/ })).toHaveTextContent("docs.example.com");
  });

  it("asks Rust to sync the web links of the shown issue", async () => {
    renderLinks();

    await waitFor(() => expect(mocked.issueSyncWebLinks).toHaveBeenCalledWith("10042"));
  });

  it.each(["issue", "peek"] as const)("shows nothing in the %s variant when the issue has no links", async (variant) => {
    renderLinks(variant);

    await waitFor(() => expect(mocked.issueWebLinks).toHaveBeenCalledWith("10042"));
    expect(screen.queryByRole("region", { name: "Web links" })).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Web links" })).not.toBeInTheDocument();
  });

  it("opens an openable link with its url", async () => {
    mocked.issueWebLinks.mockResolvedValue([pullRequestLink()]);
    renderLinks();

    await userEvent.click(await screen.findByRole("button", { name: /Validate DACH postcodes/ }));

    expect(mocked.openUrl).toHaveBeenCalledWith("https://github.com/example/shop/pull/481");
  });

  it.each(["http://intranet.example.com/x", "mailto:alex@example.com"])("shows the link %s as text that does not open", async (url) => {
    mocked.issueWebLinks.mockResolvedValue([webLink({ url, title: "Internal note", host: "intranet.example.com", openable: false })]);
    renderLinks();

    await userEvent.click(await screen.findByText("Internal note"));

    expect(screen.queryByRole("button", { name: /Internal note/ })).not.toBeInTheDocument();
    expect(mocked.openUrl).not.toHaveBeenCalled();
  });

  it("shows the newly fetched links at once when the sync stored new ones", async () => {
    mocked.issueWebLinks.mockResolvedValueOnce([]).mockResolvedValue([pullRequestLink(), webLink()]);
    mocked.issueSyncWebLinks.mockResolvedValue(true);
    renderLinks();

    const section = within(await screen.findByRole("region", { name: "Web links" }));
    expect(section.getByRole("button", { name: /Validate DACH postcodes/ })).toBeInTheDocument();
    expect(section.getByRole("button", { name: /Postcode guide/ })).toBeInTheDocument();
  });

  it("keeps the section hidden when the sync stored nothing new", async () => {
    mocked.issueWebLinks.mockResolvedValueOnce([]).mockResolvedValue([pullRequestLink(), webLink()]);
    mocked.issueSyncWebLinks.mockResolvedValue(false);
    renderLinks();

    await waitFor(() => expect(mocked.issueSyncWebLinks).toHaveBeenCalled());
    await mocked.issueSyncWebLinks.mock.results[0].value;
    await waitFor(() => expect(mocked.issueWebLinks).toHaveBeenCalled());
    expect(screen.queryByRole("region", { name: "Web links" })).not.toBeInTheDocument();
  });

  it("still shows the cached links when the sync fails", async () => {
    mocked.issueWebLinks.mockResolvedValue([pullRequestLink()]);
    mocked.issueSyncWebLinks.mockRejectedValue({ kind: "network", message: "Offline", code: null });
    renderLinks();

    expect(await screen.findByRole("button", { name: /Validate DACH postcodes/ })).toBeInTheDocument();
    await waitFor(() => expect(mocked.issueSyncWebLinks).toHaveBeenCalled());
  });
});

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { connectionView } from "../test/jira";
import { JiraHtml } from "./JiraHtml";

const mocked = vi.mocked(api, true);

function renderHtml(html: string) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <JiraHtml html={html} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "https://example.atlassian.net/" }));
});

describe("JiraHtml", () => {
  it("renders the sanitized markup without its scripts", () => {
    renderHtml('<p>Hello <strong>team</strong></p><script>alert(1)</script>');

    expect(screen.getByText("team").tagName).toBe("STRONG");
    expect(document.querySelector("script")).toBeNull();
  });

  it("opens a clicked link in the browser instead of navigating", async () => {
    renderHtml('<p><a href="https://example.com/spec">Spec</a></p>');

    const event = new MouseEvent("click", { bubbles: true, cancelable: true });
    screen.getByRole("link", { name: "Spec" }).dispatchEvent(event);

    expect(mocked.openUrl).toHaveBeenCalledWith("https://example.com/spec");
    expect(event.defaultPrevented).toBe(true);
  });

  it("opens a relative link on the Jira site once the connection is known", async () => {
    renderHtml('<a href="/browse/NW-1">NW-1</a>');

    await vi.waitFor(() => expect(screen.getByRole("link", { name: "NW-1" })).toHaveAttribute("href", "https://example.atlassian.net/browse/NW-1"));
    await userEvent.click(screen.getByRole("link", { name: "NW-1" }));

    expect(mocked.openUrl).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW-1");
  });

  it("does nothing when a link without an address is clicked", async () => {
    renderHtml('<a href="javascript:alert(1)">Bad</a>');

    await userEvent.click(screen.getByText("Bad"));

    expect(mocked.openUrl).not.toHaveBeenCalled();
  });

  it("does nothing when plain text is clicked", async () => {
    renderHtml('<p>Just text <a href="https://example.com/x">link</a></p>');

    await userEvent.click(screen.getByText("Just text", { exact: false }));

    expect(mocked.openUrl).not.toHaveBeenCalled();
  });
});

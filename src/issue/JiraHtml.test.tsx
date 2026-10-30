import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { connectionView } from "../test/jira";
import { JiraHtml } from "./JiraHtml";

const mocked = vi.mocked(api, true);

const IMAGE_HTML = '<p>See <img data-attachment-id="501" alt="Checkout screenshot" width="640" height="360"></p>';
const IMAGE_URL = "inline-image://localhost/issue-7/501";

function renderHtml(html: string) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <JiraHtml html={html} issueId="issue-7" />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "https://example.atlassian.net/" }));
});

afterEach(() => {
  vi.unstubAllGlobals();
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

  describe("inline images", () => {
    it("points the image at the app scheme for the issue and attachment, named by its alt text", () => {
      renderHtml(IMAGE_HTML);

      expect(screen.getByAltText("Checkout screenshot")).toHaveAttribute("src", IMAGE_URL);
    });

    it("keeps the image busy and its button disabled until the image has loaded", () => {
      renderHtml(IMAGE_HTML);

      expect(screen.getByAltText("Checkout screenshot")).toHaveAttribute("aria-busy", "true");
      expect(screen.getByRole("button")).toBeDisabled();
    });

    it("enables the image button once the image has loaded", () => {
      renderHtml(IMAGE_HTML);

      fireEvent.load(screen.getByAltText("Checkout screenshot"));

      expect(screen.getByRole("button", { name: "Checkout screenshot" })).toBeEnabled();
      expect(screen.getByAltText("Checkout screenshot")).not.toHaveAttribute("aria-busy");
    });

    it("falls back to a chip when the image fails to load", () => {
      renderHtml(IMAGE_HTML);

      fireEvent.error(screen.getByAltText("Checkout screenshot"));

      expect(screen.getByText("Attachment · Checkout screenshot")).toBeInTheDocument();
      expect(screen.queryByAltText("Checkout screenshot")).toBeNull();
    });

    it("opens Quick Look for the attachment when the loaded image is clicked", async () => {
      renderHtml(IMAGE_HTML);
      fireEvent.load(screen.getByAltText("Checkout screenshot"));

      await userEvent.click(screen.getByRole("button", { name: "Checkout screenshot" }));

      expect(mocked.attachmentPreview).toHaveBeenCalledWith("issue-7", "501");
    });

    describe("lazy loading", () => {
      let report: (isIntersecting: boolean) => void;

      beforeEach(() => {
        class FakeObserver {
          constructor(callback: (entries: { isIntersecting: boolean }[]) => void) {
            report = (isIntersecting) => callback([{ isIntersecting }]);
          }
          observe() {}
          disconnect() {}
        }
        vi.stubGlobal("IntersectionObserver", FakeObserver);
      });

      it("holds the place with a busy box and loads the image only once it has scrolled into view", () => {
        renderHtml(IMAGE_HTML);

        act(() => report(false));
        expect(screen.getByRole("img", { name: "Checkout screenshot" })).toHaveAttribute("aria-busy", "true");
        expect(screen.queryByRole("button")).toBeNull();

        act(() => report(true));

        expect(screen.getByAltText("Checkout screenshot")).toHaveAttribute("src", IMAGE_URL);
      });
    });
  });
});

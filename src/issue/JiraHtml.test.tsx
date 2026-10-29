import { act, render, screen } from "@testing-library/react";
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
const DATA_URL = "data:image/png;base64,AAAA";

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
  mocked.attachmentImage.mockResolvedValue(null);
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
    it("shows the image from the data URL the api returns, named by its alt text", async () => {
      mocked.attachmentImage.mockResolvedValue(DATA_URL);

      renderHtml(IMAGE_HTML);

      expect(await screen.findByAltText("Checkout screenshot")).toHaveAttribute("src", DATA_URL);
      expect(mocked.attachmentImage).toHaveBeenCalledWith("issue-7", "501");
    });

    it("holds the place with a busy box while the image is loading", async () => {
      mocked.attachmentImage.mockReturnValue(new Promise(() => {}));

      renderHtml(IMAGE_HTML);

      expect(await screen.findByRole("img", { name: "Checkout screenshot" })).toHaveAttribute("aria-busy", "true");
    });

    it("keeps a chip when the api has no image for the attachment", async () => {
      renderHtml(IMAGE_HTML);

      expect(await screen.findByText("Attachment · Checkout screenshot")).toBeInTheDocument();
      expect(screen.queryByRole("img")).toBeNull();
    });

    it("keeps a chip when loading the image fails", async () => {
      mocked.attachmentImage.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      renderHtml(IMAGE_HTML);

      expect(await screen.findByText("Attachment · Checkout screenshot")).toBeInTheDocument();
    });

    it("opens Quick Look for the attachment when the image is clicked", async () => {
      mocked.attachmentImage.mockResolvedValue(DATA_URL);
      renderHtml(IMAGE_HTML);

      await userEvent.click(await screen.findByRole("button", { name: "Checkout screenshot" }));

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

      it("asks for the image only once it has scrolled into view", async () => {
        mocked.attachmentImage.mockResolvedValue(DATA_URL);
        renderHtml(IMAGE_HTML);

        await screen.findByRole("img", { name: "Checkout screenshot" });
        report(false);
        expect(mocked.attachmentImage).not.toHaveBeenCalled();

        act(() => report(true));

        expect(await screen.findByAltText("Checkout screenshot")).toHaveAttribute("src", DATA_URL);
        expect(mocked.attachmentImage).toHaveBeenCalledWith("issue-7", "501");
      });
    });
  });
});

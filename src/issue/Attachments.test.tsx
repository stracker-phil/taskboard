import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { MenuHost } from "../components/menu";
import { resetStore, useStore } from "../state/store";
import { attachment } from "../test/issues";
import { Attachments } from "./Attachments";

const mocked = vi.mocked(api, true);

const URL = "https://example.com/secure/attachment/30001/release-notes.pdf";

function renderAttachments(...attachments: ReturnType<typeof attachment>[]) {
  return render(
    <>
      <Attachments issueId="10042" attachments={attachments} />
      <MenuHost />
    </>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("Attachments", () => {
  it("renders nothing without attachments", () => {
    renderAttachments();

    expect(screen.queryByRole("region", { name: "Attachments" })).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Attachments" })).not.toBeInTheDocument();
  });

  describe("tiles", () => {
    it("lists each file in an Attachments region with its filename and size and author", () => {
      renderAttachments(attachment(), attachment({ id: "30002", filename: "wireframes.png", badge: "PNG", size: "340 KB" }));

      const region = within(screen.getByRole("region", { name: "Attachments" }));
      expect(region.getByRole("heading", { name: "Attachments" })).toBeInTheDocument();
      expect(region.getByRole("button", { name: /release-notes\.pdf/ })).toHaveTextContent("1.2 MB · Ada Example");
      expect(region.getByRole("button", { name: /wireframes\.png/ })).toHaveTextContent("340 KB · Ada Example");
    });

    it("shows the badge of the file type", () => {
      renderAttachments(attachment());

      expect(screen.getByText("PDF")).toBeInTheDocument();
    });

    it.each([
      ["has no size", { size: null }, "Ada Example"],
      ["has no author", { authorName: "" }, "1.2 MB"],
    ])("leaves out the part that is missing when the file %s", (_case, patch, meta) => {
      renderAttachments(attachment(patch));

      expect(screen.getByText(meta, { exact: true })).toBeInTheDocument();
    });

    it("shows a filename that looks like markup as plain text", () => {
      const filename = "<img src=x onerror=alert(1)>.png";
      const { container } = renderAttachments(attachment({ filename }));

      expect(screen.getByText(filename)).toBeInTheDocument();
      expect(container.querySelector("img")).toBeNull();
    });
  });

  describe("previewing", () => {
    const tile = () => screen.getByRole("button", { name: /release-notes\.pdf/ });

    it("previews the file of the issue in Quick Look when its tile is clicked, without opening the browser", async () => {
      renderAttachments(attachment());

      await userEvent.click(tile());

      expect(mocked.attachmentPreview).toHaveBeenCalledWith("10042", "30001");
      expect(mocked.openUrl).not.toHaveBeenCalled();
    });

    it("previews a file without an address too", async () => {
      renderAttachments(attachment({ url: null }));

      await userEvent.click(tile());

      expect(mocked.attachmentPreview).toHaveBeenCalledWith("10042", "30001");
    });

    it("marks the tile busy and reads Loading… while the preview is pending, then restores the meta line", async () => {
      let finish: () => void = () => {};
      mocked.attachmentPreview.mockReturnValue(new Promise<void>((resolve) => (finish = resolve)));
      renderAttachments(attachment());

      await userEvent.click(tile());

      expect(tile()).toHaveAttribute("aria-busy", "true");
      expect(tile()).toHaveTextContent("Loading…");
      expect(tile()).not.toHaveTextContent("1.2 MB");

      finish();

      await waitFor(() => expect(tile()).toHaveAttribute("aria-busy", "false"));
      expect(tile()).toHaveTextContent("1.2 MB · Ada Example");
    });

    it("shows an error toast when the file cannot be previewed", async () => {
      mocked.attachmentPreview.mockRejectedValue({ kind: "network", message: "Download failed", code: null });
      renderAttachments(attachment());

      await userEvent.click(tile());

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not show the attachment: Download failed"));
      expect(tile()).toHaveAttribute("aria-busy", "false");
    });
  });

  describe("context menu", () => {
    const rightClick = () => userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("button", { name: /release-notes\.pdf/ }) });

    it("offers Quick Look, Open in Browser and Copy link on right-click", async () => {
      renderAttachments(attachment());

      await rightClick();

      const menu = within(screen.getByRole("menu"));
      expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
        expect.stringContaining("Quick Look"),
        expect.stringContaining("Open in Browser"),
        expect.stringContaining("Copy link"),
      ]);
      expect(menu.getByRole("menuitem", { name: "Open in Browser" })).not.toHaveAttribute("aria-disabled");
      expect(menu.getByRole("menuitem", { name: "Copy link" })).not.toHaveAttribute("aria-disabled");
    });

    it("previews the file from Quick Look", async () => {
      renderAttachments(attachment());

      await rightClick();
      await userEvent.click(screen.getByRole("menuitem", { name: "Quick Look" }));

      expect(mocked.attachmentPreview).toHaveBeenCalledWith("10042", "30001");
    });

    it("opens the file's address from Open in Browser", async () => {
      renderAttachments(attachment());

      await rightClick();
      await userEvent.click(screen.getByRole("menuitem", { name: "Open in Browser" }));

      expect(mocked.openUrl).toHaveBeenCalledWith(URL);
    });

    it("shows an error toast when the address cannot be opened", async () => {
      mocked.openUrl.mockRejectedValue({ kind: "invalid", message: "Only https links open", code: null });
      renderAttachments(attachment());

      await rightClick();
      await userEvent.click(screen.getByRole("menuitem", { name: "Open in Browser" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not open the attachment: Only https links open"));
    });

    it("disables Open in Browser and Copy link without an address and does nothing when they are chosen", async () => {
      renderAttachments(attachment({ url: null }));

      await rightClick();
      const open = screen.getByRole("menuitem", { name: "Open in Browser" });
      const copy = screen.getByRole("menuitem", { name: "Copy link" });
      expect(open).toHaveAttribute("aria-disabled", "true");
      expect(copy).toHaveAttribute("aria-disabled", "true");
      expect(screen.getByRole("menuitem", { name: "Quick Look" })).not.toHaveAttribute("aria-disabled");

      await userEvent.click(open);
      await userEvent.click(copy);

      expect(mocked.openUrl).not.toHaveBeenCalled();
      expect(mocked.copyText).not.toHaveBeenCalled();
    });

    it("copies the address from Copy link and confirms it in a toast", async () => {
      renderAttachments(attachment());

      await rightClick();
      await userEvent.click(screen.getByRole("menuitem", { name: "Copy link" }));

      expect(mocked.copyText).toHaveBeenCalledWith(URL);
      await waitFor(() => expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["success", "Copied the link to release-notes.pdf"]]));
    });

    it("shows an error toast and no success toast when the link cannot be copied", async () => {
      mocked.copyText.mockRejectedValue({ kind: "io", message: "Clipboard is not available", code: null });
      renderAttachments(attachment());

      await rightClick();
      await userEvent.click(screen.getByRole("menuitem", { name: "Copy link" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => [t.kind, t.message])).toEqual([["error", "Could not copy: Clipboard is not available"]]));
    });
  });
});

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
      <Attachments attachments={attachments} />
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

  describe("opening", () => {
    it("opens the file's address when its tile is clicked", async () => {
      renderAttachments(attachment());

      await userEvent.click(screen.getByRole("button", { name: /release-notes\.pdf/ }));

      expect(mocked.openUrl).toHaveBeenCalledWith(URL);
    });

    it("is inert without an address: not a button, and clicking does nothing", async () => {
      renderAttachments(attachment({ url: null }));

      expect(screen.queryByRole("button")).not.toBeInTheDocument();
      await userEvent.click(screen.getByText("release-notes.pdf"));

      expect(mocked.openUrl).not.toHaveBeenCalled();
    });

    it("shows an error toast when the file cannot be opened", async () => {
      mocked.openUrl.mockRejectedValue({ kind: "invalid", message: "Only https links open", code: null });
      renderAttachments(attachment());

      await userEvent.click(screen.getByRole("button", { name: /release-notes\.pdf/ }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not open the attachment: Only https links open"));
    });
  });

  describe("context menu", () => {
    const rightClick = () => userEvent.pointer({ keys: "[MouseRight]", target: screen.getByRole("button", { name: /release-notes\.pdf/ }) });

    it("offers Open and Copy link on right-click", async () => {
      renderAttachments(attachment());

      await rightClick();

      const menu = within(screen.getByRole("menu"));
      expect(menu.getByRole("menuitem", { name: "Open" })).toBeInTheDocument();
      expect(menu.getByRole("menuitem", { name: "Copy link" })).toBeInTheDocument();
    });

    it("opens the file's address from Open", async () => {
      renderAttachments(attachment());

      await rightClick();
      await userEvent.click(screen.getByRole("menuitem", { name: "Open" }));

      expect(mocked.openUrl).toHaveBeenCalledWith(URL);
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

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { useSettings } from "../queries/settings";
import { resetStore, useStore } from "../state/store";
import { behaviour, settings } from "../test/settings";
import { PinPopover } from "./PinPopover";

const mocked = vi.mocked(api, true);

const onClose = vi.fn();

function renderPopover(props: { meta?: string } = {}) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <PinPopover title="Bookmark this comment" kind="comment" target="c-77" anchor={{ current: null }} onClose={onClose} {...props} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.getSettings.mockResolvedValue(settings());
});

describe("PinPopover", () => {
  it("shows the title, the meta line and the four durations with Until removed chosen", () => {
    renderPopover({ meta: "Sam Rivera · 3 days ago" });

    const dialog = screen.getByRole("dialog", { name: "Bookmark this comment" });
    expect(dialog).toHaveTextContent("Bookmark this comment");
    expect(dialog).toHaveTextContent("Sam Rivera · 3 days ago");
    expect(dialog).toHaveTextContent("Keep bookmarked for");
    for (const name of ["Today", "2 days", "1 week", "Until removed"]) {
      expect(screen.getByRole("button", { name })).toHaveAttribute("aria-pressed", name === "Until removed" ? "true" : "false");
    }
    expect(screen.getByLabelText(/Reason/)).toHaveValue("");
  });

  it("starts on the duration set in the settings when they are loaded before it opens", async () => {
    mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ pinDefault: "twoDays" }) }));
    function OpensWhenLoaded() {
      const { data } = useSettings();
      return data ? <PinPopover title="Bookmark this comment" kind="comment" target="c-77" anchor={{ current: null }} onClose={onClose} /> : null;
    }
    render(
      <QueryClientProvider client={createQueryClient()}>
        <OpensWhenLoaded />
      </QueryClientProvider>,
    );

    expect(await screen.findByRole("button", { name: "2 days" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Until removed" })).toHaveAttribute("aria-pressed", "false");
  });

  it("moves the chosen mark to the clicked duration only", async () => {
    renderPopover();

    await userEvent.click(screen.getByRole("button", { name: "Today" }));

    expect(screen.getByRole("button", { name: "Today" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Until removed" })).toHaveAttribute("aria-pressed", "false");
  });

  describe("bookmarking", () => {
    it("sends the kind, target, chosen duration and typed reason, then closes", async () => {
      renderPopover();

      await userEvent.click(screen.getByRole("button", { name: "1 week" }));
      await userEvent.type(screen.getByLabelText(/Reason/), "  Answer before Friday  ");
      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      expect(mocked.pinCreate).toHaveBeenCalledWith("comment", "c-77", "week", "Answer before Friday");
      expect(onClose).toHaveBeenCalled();
    });

    it("sends no reason when the field is empty or whitespace", async () => {
      renderPopover();

      await userEvent.type(screen.getByLabelText(/Reason/), "   ");
      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      expect(mocked.pinCreate).toHaveBeenCalledWith("comment", "c-77", "untilUnpinned", null);
    });

    it("bookmarks on Enter in the reason field", async () => {
      renderPopover();

      await userEvent.type(screen.getByLabelText(/Reason/), "Check Monday{Enter}");

      expect(mocked.pinCreate).toHaveBeenCalledWith("comment", "c-77", "untilUnpinned", "Check Monday");
      expect(onClose).toHaveBeenCalled();
    });

    it("bookmarks on Enter while a duration chip has focus, without re-choosing it", async () => {
      renderPopover();

      await userEvent.click(screen.getByRole("button", { name: "Today" }));
      await userEvent.keyboard("{Enter}");

      expect(mocked.pinCreate).toHaveBeenCalledWith("comment", "c-77", "today", null);
    });

    it("shows an error toast when the bookmark could not be saved", async () => {
      mocked.pinCreate.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      renderPopover();

      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not bookmark it: Disk full"));
    });
  });

  describe("leaving without bookmarking", () => {
    it("closes on Cancel without creating a bookmark", async () => {
      renderPopover();

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(onClose).toHaveBeenCalled();
      expect(mocked.pinCreate).not.toHaveBeenCalled();
    });

    it("closes on Escape without creating a bookmark", async () => {
      renderPopover();

      await userEvent.keyboard("{Escape}");

      expect(onClose).toHaveBeenCalled();
      expect(mocked.pinCreate).not.toHaveBeenCalled();
    });
  });
});

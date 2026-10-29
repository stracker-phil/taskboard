import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore } from "../state/store";
import { pinItem, pins } from "../test/pins";
import { settings } from "../test/settings";
import { PinButton } from "./PinButton";

const mocked = vi.mocked(api, true);

function renderButton() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <PinButton kind="issue" target="10042" label="Bookmark this issue" meta="NW-1042" className="peek__btn" />
    </QueryClientProvider>,
  );
}

const open = async () => {
  await userEvent.click(await screen.findByRole("button", { name: "Bookmark this issue" }));
  return screen.findByRole("dialog", { name: "Bookmark this issue" });
};

const settingsLoaded = () => act(async () => void (await mocked.getSettings.mock.results[0]?.value));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.pinsList.mockResolvedValue(pins());
  mocked.getSettings.mockResolvedValue(settings());
});

describe("PinButton", () => {
  describe("on an unpinned target", () => {
    it("opens a dialog with the title, the durations (Until removed chosen) and a reason field", async () => {
      renderButton();

      const dialog = await open();

      expect(dialog).toHaveTextContent("Bookmark this issue");
      expect(dialog).toHaveTextContent("NW-1042");
      expect(dialog).toHaveTextContent("Keep bookmarked for");
      for (const name of ["Today", "2 days", "1 week", "Until removed"]) {
        expect(screen.getByRole("button", { name })).toHaveAttribute("aria-pressed", name === "Until removed" ? "true" : "false");
      }
      expect(screen.getByLabelText(/Reason/)).toHaveValue("");
    });

    it("preselects the duration set in the settings", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds: 2, pinDefault: "week", openIssuesIn: "preview" } }));
      renderButton();
      await settingsLoaded();

      await open();

      expect(screen.getByRole("button", { name: "1 week" })).toHaveAttribute("aria-pressed", "true");
      expect(screen.getByRole("button", { name: "Until removed" })).toHaveAttribute("aria-pressed", "false");
    });

    it("pins with the duration set in the settings when the durations are left alone", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds: 2, pinDefault: "today", openIssuesIn: "preview" } }));
      renderButton();
      await settingsLoaded();
      await open();

      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "10042", "today", null);
    });

    it("pins with the chosen duration and the typed reason", async () => {
      renderButton();
      await open();

      await userEvent.click(screen.getByRole("button", { name: "1 week" }));
      await userEvent.type(screen.getByLabelText(/Reason/), "Waiting for the copy");
      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "10042", "week", "Waiting for the copy");
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("pins with the default duration when the durations are left alone", async () => {
      renderButton();
      await open();

      await userEvent.click(screen.getByRole("button", { name: "Bookmark" }));

      expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "10042", "untilUnpinned", null);
    });

    it("pins on enter in the reason field", async () => {
      renderButton();
      await open();

      await userEvent.type(screen.getByLabelText(/Reason/), "Check Monday{Enter}");

      expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "10042", "untilUnpinned", "Check Monday");
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("sends no reason when it is only whitespace", async () => {
      renderButton();
      await open();

      await userEvent.type(screen.getByLabelText(/Reason/), "   {Enter}");

      expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "10042", "untilUnpinned", null);
    });

    it("closes without pinning on Cancel", async () => {
      renderButton();
      await open();

      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(mocked.pinCreate).not.toHaveBeenCalled();
    });

    it("closes without pinning on Escape", async () => {
      renderButton();
      await open();

      await userEvent.keyboard("{Escape}");

      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
      expect(mocked.pinCreate).not.toHaveBeenCalled();
    });

    it("pins at once until unpinned without a dialog on an alt-click", async () => {
      renderButton();
      await settingsLoaded();

      fireEvent.click(await screen.findByRole("button", { name: "Bookmark this issue" }), { altKey: true });

      expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "10042", "untilUnpinned", null);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("pins at once with the duration set in the settings on an alt-click", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: { seenAfterSeconds: 2, pinDefault: "twoDays", openIssuesIn: "preview" } }));
      renderButton();
      await settingsLoaded();

      fireEvent.click(await screen.findByRole("button", { name: "Bookmark this issue" }), { altKey: true });

      expect(mocked.pinCreate).toHaveBeenCalledWith("issue", "10042", "twoDays", null);
    });
  });

  describe("on a pinned target", () => {
    it("offers Remove bookmark, pressed, and removes it on click without opening the dialog", async () => {
      mocked.pinsList.mockResolvedValue(pins([pinItem({ id: 7 })]));
      renderButton();

      const button = await screen.findByRole("button", { name: "Remove bookmark" });
      expect(button).toHaveAttribute("aria-pressed", "true");
      await userEvent.click(button);

      expect(mocked.pinRemove).toHaveBeenCalledWith(7);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });
  });
});

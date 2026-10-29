import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Behaviour, type PinDuration } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import { settings } from "../../test/settings";
import { GeneralPage } from "./GeneralPage";

const mocked = vi.mocked(api, true);

const behaviour = (patch: Partial<Behaviour> = {}): Behaviour => ({ seenAfterSeconds: 2, pinDefault: "untilUnpinned", ...patch });

async function renderPage(current: Behaviour = behaviour()) {
  mocked.getSettings.mockResolvedValue(settings({ behaviour: current }));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <GeneralPage />
    </QueryClientProvider>,
  );
  return screen.findByLabelText("Count an issue as read after");
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.getSettings.mockResolvedValue(settings());
  mocked.behaviourSet.mockImplementation(async (b) => settings({ behaviour: b }));
});

describe("GeneralPage", () => {
  describe("Behavior", () => {
    it("shows the seconds before an issue counts as read", async () => {
      const field = await renderPage(behaviour({ seenAfterSeconds: 4 }));

      expect(field).toHaveValue(4);
      expect(screen.getByText("seconds")).toBeInTheDocument();
    });

    it("sends the new delay with the current pin default when it is committed", async () => {
      const field = await renderPage(behaviour({ pinDefault: "week" }));

      await userEvent.clear(field);
      await userEvent.type(field, "5{Enter}");

      expect(mocked.behaviourSet).toHaveBeenCalledWith({ seenAfterSeconds: 5, pinDefault: "week" });
    });

    it("accepts 0 to count an issue as read at once", async () => {
      const field = await renderPage();

      await userEvent.clear(field);
      await userEvent.type(field, "0{Enter}");

      expect(mocked.behaviourSet).toHaveBeenCalledWith({ seenAfterSeconds: 0, pinDefault: "untilUnpinned" });
    });

    it.each(["11", "-1", "1.5"])("does not send %s and shows the saved delay again", async (typed) => {
      const field = await renderPage();

      await userEvent.clear(field);
      await userEvent.type(field, `${typed}{Enter}`);

      expect(mocked.behaviourSet).not.toHaveBeenCalled();
      expect(field).toHaveValue(2);
    });

    it("offers Today, 2 days, 1 week and Until removed with the saved one pressed", async () => {
      await renderPage(behaviour({ pinDefault: "week" }));

      const group = screen.getByRole("group", { name: "Bookmark for" });
      expect(within(group).getAllByRole("button").map((b) => b.textContent)).toEqual(["Today", "2 days", "1 week", "Until removed"]);
      expect(within(group).getByRole("button", { pressed: true })).toHaveTextContent("1 week");
    });

    it.each<[string, PinDuration]>([
      ["Today", "today"],
      ["2 days", "twoDays"],
      ["1 week", "week"],
    ])("sends %s as the pin default together with the current delay", async (label, pinDefault) => {
      await renderPage(behaviour({ seenAfterSeconds: 7 }));

      await userEvent.click(within(screen.getByRole("group", { name: "Bookmark for" })).getByRole("button", { name: label }));

      expect(mocked.behaviourSet).toHaveBeenCalledWith({ seenAfterSeconds: 7, pinDefault });
    });

    it("shows an error toast when the setting cannot be saved", async () => {
      mocked.behaviourSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderPage();

      await userEvent.click(within(screen.getByRole("group", { name: "Bookmark for" })).getByRole("button", { name: "Today" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not save the setting: Disk full"));
    });
  });
});

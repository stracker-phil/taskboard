import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { resetStore, useStore } from "../state/store";
import { copyWithToast } from "./clipboard";

const mocked = vi.mocked(api, true);
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("copyWithToast", () => {
  it("copies the text and confirms with a success toast", async () => {
    copyWithToast("DEMO-12", "Copied DEMO-12");

    await vi.waitFor(() => expect(toasts()).toEqual([["success", "Copied DEMO-12"]]));
    expect(mocked.copyText).toHaveBeenCalledWith("DEMO-12");
  });

  it("shows an error toast with the reason and no confirmation when copying fails", async () => {
    mocked.copyText.mockRejectedValue({ kind: "io", message: "Pasteboard unavailable", code: null });

    copyWithToast("DEMO-12", "Copied DEMO-12");

    await vi.waitFor(() => expect(toasts()).toEqual([["error", "Could not copy: Pasteboard unavailable"]]));
  });
});

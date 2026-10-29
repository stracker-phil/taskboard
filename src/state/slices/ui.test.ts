import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { resetStore, useStore } from "../store";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("ui slice", () => {
  it("starts with the sidebar shown and the default activity width", () => {
    expect(useStore.getState()).toMatchObject({ sidebarHidden: false, activityWidth: null });
  });

  it("loads the sidebar state and the activity width Rust saved", async () => {
    mocked.getUiState.mockResolvedValue({ sidebarHidden: true, activityWidth: 520 });

    await useStore.getState().loadUiState();

    expect(useStore.getState()).toMatchObject({ sidebarHidden: true, activityWidth: 520 });
  });

  it("toasts an error and keeps the defaults when the layout cannot be loaded", async () => {
    mocked.getUiState.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

    await useStore.getState().loadUiState();

    expect(useStore.getState().activityWidth).toBeNull();
    expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not load the window layout: Disk full");
  });

  it("saves both fields when the sidebar is toggled, keeping the activity width", () => {
    resetStore({ activityWidth: 520 });

    useStore.getState().toggleSidebar();

    expect(useStore.getState().sidebarHidden).toBe(true);
    expect(mocked.setUiState).toHaveBeenCalledWith({ sidebarHidden: true, activityWidth: 520 });
  });

  it("saves both fields when the activity width is set, keeping the sidebar state", () => {
    resetStore({ sidebarHidden: true });

    useStore.getState().setActivityWidth(448);

    expect(mocked.setUiState).toHaveBeenCalledWith({ sidebarHidden: true, activityWidth: 448 });
  });
});

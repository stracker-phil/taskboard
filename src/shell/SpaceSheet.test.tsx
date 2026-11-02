import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { sidebarTree, space, spaceConfig } from "../test/spaces";
import { SpaceSheet } from "./SpaceSheet";

const mocked = vi.mocked(api, true);

function renderSheet(projectId: string | null) {
  useStore.getState().setSpaceSheet(projectId);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SpaceSheet />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space()]));
  mocked.spaceConfig.mockResolvedValue(spaceConfig());
});

describe("SpaceSheet", () => {
  it("shows nothing while no Space is set", async () => {
    renderSheet(null);

    await vi.waitFor(() => expect(mocked.sidebarTree).toHaveBeenCalled());
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("shows nothing for an unknown project id", async () => {
    renderSheet("99999");

    await vi.waitFor(() => expect(mocked.sidebarTree).toHaveBeenCalled());
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("shows the Space's settings in a dialog named Space Settings", async () => {
    renderSheet("10001");

    const dialog = await screen.findByRole("dialog", { name: "Space Settings" });
    expect(dialog).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "Website relaunch" })).toBeInTheDocument();
    expect(await screen.findByRole("region", { name: "General" })).toBeInTheDocument();
  });

  it("has no Spaces back button", async () => {
    renderSheet("10001");
    await screen.findByRole("region", { name: "General" });

    expect(screen.queryByRole("button", { name: "Spaces" })).not.toBeInTheDocument();
  });

  it("closes when Done is clicked", async () => {
    renderSheet("10001");

    await userEvent.click(await screen.findByRole("button", { name: "Done" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(useStore.getState().spaceSheet).toBeNull();
  });

  it("closes on Escape", async () => {
    renderSheet("10001");
    await screen.findByRole("dialog", { name: "Space Settings" });

    await userEvent.keyboard("{Escape}");

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(useStore.getState().spaceSheet).toBeNull();
  });
});

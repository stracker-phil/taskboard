import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type SpaceNode } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { lumenSpace, sidebarTree, space } from "../../../test/spaces";
import { Toasts } from "../../Toasts";
import { SectionTarget } from "../controls";
import { SpacesPage } from "./SpacesPage";

const mocked = vi.mocked(api, true);

async function renderPage(spaces: SpaceNode[], target?: string) {
  mocked.sidebarTree.mockResolvedValue(sidebarTree(spaces));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SectionTarget.Provider value={target}>
        <SpacesPage />
      </SectionTarget.Provider>
      <Toasts />
    </QueryClientProvider>,
  );
  await screen.findByRole("region", { name: "Spaces" });
}

const NORTHWIND = () => space({ client: "Northwind" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.spaceMove.mockResolvedValue(undefined);
  mocked.spaceUpdate.mockResolvedValue(undefined);
});

describe("SpacesPage", () => {
  it("points to Projects when no Space is tracked", async () => {
    await renderPage([]);

    expect(await screen.findByText("Track projects under Projects to get Spaces.")).toBeInTheDocument();
  });

  it("lists the Spaces in sidebar order with name, key and client", async () => {
    await renderPage([NORTHWIND(), lumenSpace()]);

    const rows = await screen.findAllByRole("button", { name: /settings$/ });
    expect(rows.map((t) => t.textContent)).toEqual([expect.stringContaining("Website relaunch"), expect.stringContaining("Lumen iOS app")]);
    expect(rows[0]).toHaveTextContent("NW · Northwind");
    expect(rows[1]).toHaveTextContent("LUM");
    expect(rows[1]).not.toHaveTextContent("·");
    expect(screen.queryByText("Track projects under Projects to get Spaces.")).not.toBeInTheDocument();
  });

  describe("opening a Space", () => {
    it("opens the Space's settings when its row is clicked, and returns to the list with Spaces", async () => {
      await renderPage([NORTHWIND(), lumenSpace()]);

      await userEvent.click(await screen.findByRole("button", { name: "Lumen iOS app settings" }));

      expect(await screen.findByRole("heading", { name: "Lumen iOS app" })).toBeInTheDocument();
      expect(await screen.findByRole("region", { name: "Layouts" })).toBeInTheDocument();
      expect(mocked.spaceConfig).toHaveBeenCalledWith("10002");
      expect(screen.queryByRole("button", { name: "Website relaunch settings" })).not.toBeInTheDocument();

      await userEvent.click(screen.getByRole("button", { name: "Spaces" }));

      expect(await screen.findByRole("button", { name: "Website relaunch settings" })).toBeInTheDocument();
      expect(screen.queryByRole("region", { name: "Layouts" })).not.toBeInTheDocument();
    });

    it("saves a changed client for the opened Space", async () => {
      await renderPage([space(), lumenSpace()]);
      await userEvent.click(await screen.findByRole("button", { name: "Lumen iOS app settings" }));

      await userEvent.type(await screen.findByLabelText("Client"), "Contoso");
      await userEvent.tab();

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10002", { name: null, icon: null, color: "1", client: "Contoso" });
    });

    it("opens the Space named by a space:<projectId> target directly", async () => {
      await renderPage([space(), lumenSpace()], "space:10002");

      expect(await screen.findByRole("heading", { name: "Lumen iOS app" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Website relaunch settings" })).not.toBeInTheDocument();
    });

    it("stays on the list for a target naming no tracked Space", async () => {
      await renderPage([space()], "space:99999");

      expect(await screen.findByRole("button", { name: "Website relaunch settings" })).toBeInTheDocument();
    });
  });

  describe("moving", () => {
    it("disables moving the first Space up and the last Space down", async () => {
      await renderPage([space(), lumenSpace()]);

      expect(await screen.findByRole("button", { name: "Move Website relaunch up" })).toBeDisabled();
      expect(screen.getByRole("button", { name: "Move Website relaunch down" })).toBeEnabled();
      expect(screen.getByRole("button", { name: "Move Lumen iOS app up" })).toBeEnabled();
      expect(screen.getByRole("button", { name: "Move Lumen iOS app down" })).toBeDisabled();
    });

    it("moves a Space one position down", async () => {
      await renderPage([space(), lumenSpace()]);

      await userEvent.click(await screen.findByRole("button", { name: "Move Website relaunch down" }));

      expect(mocked.spaceMove).toHaveBeenCalledWith("10001", 1);
    });

    it("moves a Space one position up", async () => {
      await renderPage([space(), lumenSpace()]);

      await userEvent.click(await screen.findByRole("button", { name: "Move Lumen iOS app up" }));

      expect(mocked.spaceMove).toHaveBeenCalledWith("10002", 0);
    });

    it("lists the Spaces in their new order after a move", async () => {
      await renderPage([space(), lumenSpace()]);
      mocked.sidebarTree.mockResolvedValue(sidebarTree([lumenSpace(), space()]));

      await userEvent.click(await screen.findByRole("button", { name: "Move Website relaunch down" }));

      expect(await screen.findByRole("button", { name: "Move Website relaunch up" })).toBeEnabled();
      expect(screen.getByRole("button", { name: "Move Lumen iOS app up" })).toBeDisabled();
    });

    it("shows an error toast when the move is rejected", async () => {
      mocked.spaceMove.mockRejectedValue({ kind: "db", message: "Database is locked" });
      await renderPage([space(), lumenSpace()]);

      await userEvent.click(await screen.findByRole("button", { name: "Move Website relaunch down" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not move Website relaunch: Database is locked");
    });
  });
});

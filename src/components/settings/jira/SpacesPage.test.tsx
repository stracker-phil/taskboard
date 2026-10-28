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
import { SpacesPage } from "./SpacesPage";

const mocked = vi.mocked(api, true);

async function renderPage(spaces: SpaceNode[]) {
  mocked.sidebarTree.mockResolvedValue(sidebarTree(spaces));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SpacesPage />
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

    const toggles = await screen.findAllByRole("button", { expanded: false });
    expect(toggles.map((t) => t.textContent)).toEqual([expect.stringContaining("Website relaunch"), expect.stringContaining("Lumen iOS app")]);
    expect(toggles[0]).toHaveTextContent("NW · Northwind");
    expect(toggles[1]).toHaveTextContent("LUM");
    expect(toggles[1]).not.toHaveTextContent("·");
    expect(screen.queryByText("Track projects under Projects to get Spaces.")).not.toBeInTheDocument();
  });

  describe("row toggle", () => {
    it("shows the Space's fields when opened and hides them when clicked again", async () => {
      await renderPage([NORTHWIND()]);
      const toggle = await screen.findByRole("button", { name: /^Website relaunch/, expanded: false });
      expect(screen.queryByLabelText("Client")).not.toBeInTheDocument();

      await userEvent.click(toggle);

      expect(toggle).toHaveAttribute("aria-expanded", "true");
      expect(screen.getByLabelText("Name")).toBeInTheDocument();
      expect(screen.getByLabelText("Icon letter")).toBeInTheDocument();
      expect(screen.getByRole("group", { name: "Colour" })).toBeInTheDocument();
      expect(await screen.findByLabelText("Client")).toHaveValue("Northwind");

      await userEvent.click(toggle);

      expect(toggle).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByLabelText("Client")).not.toBeInTheDocument();
    });

    it("keeps one Space open at a time", async () => {
      await renderPage([space(), lumenSpace()]);

      await userEvent.click(await screen.findByRole("button", { name: /^Website relaunch/ }));
      await userEvent.click(screen.getByRole("button", { name: /^Lumen iOS app/ }));

      expect(screen.getByRole("button", { name: /^Website relaunch/ })).toHaveAttribute("aria-expanded", "false");
      expect(screen.getAllByLabelText("Client")).toHaveLength(1);
    });

    it("saves a changed client for the opened Space", async () => {
      await renderPage([space(), lumenSpace()]);
      await userEvent.click(await screen.findByRole("button", { name: /^Lumen iOS app/ }));

      await userEvent.type(await screen.findByLabelText("Client"), "Contoso");
      await userEvent.tab();

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10002", { name: null, icon: null, color: "1", client: "Contoso" });
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

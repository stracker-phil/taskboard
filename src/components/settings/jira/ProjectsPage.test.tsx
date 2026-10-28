import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type ProjectRow } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { connectionView, PROJECTS } from "../../../test/jira";
import { ProjectsPage } from "./ProjectsPage";

const mocked = vi.mocked(api, true);

const THREE_PROJECTS: ProjectRow[] = [...PROJECTS, { id: "10003", key: "OPS", name: "Office network upgrade", tracked: false }];

async function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ProjectsPage />
    </QueryClientProvider>,
  );
  await screen.findByRole("region", { name: "Projects" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.connectionGet.mockResolvedValue(connectionView());
  mocked.projectsList.mockResolvedValue([]);
  mocked.projectsRefresh.mockResolvedValue([]);
});

describe("ProjectsPage", () => {
  it("disables Refresh and asks to test the connection while not connected", async () => {
    mocked.connectionGet.mockResolvedValue(connectionView({ accountId: null }));
    await renderPage();

    expect(await screen.findByText("Test the connection first.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Refresh" })).toBeDisabled();
  });

  it("asks to refresh when no project is listed", async () => {
    await renderPage();

    expect(await screen.findByText("Refresh to load your projects.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Refresh" })).toBeEnabled();
  });

  it("says how many of the listed projects are tracked when connected", async () => {
    mocked.projectsList.mockResolvedValue(THREE_PROJECTS);
    await renderPage();

    expect(await screen.findByText(/1 of 3 tracked\./)).toBeInTheDocument();
  });

  it("lists each project with its key and a switch showing whether it is tracked", async () => {
    mocked.projectsList.mockResolvedValue(PROJECTS);
    await renderPage();

    expect(await screen.findByText("Website relaunch")).toBeInTheDocument();
    expect(screen.getByText("NW")).toBeInTheDocument();
    expect(screen.getByText("Lumen iOS app")).toBeInTheDocument();
    expect(screen.getByText("LUM")).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Track Website relaunch" })).toBeChecked();
    expect(screen.getByRole("switch", { name: "Track Lumen iOS app" })).not.toBeChecked();
  });

  it("tracks a project when its switch is turned on", async () => {
    mocked.projectsList.mockResolvedValue(PROJECTS);
    await renderPage();

    await userEvent.click(await screen.findByRole("switch", { name: "Track Lumen iOS app" }));

    expect(mocked.projectTrack).toHaveBeenCalledWith("10002", true);
    expect(screen.getByRole("switch", { name: "Track Lumen iOS app" })).toBeChecked();
  });

  it("stops tracking a project when its switch is turned off", async () => {
    mocked.projectsList.mockResolvedValue(PROJECTS);
    await renderPage();

    await userEvent.click(await screen.findByRole("switch", { name: "Track Website relaunch" }));

    expect(mocked.projectTrack).toHaveBeenCalledWith("10001", false);
  });

  it("shows the projects Jira returns when Refresh is pressed", async () => {
    mocked.projectsRefresh.mockResolvedValue(PROJECTS);
    await renderPage();

    await userEvent.click(await screen.findByRole("button", { name: "Refresh" }));

    expect(await screen.findByText("Website relaunch")).toBeInTheDocument();
    expect(mocked.projectsRefresh).toHaveBeenCalled();
  });

  it("puts the switch back when tracking fails", async () => {
    mocked.projectsList.mockResolvedValue(PROJECTS);
    mocked.projectTrack.mockRejectedValueOnce({ kind: "db", message: "Database is locked" });
    await renderPage();

    await userEvent.click(await screen.findByRole("switch", { name: "Track Lumen iOS app" }));

    await waitFor(() => expect(screen.getByRole("switch", { name: "Track Lumen iOS app" })).not.toBeChecked());
  });

  describe("filter", () => {
    beforeEach(() => {
      mocked.projectsList.mockResolvedValue(THREE_PROJECTS);
    });

    async function filterBy(text: string) {
      await userEvent.type(await screen.findByLabelText("Filter projects"), text);
    }

    it.each([
      ["by name", "lumen", "Lumen iOS app"],
      ["by key", "OPS", "Office network upgrade"],
      ["ignoring case", "wEbSiTe", "Website relaunch"],
      ["with every word matching the name", "network office", "Office network upgrade"],
      ["with words split between name and key", "ios lum", "Lumen iOS app"],
    ])("narrows the rows %s", async (_case, query, expected) => {
      await renderPage();

      await filterBy(query);

      const shown = screen.getAllByRole("switch");
      expect(shown).toHaveLength(1);
      expect(shown[0]).toHaveAccessibleName(`Track ${expected}`);
    });

    it("hides a project when one of the typed words matches nothing", async () => {
      await renderPage();

      await filterBy("lumen relaunch");

      expect(screen.queryAllByRole("switch")).toHaveLength(0);
      expect(screen.getByText("No projects match “lumen relaunch”.")).toBeInTheDocument();
    });

    it("says no project matches when the query finds nothing", async () => {
      await renderPage();

      await filterBy("  zzz ");

      expect(screen.getByText("No projects match “zzz”.")).toBeInTheDocument();
      expect(screen.queryAllByRole("switch")).toHaveLength(0);
    });

    it("shows all projects again when the filter is cleared", async () => {
      await renderPage();
      await filterBy("zzz");

      await userEvent.clear(screen.getByLabelText("Filter projects"));

      expect(screen.getAllByRole("switch")).toHaveLength(3);
      expect(screen.queryByText(/No projects match/)).not.toBeInTheDocument();
    });

    it("is not shown while no project is listed", async () => {
      mocked.projectsList.mockResolvedValue([]);
      await renderPage();

      expect(await screen.findByText("Refresh to load your projects.")).toBeInTheDocument();
      expect(screen.queryByLabelText("Filter projects")).not.toBeInTheDocument();
    });
  });
});

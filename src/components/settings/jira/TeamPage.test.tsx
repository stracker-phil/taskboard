import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type TeamMatrix } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { lumenSpace, sidebarTree, space } from "../../../test/spaces";
import { teamCell, teamMatrix } from "../../../test/team";
import { Toasts } from "../../Toasts";
import { TeamPage } from "./TeamPage";

const mocked = vi.mocked(api, true);

async function renderPage(matrix: TeamMatrix = teamMatrix()) {
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space(), lumenSpace()]));
  mocked.teamMatrix.mockResolvedValue(matrix);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TeamPage />
      <Toasts />
    </QueryClientProvider>,
  );
  if (matrix.people.length === 0) await screen.findByText("People appear after the first sync.");
  else await screen.findByRole("textbox", { name: "Search people" });
}

const box = (name: string) => screen.findByRole("checkbox", { name });
const names = () => screen.getAllByRole("rowheader").map((header) => header.textContent);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("TeamPage", () => {
  describe("matrix", () => {
    it("shows a row per person and a column per tracked Space", async () => {
      await renderPage();
      await box("Alex Morgan on the Website relaunch team");

      expect(names()).toEqual([expect.stringContaining("Alex Morgan"), expect.stringContaining("Sam Lee"), expect.stringContaining("Jordan Rivera")]);
      expect(screen.getByRole("columnheader", { name: /NW/ })).toBeInTheDocument();
      expect(screen.getByRole("columnheader", { name: /LUM/ })).toBeInTheDocument();
    });

    it("checks the boxes of the people who are on a team", async () => {
      await renderPage();

      expect(await box("Alex Morgan on the Website relaunch team")).toBeChecked();
      expect(screen.getByRole("checkbox", { name: "Alex Morgan on the Lumen iOS app team" })).not.toBeChecked();
      expect(screen.getByRole("checkbox", { name: "Sam Lee on the Website relaunch team" })).not.toBeChecked();
      expect(screen.getByRole("checkbox", { name: "Sam Lee on the Lumen iOS app team" })).toBeChecked();
    });

    it("leaves out the columns of Spaces that are not tracked", async () => {
      await renderPage(teamMatrix({ spaces: ["10001", "99999"], people: [{ person: teamMatrix().people[0].person, cells: [teamCell({ active: true }), teamCell()] }] }));
      await box("Alex Morgan on the Website relaunch team");

      expect(screen.getAllByRole("checkbox")).toHaveLength(1);
    });

    it("says people appear after the first sync when the matrix is empty", async () => {
      await renderPage({ spaces: [], people: [] });

      expect(await screen.findByText("People appear after the first sync.")).toBeInTheDocument();
      expect(screen.queryByRole("textbox", { name: "Search people" })).not.toBeInTheDocument();
    });
  });

  describe("changing the team", () => {
    it("adds a person to a team and shows the matrix Rust returns", async () => {
      const saved = teamMatrix();
      saved.people[1].cells[0] = teamCell({ active: true, involved: true });
      mocked.spaceSetMember.mockResolvedValue(saved);
      await renderPage();

      await userEvent.click(await box("Sam Lee on the Website relaunch team"));

      expect(mocked.spaceSetMember).toHaveBeenCalledWith("10001", "demo-account-2", true);
      expect(await screen.findByRole("checkbox", { name: "Sam Lee on the Website relaunch team", checked: true })).toBeInTheDocument();
    });

    it("removes a person from a team", async () => {
      const saved = teamMatrix();
      saved.people[0].cells[0] = teamCell({ assignee: true, involved: true });
      mocked.spaceSetMember.mockResolvedValue(saved);
      await renderPage();

      await userEvent.click(await box("Alex Morgan on the Website relaunch team"));

      expect(mocked.spaceSetMember).toHaveBeenCalledWith("10001", "demo-account-1", false);
      expect(await screen.findByRole("checkbox", { name: "Alex Morgan on the Website relaunch team", checked: false })).toBeInTheDocument();
    });

    it("keeps the order of the rows after a change", async () => {
      const saved = teamMatrix();
      saved.people[2].cells[1] = teamCell({ active: true, involved: true });
      mocked.spaceSetMember.mockResolvedValue(saved);
      await renderPage();
      const before = (await screen.findAllByRole("rowheader")).map((header) => header.textContent);

      await userEvent.click(await box("Jordan Rivera on the Lumen iOS app team"));

      await screen.findByRole("checkbox", { name: "Jordan Rivera on the Lumen iOS app team", checked: true });
      expect(names()).toEqual(before);
    });

    it("shows an error toast and keeps the box as it was when the change fails", async () => {
      mocked.spaceSetMember.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderPage();

      await userEvent.click(await box("Sam Lee on the Website relaunch team"));

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not change Sam Lee in Website relaunch");
      expect(screen.getByRole("checkbox", { name: "Sam Lee on the Website relaunch team" })).not.toBeChecked();
    });
  });

  describe("search", () => {
    it("narrows the rows by name, ignoring case", async () => {
      await renderPage();
      await box("Alex Morgan on the Website relaunch team");

      await userEvent.type(screen.getByRole("textbox", { name: "Search people" }), "sAM");

      expect(names()).toEqual([expect.stringContaining("Sam Lee")]);
    });

    it("says nobody matches when no name does", async () => {
      await renderPage();
      await box("Alex Morgan on the Website relaunch team");

      await userEvent.type(screen.getByRole("textbox", { name: "Search people" }), "zed");

      expect(screen.getByText("Nobody matches “zed”.")).toBeInTheDocument();
      expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
    });
  });

  describe("cell tooltips", () => {
    it.each([
      ["Alex Morgan on the Website relaunch team", "Assigned issues here: on the team unless switched off"],
      ["Sam Lee on the Website relaunch team", "Reported, commented or changed something here"],
      ["Alex Morgan on the Lumen iOS app team", "No activity here"],
    ])("hints at %s: %s", async (label, hint) => {
      await renderPage();

      expect((await box(label)).closest("td")).toHaveAttribute("title", hint);
    });
  });
});

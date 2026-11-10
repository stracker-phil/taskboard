import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type TeamMatrix } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { ALEX, SAM } from "../../../test/issues";
import { connectionView } from "../../../test/jira";
import { JORDAN, teamCell } from "../../../test/team";
import { SpaceTeam } from "./SpaceTeam";

const mocked = vi.mocked(api, true);

const active = teamCell({ active: true, involved: true });

const MATRIX: TeamMatrix = {
  spaces: ["p1", "p2"],
  people: [
    { person: ALEX, cells: [active, teamCell()] },
    { person: SAM, cells: [teamCell({ involved: true }), active] },
    { person: JORDAN, cells: [active, teamCell()] },
  ],
};

async function renderTeam(matrix: TeamMatrix = MATRIX, projectId = "p1") {
  mocked.teamMatrix.mockResolvedValue(matrix);
  mocked.connectionGet.mockResolvedValue(connectionView({ accountId: ALEX.accountId }));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SpaceTeam projectId={projectId} />
    </QueryClientProvider>,
  );
  await screen.findByText(/on the team/);
}

const memberNames = () => within(screen.getByRole("list", { name: "Team members" })).getAllByRole("listitem").map((item) => item.textContent);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("SpaceTeam", () => {
  it("lists the active people of the Space in the matrix's order and leaves out the inactive ones", async () => {
    await renderTeam();

    expect(await screen.findByRole("list", { name: "Team members" })).toBeInTheDocument();
    expect(memberNames()).toEqual([expect.stringContaining("Alex Morgan"), expect.stringContaining("Jordan Rivera")]);
    expect(screen.queryByText("Sam Lee")).not.toBeInTheDocument();
  });

  it("reads the column of the given Space", async () => {
    await renderTeam(MATRIX, "p2");

    expect(await screen.findByText("1 person on the team")).toBeInTheDocument();
    expect(memberNames()).toEqual([expect.stringContaining("Sam Lee")]);
  });

  it("counts the people on the team and those with activity in the Space", async () => {
    await renderTeam();

    expect(await screen.findByText("2 people on the team")).toBeInTheDocument();
    expect(screen.getByText("3 with activity in this Space")).toBeInTheDocument();
  });

  it("marks only the connected account with (you)", async () => {
    await renderTeam();

    await screen.findByRole("list", { name: "Team members" });
    expect(screen.getAllByText("(you)")).toHaveLength(1);
    expect(memberNames()[0]).toContain("(you)");
    expect(memberNames()[1]).not.toContain("(you)");
  });

  it("shows no list and zero people when nobody is active", async () => {
    await renderTeam({ spaces: ["p1"], people: [{ person: SAM, cells: [teamCell({ involved: true })] }] });

    expect(await screen.findByText("0 people on the team")).toBeInTheDocument();
    expect(screen.getByText("1 with activity in this Space")).toBeInTheDocument();
    expect(screen.queryByRole("list", { name: "Team members" })).not.toBeInTheDocument();
  });

  it("shows no list for a Space that is not in the matrix", async () => {
    await renderTeam(MATRIX, "p9");

    expect(await screen.findByText("0 people on the team")).toBeInTheDocument();
    expect(screen.queryByRole("list", { name: "Team members" })).not.toBeInTheDocument();
  });

  it("runs the Team settings command when Edit Team… is pressed", async () => {
    await renderTeam();

    await userEvent.click(screen.getByRole("button", { name: "Edit Team…" }));

    expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:team");
  });
});

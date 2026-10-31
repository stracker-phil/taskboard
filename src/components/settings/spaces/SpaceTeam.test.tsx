import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type TeamMember } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { ALEX, SAM } from "../../../test/issues";
import { Toasts } from "../../Toasts";
import { SpaceTeam } from "./SpaceTeam";

const mocked = vi.mocked(api, true);

const member = (person = ALEX, patch: Partial<Omit<TeamMember, "person">> = {}): TeamMember => ({ person, active: true, assigned: 0, ...patch });

const NAMES = ["Alex Morgan", "Sam Lee", "Robin Park", "Jamie Cole", "Taylor Reed", "Casey Moore", "Drew Hall", "Quinn Ward", "Riley Fox"];
const crowd = (): TeamMember[] =>
  NAMES.map((name, i) => member({ accountId: `demo-account-${i + 1}`, name, initials: name.slice(0, 1), color: 1 }));

async function renderTeam(team: TeamMember[], projectId = "10001") {
  mocked.spaceTeam.mockResolvedValue(team);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SpaceTeam projectId={projectId} />
      <Toasts />
    </QueryClientProvider>,
  );
  await screen.findByRole("region", { name: "Team" });
}

const switchNames = () => screen.getAllByRole("switch").map((s) => s.getAttribute("aria-label"));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("SpaceTeam", () => {
  it("lists the people of the given Space with a switch that is on for active members", async () => {
    await renderTeam([member(ALEX), member(SAM, { active: false })], "10002");

    expect(await screen.findByText("Alex Morgan")).toBeInTheDocument();
    expect(screen.getByText("Sam Lee")).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Alex Morgan is on the team" })).toBeChecked();
    expect(screen.getByRole("switch", { name: "Sam Lee is on the team" })).not.toBeChecked();
    expect(mocked.spaceTeam).toHaveBeenCalledWith("10002");
  });

  it("says people appear after the first sync when there are none", async () => {
    await renderTeam([]);

    expect(screen.getByText("People appear after the first sync.")).toBeInTheDocument();
  });

  it("does not show the empty message when there are people", async () => {
    await renderTeam([member()]);

    await screen.findByText("Alex Morgan");
    expect(screen.queryByText("People appear after the first sync.")).not.toBeInTheDocument();
  });

  it.each([
    [0, "No open issues"],
    [1, "1 open issue"],
    [7, "7 open issues"],
  ])("shows %i assigned issues as %s", async (assigned, text) => {
    await renderTeam([member(ALEX, { assigned })]);

    expect(await screen.findByText(text)).toBeInTheDocument();
  });

  describe("switching a person", () => {
    it("switches a person off for the Space and shows the team Rust returns", async () => {
      await renderTeam([member(ALEX), member(SAM)], "10002");
      mocked.spaceSetMember.mockResolvedValue([member(ALEX, { active: false }), member(SAM)]);

      await userEvent.click(await screen.findByRole("switch", { name: "Alex Morgan is on the team" }));

      expect(mocked.spaceSetMember).toHaveBeenCalledWith("10002", "demo-account-1", false);
      expect(await screen.findByRole("switch", { name: "Alex Morgan is on the team" })).not.toBeChecked();
      expect(screen.getByRole("switch", { name: "Sam Lee is on the team" })).toBeChecked();
    });

    it("switches a person back on", async () => {
      await renderTeam([member(ALEX), member(SAM, { active: false })]);
      mocked.spaceSetMember.mockResolvedValue([member(ALEX), member(SAM)]);

      await userEvent.click(await screen.findByRole("switch", { name: "Sam Lee is on the team" }));

      expect(mocked.spaceSetMember).toHaveBeenCalledWith("10001", "demo-account-2", true);
      expect(await screen.findByRole("switch", { name: "Sam Lee is on the team" })).toBeChecked();
    });

    it("keeps the order of the rows after a person is switched off", async () => {
      await renderTeam([member(ALEX), member(SAM), member({ ...SAM, accountId: "demo-account-3", name: "Robin Park" })]);
      await screen.findByRole("switch", { name: "Robin Park is on the team" });
      const before = switchNames();
      mocked.spaceSetMember.mockResolvedValue([
        member(ALEX),
        member(SAM, { active: false }),
        member({ ...SAM, accountId: "demo-account-3", name: "Robin Park" }),
      ]);

      await userEvent.click(screen.getByRole("switch", { name: "Sam Lee is on the team" }));

      await vi.waitFor(() => expect(screen.getByRole("switch", { name: "Sam Lee is on the team" })).not.toBeChecked());
      expect(switchNames()).toEqual(before);
    });

    it("shows an error toast and keeps the switch when the change fails", async () => {
      await renderTeam([member(ALEX)]);
      mocked.spaceSetMember.mockRejectedValue({ kind: "io", message: "Disk full", code: null });

      await userEvent.click(await screen.findByRole("switch", { name: "Alex Morgan is on the team" }));

      expect(await screen.findByRole("alert")).toHaveTextContent("Could not change Alex Morgan: Disk full");
      expect(screen.getByRole("switch", { name: "Alex Morgan is on the team" })).toBeChecked();
    });
  });

  describe("search", () => {
    it("has no search field for eight people or fewer", async () => {
      await renderTeam(crowd().slice(0, 8));

      await screen.findByText("Alex Morgan");
      expect(screen.queryByRole("textbox", { name: "Search people" })).not.toBeInTheDocument();
    });

    it("offers a search field for more than eight people", async () => {
      await renderTeam(crowd());

      expect(await screen.findByRole("textbox", { name: "Search people" })).toBeInTheDocument();
    });

    it("keeps only the people whose name contains the text, ignoring case", async () => {
      await renderTeam(crowd());

      await userEvent.type(await screen.findByRole("textbox", { name: "Search people" }), "ROBIN");

      expect(screen.getByRole("switch", { name: "Robin Park is on the team" })).toBeInTheDocument();
      expect(screen.getAllByRole("switch")).toHaveLength(1);
      expect(screen.queryByText("Alex Morgan")).not.toBeInTheDocument();
    });

    it("says nobody matches when no name does", async () => {
      await renderTeam(crowd());

      await userEvent.type(await screen.findByRole("textbox", { name: "Search people" }), "zzz");

      expect(screen.getByText("Nobody matches “zzz”.")).toBeInTheDocument();
      expect(screen.queryAllByRole("switch")).toHaveLength(0);
    });

    it("lists everyone again when the search is cleared", async () => {
      await renderTeam(crowd());
      const field = await screen.findByRole("textbox", { name: "Search people" });
      await userEvent.type(field, "robin");

      await userEvent.clear(field);

      expect(within(screen.getByRole("region", { name: "Team" })).getAllByRole("switch")).toHaveLength(9);
      expect(screen.queryByText(/Nobody matches/)).not.toBeInTheDocument();
    });
  });
});

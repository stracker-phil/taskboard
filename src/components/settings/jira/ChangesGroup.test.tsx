import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type Behaviour } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore, useStore } from "../../../state/store";
import { capabilityRows } from "../../../test/capabilities";
import { behaviour, settings } from "../../../test/settings";
import { ChangesGroup } from "./ChangesGroup";

const mocked = vi.mocked(api, true);

async function renderGroup(allowJiraChanges: boolean, patch: Partial<Behaviour> = {}) {
  mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges, ...patch }) }));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ChangesGroup />
    </QueryClientProvider>,
  );
  return screen.findByRole("switch", { name: "Allow changes to Jira" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.connectionChangePermissions.mockReset();
  mocked.connectionChangePermissions.mockResolvedValue([]);
  mocked.behaviourSet.mockImplementation(async (b) => settings({ behaviour: b }));
});

describe("ChangesGroup", () => {
  it("shows the switch off by default", async () => {
    expect(await renderGroup(false)).not.toBeChecked();
  });

  it("shows the switch on when changes are allowed", async () => {
    expect(await renderGroup(true)).toBeChecked();
  });

  it("saves the behaviour with changes allowed when the switch is turned on", async () => {
    const toggle = await renderGroup(false, { seenAfterSeconds: 7, pinDefault: "week" });

    await userEvent.click(toggle);

    expect(mocked.behaviourSet).toHaveBeenCalledWith(behaviour({ seenAfterSeconds: 7, pinDefault: "week", allowJiraChanges: true }));
  });

  it("saves the behaviour with changes disallowed when the switch is turned off", async () => {
    const toggle = await renderGroup(true);

    await userEvent.click(toggle);

    expect(mocked.behaviourSet).toHaveBeenCalledWith(behaviour({ allowJiraChanges: false }));
  });

  it("shows an error toast when the setting cannot be saved", async () => {
    mocked.behaviourSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    const toggle = await renderGroup(false);

    await userEvent.click(toggle);

    await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not save the setting: Disk full"));
  });

  describe("each kind of change", () => {
    it("lists a switch per change with its detail, checked unless the change is disabled", async () => {
      await renderGroup(true, { disabledChanges: ["comment"] });

      expect(await screen.findByRole("switch", { name: "Assign issues" })).toBeChecked();
      expect(screen.getByRole("switch", { name: "Change the status" })).toBeChecked();
      expect(screen.getByRole("switch", { name: "Change the priority" })).toBeChecked();
      expect(screen.getByRole("switch", { name: "Add web links" })).toBeChecked();
      expect(screen.getByRole("switch", { name: "Add comments" })).not.toBeChecked();
      expect(screen.getByText(/Sets or clears the assignee of an issue\./)).toBeInTheDocument();
    });

    it("sends the capability and false when a checked change is switched off", async () => {
      await renderGroup(true);

      await userEvent.click(await screen.findByRole("switch", { name: "Add comments" }));

      expect(mocked.jiraCapabilitySet).toHaveBeenCalledWith("comment", false);
    });

    it("sends the capability and true when a disabled change is switched on", async () => {
      await renderGroup(true, { disabledChanges: ["addWebLink"] });

      await userEvent.click(await screen.findByRole("switch", { name: "Add web links" }));

      expect(mocked.jiraCapabilitySet).toHaveBeenCalledWith("addWebLink", true);
    });

    it("shows an error toast naming the change when it cannot be saved", async () => {
      mocked.jiraCapabilitySet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderGroup(true);

      await userEvent.click(await screen.findByRole("switch", { name: "Assign issues" }));

      await vi.waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not change “Assign issues”: Disk full"));
    });

    it("disables every change switch while changes to Jira are off", async () => {
      await renderGroup(false);

      for (const name of ["Assign issues", "Change the status", "Change the priority", "Rename issues", "Add comments", "Add web links"]) {
        expect(await screen.findByRole("switch", { name })).toBeDisabled();
      }
    });

    it("enables the change switches while changes to Jira are on", async () => {
      await renderGroup(true);

      expect(await screen.findByRole("switch", { name: "Assign issues" })).toBeEnabled();
    });
  });

  describe("projects where Jira doesn't allow a change", () => {
    const note = /Jira doesn't allow it in/;

    it("names the projects under the matching change and under no other", async () => {
      mocked.connectionChangePermissions.mockResolvedValue([{ capability: "comment", projects: ["NW", "OPS"] }]);
      await renderGroup(true);

      const shown = await screen.findByText("Jira doesn't allow it in NW, OPS");

      expect(screen.getAllByText(note)).toHaveLength(1);
      expect(shown.parentElement).toHaveTextContent(capabilityRows().find((row) => row.capability === "comment")!.detail);
    });

    it("neither asks Jira nor shows a note while changes are off", async () => {
      mocked.connectionChangePermissions.mockResolvedValue([{ capability: "comment", projects: ["NW"] }]);
      await renderGroup(false);
      await screen.findByRole("switch", { name: "Add comments" });

      expect(mocked.connectionChangePermissions).not.toHaveBeenCalled();
      expect(screen.queryByText(note)).not.toBeInTheDocument();
    });

    it("shows no note when Jira allows every change", async () => {
      await renderGroup(true);
      await screen.findByRole("switch", { name: "Add comments" });

      expect(screen.queryByText(note)).not.toBeInTheDocument();
    });

    it("shows no note and keeps the rows when the answer fails", async () => {
      mocked.connectionChangePermissions.mockRejectedValue({ kind: "network", message: "Offline", code: null });
      await renderGroup(true);

      expect(await screen.findByRole("switch", { name: "Add comments" })).toBeEnabled();
      await vi.waitFor(() => expect(mocked.connectionChangePermissions).toHaveBeenCalled());
      expect(screen.queryByText(note)).not.toBeInTheDocument();
    });
  });

  describe("what Taskboard reads", () => {
    it("lists the read rows with their detail and no switch", async () => {
      await renderGroup(true);

      expect(await screen.findByText("Your account")).toBeInTheDocument();
      expect(screen.getByText(/Reads the issues of the projects you track\./)).toBeInTheDocument();
      expect(screen.queryByRole("switch", { name: "Your account" })).not.toBeInTheDocument();
      expect(screen.queryByRole("switch", { name: "Issues" })).not.toBeInTheDocument();
    });

    it("keeps the reads out of the change switches", async () => {
      await renderGroup(true);

      await screen.findByRole("switch", { name: "Assign issues" });

      expect(screen.getAllByRole("switch")).toHaveLength(7);
    });
  });

  describe("token scopes", () => {
    const rows = capabilityRows();

    it.each(rows.map((row) => [row.label, row.detail, row.scope]))("shows the scope of “%s” after its detail", async (_label, detail, scope) => {
      await renderGroup(true);

      const description = await screen.findByText(new RegExp(`^${detail.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`));

      expect(description).toHaveTextContent(`${detail} · ${scope}`);
    });

    it("lists the optional scopes of the changes in the Changes description", async () => {
      await renderGroup(true);

      expect(await screen.findByText(/Optional for a scoped token:/)).toHaveTextContent(
        "Each change is sent to Jira once. Optional for a scoped token: write:jira-work, only while changes are on.",
      );
    });

    it("lists the required scopes of the reads once each, in row order, in the reads description", async () => {
      await renderGroup(true);

      expect(await screen.findByText(/Required for a scoped token:/)).toHaveTextContent(
        "Always on: Taskboard reads only these from Jira. Required for a scoped token: read:jira-user, read:jira-work.",
      );
    });

    it("derives both descriptions from the scopes of the rows", async () => {
      mocked.jiraCapabilities.mockResolvedValue([
        { ...rows[0], scope: "read:example-first" },
        { ...rows[1], scope: "read:example-second" },
        { ...rows[5], scope: "write:example-one" },
        { ...rows[6], scope: "write:example-two" },
      ]);
      await renderGroup(true);

      expect(await screen.findByText(/Optional for a scoped token:/)).toHaveTextContent("write:example-one, write:example-two, only while");
      expect(screen.getByText(/Required for a scoped token:/)).toHaveTextContent("read:example-first, read:example-second.");
    });
  });
});

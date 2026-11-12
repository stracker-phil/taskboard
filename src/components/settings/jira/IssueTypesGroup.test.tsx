import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore, useStore } from "../../../state/store";
import { defaultSync, settings } from "../../../test/settings";
import { IssueTypesGroup } from "./IssueTypesGroup";

const mocked = vi.mocked(api, true);

const TYPES = ["Bug", "Story", "Test", "Test Execution"];

async function renderGroup(excludedTypes: string[] = []) {
  mocked.getSettings.mockResolvedValue(settings({ sync: { ...defaultSync(), excludedTypes } }));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <IssueTypesGroup />
    </QueryClientProvider>,
  );
  return screen.findByRole("button", { name: "Leave out" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.syncIssueTypes.mockResolvedValue(TYPES);
});

describe("IssueTypesGroup", () => {
  it("reads None while no type is left out", async () => {
    const field = await renderGroup();

    expect(field).toHaveTextContent("None");
  });

  it("names the types that are left out", async () => {
    const field = await renderGroup(["Test", "Test Execution"]);

    expect(field).toHaveTextContent("Test, Test Execution");
  });

  it("offers the types of the synced issues to choose from", async () => {
    const field = await renderGroup(["Story"]);

    await userEvent.click(field);

    expect(screen.getByRole("option", { name: "Story" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getAllByRole("option").map((o) => o.textContent)).toEqual(["Story", "Bug", "Test", "Test Execution"]);
  });

  it("saves the sync settings with a ticked type added, in the choices' order", async () => {
    const field = await renderGroup(["Test Execution"]);
    await userEvent.click(field);

    await userEvent.click(screen.getByRole("option", { name: "Bug" }));

    await waitFor(() => expect(mocked.syncSave).toHaveBeenCalledWith({ ...defaultSync(), excludedTypes: ["Bug", "Test Execution"] }));
  });

  it("saves the sync settings without a type that is unticked", async () => {
    const field = await renderGroup(["Bug", "Test"]);
    await userEvent.click(field);

    await userEvent.click(screen.getByRole("option", { name: "Bug" }));

    await waitFor(() => expect(mocked.syncSave).toHaveBeenCalledWith({ ...defaultSync(), excludedTypes: ["Test"] }));
  });

  it("shows the saved types in the field", async () => {
    const field = await renderGroup();
    await userEvent.click(field);

    await userEvent.click(screen.getByRole("option", { name: "Test" }));

    await waitFor(() => expect(field).toHaveTextContent("Test"));
    expect(field).not.toHaveTextContent("None");
  });

  it("shows an error toast when the types cannot be saved", async () => {
    mocked.syncSave.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
    const field = await renderGroup();
    await userEvent.click(field);

    await userEvent.click(screen.getByRole("option", { name: "Story" }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not save the issue types: Disk full"));
    expect(field).toHaveTextContent("None");
  });
});

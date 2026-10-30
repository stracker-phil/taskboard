import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type Behaviour } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore, useStore } from "../../../state/store";
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
});

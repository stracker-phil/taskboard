import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type MeRule } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import { behaviour, settings } from "../../test/settings";
import { MenuHost } from "../menu";
import { MeModeGroup } from "./MeModeGroup";

const mocked = vi.mocked(api, true);

const REPORTER: MeRule = { rule: { kind: "field", field: "reporter" }, on: true };
const MENTIONED: MeRule = { rule: { kind: "mentioned" }, on: false };
const SUBTASKS: MeRule = { rule: { kind: "subtasks" }, on: true };

async function renderGroup(meRules: MeRule[] = [REPORTER, MENTIONED, SUBTASKS]) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <MeModeGroup settings={settings({ behaviour: behaviour({ meRules }) })} />
      <MenuHost />
    </QueryClientProvider>,
  );
  await screen.findByRole("switch", { name: "Count Reporter" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.meUserFields.mockResolvedValue([
    { field: "reporter", name: "Reporter" },
    { field: "customfield_10071", name: "Reviewer" },
  ]);
});

describe("MeModeGroup", () => {
  it("shows a switch per rule at its saved position", async () => {
    await renderGroup();

    expect(screen.getByRole("switch", { name: "Count Reporter" })).toBeChecked();
    expect(screen.getByRole("switch", { name: "Count Mentioned in a comment" })).not.toBeChecked();
    expect(screen.getByRole("switch", { name: "Count Subtasks of my issues" })).toBeChecked();
  });

  it("sends a switch edit for the clicked rule", async () => {
    await renderGroup();

    await userEvent.click(screen.getByRole("switch", { name: "Count Mentioned in a comment" }));

    expect(mocked.meRuleEdit).toHaveBeenCalledWith({ kind: "switch", rule: { kind: "mentioned" }, on: true });
  });

  it("offers Remove on field rules only", async () => {
    await renderGroup();

    expect(screen.getByRole("button", { name: "Remove Reporter from Me mode" })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /^Remove / })).toHaveLength(1);
  });

  it("sends a removeField edit for the removed field", async () => {
    await renderGroup();

    await userEvent.click(screen.getByRole("button", { name: "Remove Reporter from Me mode" }));

    expect(mocked.meRuleEdit).toHaveBeenCalledWith({ kind: "removeField", field: "reporter" });
  });

  it("lists only the user fields not yet added under Add Field…", async () => {
    await renderGroup();

    await userEvent.click(screen.getByRole("button", { name: "Add Field…" }));

    expect(await screen.findByRole("menuitem", { name: "Reviewer" })).toBeInTheDocument();
    expect(screen.queryByRole("menuitem", { name: "Reporter" })).not.toBeInTheDocument();
  });

  it("sends an addField edit for the chosen field", async () => {
    await renderGroup();
    await userEvent.click(screen.getByRole("button", { name: "Add Field…" }));

    await userEvent.click(await screen.findByRole("menuitem", { name: "Reviewer" }));

    expect(mocked.meRuleEdit).toHaveBeenCalledWith({ kind: "addField", field: "customfield_10071" });
  });

  it("says there are no other user fields when all are added", async () => {
    await renderGroup([REPORTER, { rule: { kind: "field", field: "customfield_10071" }, on: true }]);

    await userEvent.click(screen.getByRole("button", { name: "Add Field…" }));

    expect(await screen.findByRole("menuitem", { name: "No other user fields" })).toHaveAttribute("aria-disabled", "true");
  });

  it("shows an error toast when the rules cannot be saved", async () => {
    mocked.meRuleEdit.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    await renderGroup();

    await userEvent.click(screen.getByRole("switch", { name: "Count Mentioned in a comment" }));

    await waitFor(() => expect(useStore.getState().toasts.map((t) => t.kind)).toContain("error"));
  });
});

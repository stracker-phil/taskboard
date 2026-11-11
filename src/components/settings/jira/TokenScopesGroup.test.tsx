import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type Behaviour, type TokenKind } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { capabilityRows } from "../../../test/capabilities";
import { behaviour, settings } from "../../../test/settings";
import { TokenScopesGroup } from "./TokenScopesGroup";

const mocked = vi.mocked(api, true);

async function renderGroup(patch: Partial<Behaviour> = {}, tokenKind: TokenKind = "scoped") {
  mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour(patch) }));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <TokenScopesGroup tokenKind={tokenKind} />
    </QueryClientProvider>,
  );
  return screen.findByRole("region", { name: "Token scopes" });
}

/** The settings row of a scope, so its need can be read next to its code; rows have no role of their own. */
function rowOf(group: HTMLElement, scope: string) {
  return within(group).getByText(scope).closest(".settings-row") as HTMLElement;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.jiraCapabilities.mockResolvedValue(capabilityRows());
});

describe("TokenScopesGroup", () => {
  it("lists each scope once, in the order the capabilities first use them", async () => {
    const group = await renderGroup();

    const codes = within(group).getAllByText(/^(read|write):/).map((el) => el.textContent);

    expect(codes).toEqual(["read:jira-user", "read:jira-work", "write:jira-work"]);
  });

  it("names every capability that uses a scope", async () => {
    const group = await renderGroup();

    expect(within(group).getByText("Used for: Your account.")).toBeInTheDocument();
    expect(within(group).getByText("Used for: Projects, Issues, Issue details, Search checks.")).toBeInTheDocument();
    expect(
      within(group).getByText("Used for: Assign issues, Change the status, Change the priority, Rename issues, Add comments, Add web links."),
    ).toBeInTheDocument();
  });

  it("marks the scopes of the reads as Required whatever the changes are set to", async () => {
    const group = await renderGroup({ allowJiraChanges: false });

    expect(within(rowOf(group, "read:jira-user")).getByText("Required")).toBeInTheDocument();
    expect(within(rowOf(group, "read:jira-work")).getByText("Required")).toBeInTheDocument();
  });

  it("marks a scope Required when a read shares it with changes", async () => {
    const rows = capabilityRows();
    mocked.jiraCapabilities.mockResolvedValue([rows[0], { ...rows[5], scope: "read:jira-user" }]);
    const group = await renderGroup({ allowJiraChanges: true });

    expect(within(group).getByText("Required")).toBeInTheDocument();
    expect(within(group).queryByText("Needed for changes")).not.toBeInTheDocument();
  });

  describe("the scope of the changes", () => {
    const writeScope = "write:jira-work";

    it("is Optional while changes to Jira are off", async () => {
      const group = await renderGroup({ allowJiraChanges: false });

      expect(within(rowOf(group, writeScope)).getByText("Optional")).toBeInTheDocument();
    });

    it("is Needed for changes while changes are on and a change using it is enabled", async () => {
      const group = await renderGroup({ allowJiraChanges: true });

      expect(within(rowOf(group, writeScope)).getByText("Needed for changes")).toBeInTheDocument();
    });

    it("stays Needed for changes while only one of its changes is enabled", async () => {
      const group = await renderGroup({
        allowJiraChanges: true,
        disabledChanges: ["assign", "changeStatus", "changePriority", "changeTitle", "comment"],
      });

      expect(within(rowOf(group, writeScope)).getByText("Needed for changes")).toBeInTheDocument();
    });

    it("is Optional when every change using it is disabled one by one", async () => {
      const group = await renderGroup({
        allowJiraChanges: true,
        disabledChanges: ["assign", "changeStatus", "changePriority", "changeTitle", "comment", "addWebLink"],
      });

      expect(within(rowOf(group, writeScope)).getByText("Optional")).toBeInTheDocument();
    });

    it("is Needed for changes only for the scope whose changes are still enabled", async () => {
      const rows = capabilityRows();
      mocked.jiraCapabilities.mockResolvedValue([
        { ...rows[5], scope: "write:example-one" },
        { ...rows[6], scope: "write:example-two" },
      ]);
      const group = await renderGroup({ allowJiraChanges: true, disabledChanges: ["changeStatus"] });

      expect(within(rowOf(group, "write:example-one")).getByText("Needed for changes")).toBeInTheDocument();
      expect(within(rowOf(group, "write:example-two")).getByText("Optional")).toBeInTheDocument();
    });
  });

  describe("description", () => {
    it("tells a scoped token to be given these scopes", async () => {
      await renderGroup({}, "scoped");

      expect(screen.getByText("Give your scoped token these scopes.")).toBeInTheDocument();
    });

    it("says the scopes are only for a scoped token when the token is classic", async () => {
      await renderGroup({}, "classic");

      expect(screen.getByText("Only for a scoped token; a classic token acts with all of your account's access.")).toBeInTheDocument();
      expect(screen.queryByText("Give your scoped token these scopes.")).not.toBeInTheDocument();
    });
  });

  it("shows nothing while the capabilities load", async () => {
    mocked.jiraCapabilities.mockReturnValue(new Promise(() => {}));
    mocked.getSettings.mockResolvedValue(settings());
    render(
      <QueryClientProvider client={createQueryClient()}>
        <TokenScopesGroup tokenKind="scoped" />
      </QueryClientProvider>,
    );

    await vi.waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());

    expect(screen.queryByText("Token scopes")).not.toBeInTheDocument();
  });
});

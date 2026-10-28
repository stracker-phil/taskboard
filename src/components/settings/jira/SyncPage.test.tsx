import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { SyncPage } from "./SyncPage";

const mocked = vi.mocked(api, true);

async function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SyncPage />
    </QueryClientProvider>,
  );
  await screen.findByLabelText("While Taskboard is in front");
  await screen.findByRole("textbox", { name: "Query" });
}

async function setQuery(text: string) {
  await userEvent.clear(screen.getByRole("textbox", { name: "Query" }));
  await userEvent.type(screen.getByRole("textbox", { name: "Query" }), text);
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.scopeGet.mockResolvedValue({ jql: "project = NW ORDER BY updated DESC", isDefault: false });
});

describe("SyncPage", () => {
  describe("Scope", () => {
    it("shows the saved query", async () => {
      await renderPage();

      expect(screen.getByRole("textbox", { name: "Query" })).toHaveValue("project = NW ORDER BY updated DESC");
    });

    it("explains the default query while the scope is the default", async () => {
      mocked.scopeGet.mockResolvedValue({ jql: "project in (NW)", isDefault: true });
      await renderPage();

      expect(screen.getByText("Default: open work plus done issues of the last 90 days in tracked projects.")).toBeInTheDocument();
    });

    it("confirms a query Jira accepts", async () => {
      await renderPage();

      await userEvent.click(screen.getByRole("button", { name: "Validate" }));

      expect(await screen.findByRole("status")).toHaveTextContent("Jira accepts this query.");
      expect(mocked.scopeValidate).toHaveBeenCalledWith("project = NW ORDER BY updated DESC");
    });

    it("shows Jira's message under the field when the query is invalid", async () => {
      mocked.scopeValidate.mockRejectedValueOnce({ kind: "invalid_jql", message: "Invalid JQL: Field 'foo' does not exist" });
      await renderPage();
      await setQuery("foo = 1");

      await userEvent.click(screen.getByRole("button", { name: "Validate" }));

      expect(await screen.findByText("Invalid JQL: Field 'foo' does not exist")).toBeInTheDocument();
      expect(screen.getByRole("textbox", { name: "Query" })).toHaveAttribute("aria-invalid", "true");
    });

    it("cannot validate an empty query", async () => {
      await renderPage();

      await userEvent.clear(screen.getByRole("textbox", { name: "Query" }));

      expect(screen.getByRole("button", { name: "Validate" })).toBeDisabled();
    });

    it("keeps Save disabled until the query changed", async () => {
      await renderPage();
      expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();

      await setQuery("project = LUM");

      expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
    });

    it("saves the edited query", async () => {
      await renderPage();
      await setQuery("project = LUM");

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      await waitFor(() => expect(mocked.scopeSave).toHaveBeenCalledWith("project = LUM"));
      await waitFor(() => expect(screen.getByRole("button", { name: "Save" })).toBeDisabled());
    });

    it("shows Jira's message when saving is refused as invalid JQL", async () => {
      mocked.scopeSave.mockRejectedValueOnce({ kind: "invalid_jql", message: "Invalid JQL: unexpected end" });
      await renderPage();
      await setQuery("project =");

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(await screen.findByText("Invalid JQL: unexpected end")).toBeInTheDocument();
    });

    it("resets to the default scope with a null query", async () => {
      await renderPage();

      await userEvent.click(screen.getByRole("button", { name: "Reset to Default" }));

      await waitFor(() => expect(mocked.scopeSave).toHaveBeenCalledWith(null));
      await waitFor(() => expect(screen.queryByRole("button", { name: "Reset to Default" })).not.toBeInTheDocument());
    });

    it("offers no reset while the scope is already the default", async () => {
      mocked.scopeGet.mockResolvedValue({ jql: "project in (NW)", isDefault: true });
      await renderPage();

      expect(screen.queryByRole("button", { name: "Reset to Default" })).not.toBeInTheDocument();
    });
  });

  describe("Intervals", () => {
    it("shows the saved intervals", async () => {
      await renderPage();

      expect(screen.getByLabelText("While Taskboard is in front")).toHaveValue(5);
      expect(screen.getByLabelText("In the background")).toHaveValue(15);
      expect(screen.getByLabelText("Keep done issues")).toHaveValue(90);
    });

    it("saves a changed value with the other values unchanged", async () => {
      await renderPage();

      await userEvent.clear(screen.getByLabelText("While Taskboard is in front"));
      await userEvent.type(screen.getByLabelText("While Taskboard is in front"), "10");
      await userEvent.tab();

      await waitFor(() =>
        expect(mocked.syncSave).toHaveBeenCalledWith({ scopeOverride: null, focusedMinutes: 10, backgroundMinutes: 15, keepDoneDays: 90 }),
      );
    });

    it.each([
      ["While Taskboard is in front", "0", 5],
      ["In the background", "4", 15],
      ["Keep done issues", "400", 90],
    ])("does not save %s when set to %s, which is out of bounds", async (label, typed, saved) => {
      await renderPage();

      await userEvent.clear(screen.getByLabelText(label));
      await userEvent.type(screen.getByLabelText(label), typed);
      await userEvent.tab();

      expect(mocked.syncSave).not.toHaveBeenCalled();
      expect(screen.getByLabelText(label)).toHaveValue(saved);
    });

    it("shows Rust's field error under the interval it names", async () => {
      mocked.syncSave.mockRejectedValueOnce({
        kind: "invalid_input",
        message: "sync.focusedMinutes: too small",
        field: { path: "sync.focusedMinutes", message: "Use 1 to 60 minutes" },
      });
      await renderPage();

      await userEvent.clear(screen.getByLabelText("While Taskboard is in front"));
      await userEvent.type(screen.getByLabelText("While Taskboard is in front"), "10");
      await userEvent.tab();

      expect(await screen.findByText("Use 1 to 60 minutes")).toBeInTheDocument();
    });
  });

  describe("Cache", () => {
    it("asks before resetting and says what stays", async () => {
      await renderPage();

      await userEvent.click(screen.getByRole("button", { name: "Reset Cache" }));

      const dialog = screen.getByRole("alertdialog", { name: "Reset the cache?" });
      expect(dialog).toHaveTextContent("Spaces, filter lists, views and pins stay");
      expect(mocked.cacheReset).not.toHaveBeenCalled();
    });

    it("closes without resetting when cancelled", async () => {
      await renderPage();
      await userEvent.click(screen.getByRole("button", { name: "Reset Cache" }));

      await userEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      expect(mocked.cacheReset).not.toHaveBeenCalled();
    });

    it("resets the cache when confirmed", async () => {
      await renderPage();
      await userEvent.click(screen.getByRole("button", { name: "Reset Cache" }));

      await userEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Reset Cache" }));

      expect(mocked.cacheReset).toHaveBeenCalled();
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    });
  });
});

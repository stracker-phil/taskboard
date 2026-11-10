import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type FieldChoiceRow } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { fieldChoice } from "../../../test/spaces";
import { Toasts } from "../../Toasts";
import { CustomFieldsPage } from "./CustomFieldsPage";

const mocked = vi.mocked(api, true);

const REVIEWER = fieldChoice();
const POINTS = fieldChoice({ id: "customfield_10072", name: "Story points", kind: "number", inList: true });
const SPRINT = fieldChoice({ id: "customfield_10073", name: "Release train", kind: "option", inDetail: true });

async function renderPage(rows: FieldChoiceRow[]) {
  mocked.fieldChoices.mockResolvedValue(rows);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <CustomFieldsPage />
      <Toasts />
    </QueryClientProvider>,
  );
  await screen.findByRole("region", { name: "Custom fields" });
}

const rowNames = () => screen.getAllByRole("rowheader").map((h) => h.textContent);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("CustomFieldsPage", () => {
  it("says custom fields appear after the first sync when there are none", async () => {
    await renderPage([]);

    expect(await screen.findByText("Custom fields appear after the first sync.")).toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "Search fields" })).not.toBeInTheDocument();
  });

  it("shows each field with its Jira type and a box per place, ticked as the app has it", async () => {
    await renderPage([REVIEWER, POINTS, SPRINT]);

    expect(await screen.findByRole("checkbox", { name: "Reviewer in the list" })).not.toBeChecked();
    expect(rowNames()).toEqual(["Revieweruser", "Story pointsnumber", "Release trainoption"]);
    expect(screen.getByRole("checkbox", { name: "Reviewer in the list" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Reviewer in the Issue view" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Story points in the list" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Story points in the Issue view" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Release train in the Issue view" })).toBeChecked();
  });

  it.each([
    ["the list is unticked", "Story points in the list", false, false],
    ["the Issue view is ticked", "Story points in the Issue view", true, true],
  ])("when %s, it sends both places and shows the rows Rust returns", async (_case, name, inList, inDetail) => {
    mocked.fieldChoiceSet.mockResolvedValue([REVIEWER, { ...POINTS, inList, inDetail }]);
    await renderPage([REVIEWER, POINTS]);

    await userEvent.click(await screen.findByRole("checkbox", { name }));

    expect(mocked.fieldChoiceSet).toHaveBeenCalledWith("customfield_10072", inList, inDetail);
    await waitFor(() => expect(screen.getByRole<HTMLInputElement>("checkbox", { name }).checked).toBe(name.endsWith("view")));
  });

  it("keeps the rows in their order after a box is ticked", async () => {
    mocked.fieldChoiceSet.mockResolvedValue([REVIEWER, { ...POINTS, inDetail: true }, SPRINT]);
    await renderPage([REVIEWER, POINTS, SPRINT]);
    const toggle = await screen.findByRole("checkbox", { name: "Story points in the Issue view" });
    const before = rowNames();

    await userEvent.click(toggle);

    await waitFor(() => expect(screen.getByRole("checkbox", { name: "Story points in the Issue view" })).toBeChecked());
    expect(rowNames()).toEqual(before);
  });

  it("shows an error toast and leaves the box when the change fails", async () => {
    mocked.fieldChoiceSet.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    await renderPage([REVIEWER]);

    await userEvent.click(await screen.findByRole("checkbox", { name: "Reviewer in the list" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not change Reviewer: Disk full");
    expect(screen.getByRole("checkbox", { name: "Reviewer in the list" })).not.toBeChecked();
  });

  describe("search", () => {
    it("narrows the rows to the fields whose name contains the text, ignoring case", async () => {
      await renderPage([REVIEWER, POINTS, SPRINT]);
      await screen.findByRole("checkbox", { name: "Reviewer in the list" });

      await userEvent.type(screen.getByRole("textbox", { name: "Search fields" }), "STORY");

      expect(rowNames()).toEqual(["Story pointsnumber"]);
    });

    it("still saves the right field from the narrowed rows", async () => {
      mocked.fieldChoiceSet.mockResolvedValue([REVIEWER, POINTS, SPRINT]);
      await renderPage([REVIEWER, POINTS, SPRINT]);
      await userEvent.type(await screen.findByRole("textbox", { name: "Search fields" }), "release");

      await userEvent.click(screen.getByRole("checkbox", { name: "Release train in the list" }));

      expect(mocked.fieldChoiceSet).toHaveBeenCalledWith("customfield_10073", true, true);
    });

    it("says no field matches when nothing does", async () => {
      await renderPage([REVIEWER, POINTS]);

      await userEvent.type(await screen.findByRole("textbox", { name: "Search fields" }), "zzz");

      expect(screen.getByText("No field matches “zzz”.")).toBeInTheDocument();
      expect(screen.queryByRole("table")).not.toBeInTheDocument();
    });
  });
});

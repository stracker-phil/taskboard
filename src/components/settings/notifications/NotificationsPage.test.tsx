import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type Notifications } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore, useStore } from "../../../state/store";
import { notifications, settings } from "../../../test/settings";
import { NotificationsPage } from "./NotificationsPage";

const mocked = vi.mocked(api, true);

const TYPES = ["Bug", "Story", "Task", "Test"];

async function renderPage(current: Notifications = notifications()) {
  mocked.getSettings.mockResolvedValue(settings({ notifications: current }));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <NotificationsPage />
    </QueryClientProvider>,
  );
  return screen.findByRole("radio", { name: "All issues in my Spaces" });
}

const box = (name: string) => screen.getByRole("checkbox", { name });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.notificationIssueTypes.mockResolvedValue(TYPES);
  mocked.notificationPeopleFields.mockResolvedValue([]);
  mocked.notificationsSave.mockImplementation(async (n) => n);
});

describe("NotificationsPage", () => {
  describe("Which issues", () => {
    it("shows the saved scope selected", async () => {
      await renderPage(notifications({ scope: "involved" }));

      expect(screen.getByRole("radio", { name: "Only issues I'm involved in" })).toBeChecked();
      expect(screen.getByRole("radio", { name: "All issues in my Spaces" })).not.toBeChecked();
    });

    it("saves the involved scope when its radio is picked", async () => {
      await renderPage();

      await userEvent.click(screen.getByRole("radio", { name: "Only issues I'm involved in" }));

      expect(mocked.notificationsSave).toHaveBeenCalledWith(notifications({ scope: "involved" }));
      expect(screen.getByRole("radio", { name: "Only issues I'm involved in" })).toBeChecked();
    });

    it.each(["Assigned to me", "Created by me", "Named in a people field"])("disables %s under all issues", async (label) => {
      await renderPage(notifications({ scope: "allIssues" }));

      expect(box(label)).toBeDisabled();
    });

    it.each(["Assigned to me", "Created by me", "Named in a people field"])("enables %s under involved", async (label) => {
      await renderPage(notifications({ scope: "involved" }));

      expect(box(label)).toBeEnabled();
    });

    it("saves only the toggled involvement tick", async () => {
      await renderPage(notifications({ scope: "involved" }));

      await userEvent.click(box("Created by me"));

      expect(mocked.notificationsSave).toHaveBeenCalledWith(
        notifications({ scope: "involved", involvement: { assigned: true, reported: false, peopleFields: true } }),
      );
      expect(box("Created by me")).not.toBeChecked();
    });

    describe("ignore automations", () => {
      const AUTOMATIONS = "Ignore changes made by automations";

      it("shows ticked by default", async () => {
        await renderPage();

        expect(box(AUTOMATIONS)).toBeChecked();
      });

      it("shows unticked when the setting is off", async () => {
        await renderPage(notifications({ ignoreAutomations: false }));

        expect(box(AUTOMATIONS)).not.toBeChecked();
      });

      it.each(["allIssues", "involved"] as const)("is enabled under the %s scope", async (scope) => {
        await renderPage(notifications({ scope }));

        expect(box(AUTOMATIONS)).toBeEnabled();
      });

      it("saves false with the rest unchanged when unticked, and true when ticked again", async () => {
        await renderPage();

        await userEvent.click(box(AUTOMATIONS));

        expect(mocked.notificationsSave).toHaveBeenLastCalledWith(notifications({ ignoreAutomations: false }));
        await waitFor(() => expect(box(AUTOMATIONS)).not.toBeChecked());

        await userEvent.click(box(AUTOMATIONS));

        expect(mocked.notificationsSave).toHaveBeenLastCalledWith(notifications({ ignoreAutomations: true }));
        await waitFor(() => expect(box(AUTOMATIONS)).toBeChecked());
      });
    });

    describe("people-field hint", () => {
      it("lists the names of the opted-in people fields", async () => {
        mocked.notificationPeopleFields.mockResolvedValue([
          { id: "customfield_10100", name: "Reviewer" },
          { id: "customfield_10101", name: "Approver" },
        ]);
        await renderPage();

        expect(await screen.findByText("Reviewer, Approver")).toBeInTheDocument();
      });

      it("points to Custom fields when none is opted in, and the link opens that page", async () => {
        await renderPage();

        expect(await screen.findByText(/Opt in a user field under/)).toHaveTextContent("Opt in a user field under Custom fields to use it.");
        await userEvent.click(screen.getByRole("button", { name: "Custom fields" }));

        expect(mocked.runCommand).toHaveBeenCalledWith("app:settings:fields");
      });

      it("shows an error toast when the Custom fields settings cannot be opened", async () => {
        mocked.runCommand.mockRejectedValue({ kind: "io", message: "No window", code: null });
        await renderPage();

        await userEvent.click(await screen.findByRole("button", { name: "Custom fields" }));

        await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not open the Custom fields settings: No window"));
      });
    });

    describe("issue types", () => {
      const picker = () => screen.getByRole("button", { name: "Issue types" });
      const pickerShows = (summary: string) => waitFor(() => expect(picker()).toHaveTextContent(new RegExp(`^${summary}$`)));

      it("shows the summary of the excluded types on the picker", async () => {
        await renderPage(notifications({ excludedTypes: ["Test"] }));

        await pickerShows("All except Test");
      });

      it("shows All types on the picker when nothing is excluded", async () => {
        await renderPage();

        await pickerShows("All types");
      });

      it("saves a type in excludedTypes when it is unticked", async () => {
        await renderPage();
        await pickerShows("All types");

        await userEvent.click(picker());
        await userEvent.click(screen.getByRole("option", { name: "Task" }));

        expect(mocked.notificationsSave).toHaveBeenCalledWith(notifications({ excludedTypes: ["Task"] }));
        await pickerShows("All except Task");
      });

      it("removes a type from excludedTypes when it is ticked again", async () => {
        await renderPage(notifications({ excludedTypes: ["Task", "Test"] }));
        await pickerShows("All except Task, Test");

        await userEvent.click(picker());
        expect(screen.getByRole("option", { name: "Task" })).toHaveAttribute("aria-selected", "false");
        await userEvent.click(screen.getByRole("option", { name: "Task" }));

        expect(mocked.notificationsSave).toHaveBeenCalledWith(notifications({ excludedTypes: ["Test"] }));
      });

      it("counts a type that came later as ticked", async () => {
        await renderPage(notifications({ excludedTypes: ["Test"] }));

        await pickerShows("All except Test");
        await userEvent.click(picker());

        expect(screen.getByRole("option", { name: "Story" })).toHaveAttribute("aria-selected", "true");
        expect(screen.getByRole("option", { name: "Test" })).toHaveAttribute("aria-selected", "false");
      });
    });
  });

  describe("Notify me about", () => {
    const ABOUT_YOU = ["Mentions me", "Assigns me", "Unassigns me", "Adds me to a people field"];
    const ON_THESE = ["New comment", "Moved to Done", "Reopened (back to To Do)", "Other status change", "Due date or priority changed", "Issue created"];

    it("lists every event under its section with an In app and a macOS box", async () => {
      await renderPage();

      const table = screen.getByRole("table");
      const rows = within(table).getAllByRole("row").slice(1).map((r) => within(r).getByRole("rowheader").textContent);
      expect(rows).toEqual(["About you", ...ABOUT_YOU, "On these issues", ...ON_THESE]);
      expect(within(table).getAllByRole("columnheader").map((h) => h.textContent)).toEqual(["Change", "In app", "macOS"]);
      for (const event of [...ABOUT_YOU, ...ON_THESE]) {
        expect(box(`${event} in app`)).toBeInTheDocument();
        expect(box(`${event} on macOS`)).toBeInTheDocument();
      }
    });

    it("ticks the boxes the settings list for each channel", async () => {
      await renderPage(notifications({ inApp: ["newComment"], mac: ["mentionsMe", "created"] }));

      expect(box("New comment in app")).toBeChecked();
      expect(box("New comment on macOS")).not.toBeChecked();
      expect(box("Mentions me in app")).not.toBeChecked();
      expect(box("Mentions me on macOS")).toBeChecked();
      expect(box("Issue created on macOS")).toBeChecked();
    });

    it("saves the event in the macOS list only when its macOS box is ticked, in page order", async () => {
      await renderPage(notifications({ mac: ["mentionsMe", "created"] }));

      await userEvent.click(box("Unassigns me on macOS"));

      expect(mocked.notificationsSave).toHaveBeenCalledWith(notifications({ mac: ["mentionsMe", "unassignsMe", "created"] }));
    });

    it("removes the event from the in-app list only when its in-app box is unticked", async () => {
      await renderPage();

      await userEvent.click(box("Moved to Done in app"));

      expect(mocked.notificationsSave).toHaveBeenCalledWith(
        notifications({ inApp: ["mentionsMe", "assignsMe", "unassignsMe", "addsMeToPeopleField", "newComment", "reopened", "otherChange", "dueOrPriority", "created"] }),
      );
    });

    it("keeps every tick when several follow before Rust answers", async () => {
      mocked.notificationsSave.mockImplementation(() => new Promise(() => {}));
      await renderPage(notifications({ inApp: [], mac: [] }));

      await userEvent.click(box("Issue created on macOS"));
      await userEvent.click(box("New comment on macOS"));
      await userEvent.click(box("Mentions me on macOS"));

      expect(mocked.notificationsSave).toHaveBeenLastCalledWith(notifications({ inApp: [], mac: ["mentionsMe", "newComment", "created"] }));
      expect(box("Issue created on macOS")).toBeChecked();
      expect(box("New comment on macOS")).toBeChecked();
      expect(box("Mentions me on macOS")).toBeChecked();
    });

    it("restores the boxes and shows an error toast when the save fails", async () => {
      mocked.notificationsSave.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderPage();

      await userEvent.click(box("New comment on macOS"));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not save the notification settings: Disk full"));
      expect(box("New comment on macOS")).not.toBeChecked();
    });
  });
});

import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { WebLinks } from "../issue/WebLinks";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { behaviour, settings } from "../test/settings";
import { webLink } from "../test/webLinks";

const mocked = vi.mocked(api, true);

const allowChanges = (on = true) => mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: on }) }));
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

function renderLinks(variant: "issue" | "peek" = "issue") {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <WebLinks id="10042" variant={variant} />
    </QueryClientProvider>,
  );
}

const addButton = () => screen.findByRole("button", { name: "Add link" });
const address = () => screen.findByRole("textbox", { name: "Link address" });
const submit = () => screen.getByRole("button", { name: "Add Link" });

async function openForm() {
  renderLinks();
  await userEvent.click(await addButton());
  return address();
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  allowChanges();
  mocked.issueWebLinks.mockResolvedValue([]);
  mocked.issueSyncWebLinks.mockResolvedValue(false);
  mocked.issueAddWebLink.mockResolvedValue({ done: "Added the link to NW-1042", reverse: null });
});

describe("Add link", () => {
  describe("availability", () => {
    it("shows the Web links section with an Add link button in the Issue view even without links", async () => {
      renderLinks();

      expect(await addButton()).toBeInTheDocument();
      expect(screen.getByRole("region", { name: "Web links" })).toBeInTheDocument();
    });

    it("offers Add link next to existing links", async () => {
      mocked.issueWebLinks.mockResolvedValue([webLink()]);
      renderLinks();

      expect(await addButton()).toBeInTheDocument();
      expect(await screen.findByRole("button", { name: /Postcode guide/ })).toBeInTheDocument();
    });

    it("has no button and no section in the peek when there are no links", async () => {
      renderLinks("peek");
      await waitFor(() => expect(mocked.issueWebLinks).toHaveBeenCalled());

      expect(screen.queryByRole("button", { name: "Add link" })).not.toBeInTheDocument();
      expect(screen.queryByRole("region", { name: "Web links" })).not.toBeInTheDocument();
    });

    it("has no button in the peek next to existing links", async () => {
      mocked.issueWebLinks.mockResolvedValue([webLink()]);
      renderLinks("peek");

      expect(await screen.findByRole("button", { name: /Postcode guide/ })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Add link" })).not.toBeInTheDocument();
    });

    it("has no button and no section while changes to Jira are off and there are no links", async () => {
      allowChanges(false);
      renderLinks();
      await waitFor(() => expect(mocked.getSettings).toHaveBeenCalled());
      await waitFor(() => expect(mocked.issueWebLinks).toHaveBeenCalled());

      expect(screen.queryByRole("button", { name: "Add link" })).not.toBeInTheDocument();
      expect(screen.queryByRole("region", { name: "Web links" })).not.toBeInTheDocument();
    });

    it("has no button while adding links is switched off, but still lists the links", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, disabledChanges: ["addWebLink"] }) }));
      mocked.issueWebLinks.mockResolvedValue([webLink()]);
      renderLinks();

      expect(await screen.findByRole("button", { name: /Postcode guide/ })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Add link" })).not.toBeInTheDocument();
    });

    it("keeps the button when only another kind of change is switched off", async () => {
      mocked.getSettings.mockResolvedValue(settings({ behaviour: behaviour({ allowJiraChanges: true, disabledChanges: ["comment"] }) }));
      renderLinks();

      expect(await addButton()).toBeInTheDocument();
    });

    it("has no button while changes to Jira are off, but still lists the links", async () => {
      allowChanges(false);
      mocked.issueWebLinks.mockResolvedValue([webLink()]);
      renderLinks();

      expect(await screen.findByRole("button", { name: /Postcode guide/ })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Add link" })).not.toBeInTheDocument();
    });
  });

  describe("form", () => {
    it("opens with the address field focused when Add link is clicked", async () => {
      const field = await openForm();

      expect(screen.getByRole("button", { name: "Add link" })).toHaveAttribute("aria-expanded", "true");
      await waitFor(() => expect(field).toHaveFocus());
      expect(screen.getByRole("textbox", { name: "Link title" })).toBeInTheDocument();
    });

    it("keeps Add Link disabled until the address has text beyond whitespace", async () => {
      const field = await openForm();
      expect(submit()).toBeDisabled();

      await userEvent.type(field, "   ");
      expect(submit()).toBeDisabled();

      await userEvent.type(field, "https://github.com/example/shop/pull/481");
      expect(submit()).toBeEnabled();
    });

    it("keeps typed keys from reaching the page's shortcuts", async () => {
      const page = vi.fn();
      const field = await openForm();
      document.addEventListener("keydown", page);

      await userEvent.type(field, "j");

      document.removeEventListener("keydown", page);
      expect(page).not.toHaveBeenCalled();
    });
  });

  describe("adding", () => {
    it("sends the trimmed address with a null title on Enter, confirms, and closes the form", async () => {
      mocked.issueAddWebLink.mockResolvedValue({ done: "Added the link to NW-1042", reverse: null });
      const field = await openForm();

      await userEvent.type(field, "  https://github.com/example/shop/pull/481  {Enter}");

      expect(mocked.issueAddWebLink).toHaveBeenCalledWith("10042", "https://github.com/example/shop/pull/481", null);
      await waitFor(() => expect(toasts()).toEqual([["success", "Added the link to NW-1042"]]));
      await waitFor(() => expect(screen.queryByRole("textbox", { name: "Link address" })).not.toBeInTheDocument());
      expect(screen.getByRole("button", { name: "Add link" })).toHaveAttribute("aria-expanded", "false");
    });

    it("sends the trimmed title when one is typed, using the Add Link button", async () => {
      const field = await openForm();

      await userEvent.type(field, "https://docs.example.com/postcodes");
      await userEvent.type(screen.getByRole("textbox", { name: "Link title" }), "  Postcode guide ");
      await userEvent.click(submit());

      expect(mocked.issueAddWebLink).toHaveBeenCalledWith("10042", "https://docs.example.com/postcodes", "Postcode guide");
    });

    it("starts with empty fields when the form is opened again after adding", async () => {
      const field = await openForm();
      await userEvent.type(field, "https://docs.example.com/postcodes{Enter}");
      await waitFor(() => expect(screen.queryByRole("textbox", { name: "Link address" })).not.toBeInTheDocument());

      await userEvent.click(screen.getByRole("button", { name: "Add link" }));

      expect(await address()).toHaveValue("");
    });

    it("does not send when Enter is pressed with an empty address", async () => {
      const field = await openForm();

      await userEvent.type(field, "{Enter}");

      expect(mocked.issueAddWebLink).not.toHaveBeenCalled();
    });

    it("keeps the text and shows an error toast when adding fails", async () => {
      mocked.issueAddWebLink.mockRejectedValue({ kind: "auth", message: "No permission to link", code: null });
      const field = await openForm();

      await userEvent.type(field, "https://docs.example.com/postcodes{Enter}");

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not add the link: No permission to link"]]));
      expect(screen.getByRole("textbox", { name: "Link address" })).toHaveValue("https://docs.example.com/postcodes");
      expect(field).toBeEnabled();
      expect(submit()).toBeEnabled();
    });
  });

  describe("closing", () => {
    it.each([
      ["Escape", async () => userEvent.keyboard("{Escape}")],
      ["Cancel", async () => userEvent.click(screen.getByRole("button", { name: "Cancel" }))],
    ])("closes without calling the API on %s", async (_name, close) => {
      const field = await openForm();
      await userEvent.type(field, "https://docs.example.com/postcodes");

      await close();

      await waitFor(() => expect(screen.queryByRole("textbox", { name: "Link address" })).not.toBeInTheDocument());
      expect(mocked.issueAddWebLink).not.toHaveBeenCalled();
      expect(toasts()).toEqual([]);
    });

    it("clears the typed text when closed and opened again", async () => {
      const field = await openForm();
      await userEvent.type(field, "https://docs.example.com/postcodes");
      await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
      await waitFor(() => expect(screen.queryByRole("textbox", { name: "Link address" })).not.toBeInTheDocument());

      await userEvent.click(screen.getByRole("button", { name: "Add link" }));

      expect(await address()).toHaveValue("");
    });
  });
});

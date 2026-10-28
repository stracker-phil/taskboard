import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { connectionView, PROJECTS } from "../../../test/jira";
import { ConnectionPage } from "./ConnectionPage";

const mocked = vi.mocked(api, true);

async function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ConnectionPage />
    </QueryClientProvider>,
  );
  await screen.findByRole("region", { name: "Jira site" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.connectionGet.mockResolvedValue(connectionView());
  mocked.projectsList.mockResolvedValue([]);
  mocked.projectsRefresh.mockResolvedValue([]);
});

describe("ConnectionPage", () => {
  describe("API token", () => {
    it("says the token is saved in the Keychain and leaves the field empty", async () => {
      await renderPage();

      expect(screen.getByText(/Saved in Keychain\. Type a new one to replace it/)).toBeInTheDocument();
      expect(screen.getByLabelText("API token")).toHaveValue("");
    });

    it("does not mention the Keychain when no token is saved", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({}, false));
      await renderPage();

      expect(screen.queryByText(/Saved in Keychain\. Type/)).not.toBeInTheDocument();
      expect(screen.getByLabelText("API token")).toHaveAttribute("placeholder", "Paste your API token");
    });

    it("clears the typed token once the connection is saved", async () => {
      await renderPage();

      await userEvent.type(screen.getByLabelText("API token"), "demo-token-123");
      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      await waitFor(() => expect(screen.getByLabelText("API token")).toHaveValue(""));
    });
  });

  describe("Save", () => {
    it("sends the edited form and a null token when none was typed", async () => {
      await renderPage();

      await userEvent.clear(screen.getByLabelText("Email"));
      await userEvent.type(screen.getByLabelText("Email"), "sam.lee@example.com");
      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.connectionSave).toHaveBeenCalledWith(
        { siteUrl: "https://example.atlassian.net/", email: "sam.lee@example.com", tokenKind: "classic", tokenExpires: null, cloudId: null },
        null,
      );
    });

    it("shows a field error under the Site URL field", async () => {
      mocked.connectionSave.mockRejectedValueOnce({
        kind: "invalid_input",
        message: "siteUrl: Use your Jira Cloud address",
        field: { path: "siteUrl", message: "Use your Jira Cloud address, like https://example.atlassian.net" },
      });
      await renderPage();

      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(await screen.findByText("Use your Jira Cloud address, like https://example.atlassian.net")).toBeInTheDocument();
      expect(screen.getByLabelText("Site URL")).toHaveAttribute("aria-invalid", "true");
    });

  });

  describe("Test Connection", () => {
    it("saves the form with the typed token, then tests, and reports who is connected", async () => {
      await renderPage();

      await userEvent.type(screen.getByLabelText("API token"), "demo-token-123");
      await userEvent.click(screen.getByRole("button", { name: "Test Connection" }));

      expect(await screen.findByText("Connected as Alex Morgan · Europe/Vienna")).toBeInTheDocument();
      expect(mocked.connectionSave).toHaveBeenCalledWith(
        { siteUrl: "https://example.atlassian.net/", email: "alex.morgan@example.com", tokenKind: "classic", tokenExpires: null, cloudId: null },
        "demo-token-123",
      );
      expect(mocked.connectionSave.mock.invocationCallOrder[0]).toBeLessThan(mocked.connectionTest.mock.invocationCallOrder[0]);
    });

    it("shows Jira's message when the credentials are refused", async () => {
      mocked.connectionTest.mockRejectedValueOnce({ kind: "auth", message: "Jira didn't accept the email and API token" });
      await renderPage();

      await userEvent.click(screen.getByRole("button", { name: "Test Connection" }));

      expect(await screen.findByRole("status")).toHaveTextContent("Jira didn't accept the email and API token");
    });

    it("shows a field error from the save under its field and does not test", async () => {
      mocked.connectionSave.mockRejectedValueOnce({
        kind: "invalid_input",
        message: "siteUrl: bad",
        field: { path: "siteUrl", message: "Use your Jira Cloud address, like https://example.atlassian.net" },
      });
      await renderPage();

      await userEvent.click(screen.getByRole("button", { name: "Test Connection" }));

      expect(await screen.findByText("Use your Jira Cloud address, like https://example.atlassian.net")).toBeInTheDocument();
      expect(mocked.connectionTest).not.toHaveBeenCalled();
    });

    it("clears the result line when a field is edited", async () => {
      await renderPage();
      await userEvent.click(screen.getByRole("button", { name: "Test Connection" }));
      await screen.findByText(/Connected as/);

      await userEvent.type(screen.getByLabelText("Email"), "x");

      expect(screen.queryByText(/Connected as/)).not.toBeInTheDocument();
    });
  });

  describe("Cloud ID", () => {
    it("is hidden for a classic token", async () => {
      await renderPage();

      expect(screen.queryByText("Cloud ID")).not.toBeInTheDocument();
    });

    it("shows the saved value read-only for a scoped token", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ tokenKind: "scoped", cloudId: "demo-cloud-id-1" }));
      await renderPage();

      expect(screen.getByText("demo-cloud-id-1")).toBeInTheDocument();
      expect(screen.queryByRole("textbox", { name: "Cloud ID" })).not.toBeInTheDocument();
    });

    it("appears when the token kind is switched to Scoped", async () => {
      await renderPage();

      await userEvent.click(screen.getByRole("button", { name: "Scoped" }));

      expect(screen.getByRole("button", { name: "Scoped" })).toHaveAttribute("aria-pressed", "true");
      expect(screen.getByRole("textbox", { name: "Cloud ID" })).toHaveValue("");
    });

    it("becomes editable when Test Connection cannot find it", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ tokenKind: "scoped", cloudId: "demo-cloud-id-1" }));
      mocked.connectionTest.mockRejectedValueOnce({ kind: "not_found", message: "Jira did not tell the cloud ID" });
      await renderPage();

      await userEvent.click(screen.getByRole("button", { name: "Test Connection" }));

      expect(await screen.findByRole("textbox", { name: "Cloud ID" })).toHaveValue("demo-cloud-id-1");
    });

    it("is sent with the form when typed in", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ tokenKind: "scoped" }));
      await renderPage();

      await userEvent.type(screen.getByRole("textbox", { name: "Cloud ID" }), "demo-cloud-id-2");
      await userEvent.click(screen.getByRole("button", { name: "Save" }));

      expect(mocked.connectionSave).toHaveBeenCalledWith(expect.objectContaining({ tokenKind: "scoped", cloudId: "demo-cloud-id-2" }), null);
    });
  });

  describe("Token expiry", () => {
    it.each([
      [{ state: "fine" } as const, "Taskboard warns 14 days ahead."],
      [{ state: "soon", days: 3 } as const, "Expires in 3 days"],
      [{ state: "soon", days: 1 } as const, "Expires in 1 day"],
      [{ state: "soon", days: 0 } as const, "Expires today"],
      [{ state: "expired" } as const, "Expired; create a new token"],
    ])("describes %j as %s", async (expiry, text) => {
      mocked.connectionGet.mockResolvedValue(connectionView({}, true, expiry));
      await renderPage();

      expect(screen.getByText(text)).toBeInTheDocument();
    });

    it("saves a typed expiry date on blur", async () => {
      await renderPage();

      await userEvent.type(screen.getByLabelText("Expires on"), "2027-01-31");
      await userEvent.tab();

      await waitFor(() =>
        expect(mocked.connectionSave).toHaveBeenCalledWith(
          { siteUrl: "https://example.atlassian.net/", email: "alex.morgan@example.com", tokenKind: "classic", tokenExpires: "2027-01-31", cloudId: null },
          null,
        ),
      );
    });

    it("shows a field error of the expiry save under the field", async () => {
      mocked.connectionSave.mockRejectedValueOnce({
        kind: "invalid_input",
        message: "tokenExpires: bad",
        field: { path: "tokenExpires", message: "Use a date like 2027-01-31" },
      });
      await renderPage();

      await userEvent.type(screen.getByLabelText("Expires on"), "2027-01-31");
      await userEvent.tab();

      expect(await screen.findByText("Use a date like 2027-01-31")).toBeInTheDocument();
    });
  });

  describe("Projects", () => {
    it("does not list projects; they live on the Projects page", async () => {
      mocked.projectsList.mockResolvedValue(PROJECTS);
      await renderPage();

      expect(screen.queryByRole("button", { name: "Refresh" })).not.toBeInTheDocument();
      expect(screen.queryByRole("switch", { name: /^Track / })).not.toBeInTheDocument();
    });
  });
});

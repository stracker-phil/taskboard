import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api, type DiagnosticRow, type Diagnostics } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { settingsPages } from "../../../registry/settingsPages";
import { resetStore } from "../../../state/store";
import { Toasts } from "../../Toasts";
import { StatusPage } from "./StatusPage";

const mocked = vi.mocked(api, true);

const row = (patch: Partial<DiagnosticRow> = {}): DiagnosticRow => ({
  label: "Connection",
  value: "Connected as Alex Morgan",
  tone: "ok",
  command: null,
  commandLabel: null,
  ...patch,
});

function renderPage(diagnostics: Diagnostics) {
  mocked.diagnosticsGet.mockResolvedValue(diagnostics);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <StatusPage />
      <Toasts />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("StatusPage", () => {
  it("shows a titled group per diagnostic group with each row's label and value", async () => {
    renderPage({
      groups: [
        { title: "Jira", rows: [row(), row({ label: "Token", value: "Expires in 30 days" })] },
        { title: "This Mac", rows: [row({ label: "Notifications", value: "Allowed" })] },
      ],
    });

    const jira = await screen.findByRole("region", { name: "Jira" });
    expect(within(jira).getByText("Connection")).toBeInTheDocument();
    expect(within(jira).getByText("Connected as Alex Morgan")).toBeInTheDocument();
    expect(within(jira).getByText("Token")).toBeInTheDocument();
    expect(within(jira).getByText("Expires in 30 days")).toBeInTheDocument();
    const mac = screen.getByRole("region", { name: "This Mac" });
    expect(within(mac).getByText("Notifications")).toBeInTheDocument();
    expect(within(mac).queryByText("Token")).not.toBeInTheDocument();
  });

  it("shows nothing when there are no groups", async () => {
    renderPage({ groups: [] });

    await vi.waitFor(() => expect(mocked.diagnosticsGet).toHaveBeenCalled());
    expect(screen.queryByRole("region")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it.each([
    ["ok", "OK"],
    ["warn", "Warning"],
    ["error", "Error"],
    ["info", "Info"],
  ] as const)("marks a %s row with the %s icon", async (tone, name) => {
    renderPage({ groups: [{ title: "Jira", rows: [row({ tone })] }] });

    const group = await screen.findByRole("region", { name: "Jira" });
    expect(within(group).getByRole("img", { name })).toBeInTheDocument();
  });

  describe("command button", () => {
    const withCommand = row({ label: "Notifications", value: "Denied", tone: "warn", command: "system:openNotificationSettings", commandLabel: "Open System Settings" });

    it("runs the row's bus command when its button is pressed", async () => {
      renderPage({ groups: [{ title: "This Mac", rows: [withCommand] }] });

      await userEvent.click(await screen.findByRole("button", { name: "Open System Settings" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("system:openNotificationSettings");
    });

    it("reads the diagnostics again after the command ran", async () => {
      renderPage({ groups: [{ title: "This Mac", rows: [withCommand] }] });
      await screen.findByText("Denied");
      mocked.diagnosticsGet.mockResolvedValue({ groups: [{ title: "This Mac", rows: [{ ...withCommand, value: "Allowed", tone: "ok", command: null, commandLabel: null }] }] });

      await userEvent.click(screen.getByRole("button", { name: "Open System Settings" }));

      expect(await screen.findByText("Allowed")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Open System Settings" })).not.toBeInTheDocument();
    });

    it("shows an error toast naming the command when it fails", async () => {
      mocked.runCommand.mockRejectedValue({ kind: "io", message: "Not allowed", code: null });
      renderPage({ groups: [{ title: "This Mac", rows: [withCommand] }] });

      await userEvent.click(await screen.findByRole("button", { name: "Open System Settings" }));

      expect(await screen.findByText(/Could not run Open System Settings/)).toBeInTheDocument();
    });

    it.each([
      ["a command without a label", { command: "sync:run", commandLabel: null }],
      ["a label without a command", { command: null, commandLabel: "Sync now" }],
      ["neither", {}],
    ])("has no button for %s", async (_name, patch) => {
      renderPage({ groups: [{ title: "Jira", rows: [row(patch)] }] });

      await screen.findByRole("region", { name: "Jira" });
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });
  });

  it("shows an alert with the message when the diagnostics cannot be read", async () => {
    mocked.diagnosticsGet.mockRejectedValue({ kind: "io", message: "Database is locked", code: null });
    render(
      <QueryClientProvider client={createQueryClient()}>
        <StatusPage />
      </QueryClientProvider>,
    );

    expect(await screen.findByRole("alert")).toHaveTextContent("Database is locked");
  });
});

describe("Status settings page", () => {
  it("is registered as Status in the System group", () => {
    const page = settingsPages().find((p) => p.id === "status");

    expect(page).toMatchObject({ label: "Status", group: "System" });
  });
});

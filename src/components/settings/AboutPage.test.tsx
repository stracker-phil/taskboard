import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore, useStore } from "../../state/store";
import { AboutPage } from "./AboutPage";

const mocked = vi.mocked(api, true);
const toasts = () => useStore.getState().toasts.map((t) => t.message);

function renderPage() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AboutPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.appInfo.mockResolvedValue({ name: "Taskboard", version: "4.5.6" });
});

describe("AboutPage", () => {
  it("shows the app name and version in the Version row", async () => {
    renderPage();

    expect(await screen.findByText("Taskboard 4.5.6")).toBeInTheDocument();
    expect(screen.getByText("Version")).toBeInTheDocument();
  });

  it("opens the data folder through the app:data command", async () => {
    renderPage();

    await userEvent.click(screen.getByRole("button", { name: "Show Data Folder" }));

    expect(mocked.runCommand).toHaveBeenCalledWith("app:data");
  });

  it("shows an error toast when the data folder cannot be opened", async () => {
    mocked.runCommand.mockRejectedValueOnce({ kind: "io", message: "No such folder", code: null });
    renderPage();

    await userEvent.click(screen.getByRole("button", { name: "Show Data Folder" }));

    await waitFor(() => expect(toasts()).toContain("Could not open the data folder: No such folder"));
  });

  it("opens the log folder from its button", async () => {
    renderPage();

    await userEvent.click(screen.getByRole("button", { name: "Open Log Folder" }));

    expect(mocked.openLogFolder).toHaveBeenCalled();
  });

  it("shows an error toast when the log folder cannot be opened", async () => {
    mocked.openLogFolder.mockRejectedValueOnce({ kind: "io", message: "No such folder", code: null });
    renderPage();

    await userEvent.click(screen.getByRole("button", { name: "Open Log Folder" }));

    await waitFor(() => expect(toasts()).toContain("Could not open the log folder: No such folder"));
  });
});

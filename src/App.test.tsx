import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", async (orig) => ({ ...(await orig<object>()), api: (await import("./test/apiMock")).createApiMock() }));

import App from "./App";
import { api } from "./api";
import { installCommands } from "./commands";
import { resetStore } from "./state/store";
import { emitEvent } from "./test/apiMock";

const mocked = vi.mocked(api, true);

async function renderApp() {
  render(<App />);
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("toggle_sidebar", expect.any(Function)));
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("main");
});

describe("App", () => {
  it("shows the sidebar beside the content panel with the Jira placeholder", async () => {
    await renderApp();

    expect(screen.getByRole("complementary", { name: "Sidebar" })).toBeInTheDocument();
    expect(screen.getByRole("main")).toHaveTextContent("Connect Jira in Settings");
  });

  describe("sidebar", () => {
    it("hides on the toggle_sidebar event and shows again on the next one, saving each state", async () => {
      await renderApp();

      act(() => emitEvent(api, "toggle_sidebar", null));
      expect(screen.queryByRole("complementary", { name: "Sidebar" })).not.toBeInTheDocument();
      expect(mocked.setUiState).toHaveBeenLastCalledWith({ sidebarHidden: true });
      expect(screen.getByRole("main")).toHaveTextContent("Connect Jira in Settings");

      act(() => emitEvent(api, "toggle_sidebar", null));
      expect(screen.getByRole("complementary", { name: "Sidebar" })).toBeInTheDocument();
      expect(mocked.setUiState).toHaveBeenLastCalledWith({ sidebarHidden: false });
    });

    it("starts hidden when the saved layout has the sidebar hidden", async () => {
      mocked.getUiState.mockResolvedValue({ sidebarHidden: true });

      await renderApp();

      await waitFor(() => expect(screen.queryByRole("complementary", { name: "Sidebar" })).not.toBeInTheDocument());
      expect(screen.getByRole("main")).toBeInTheDocument();
    });

    it("runs the view:toggleSidebar bus command on ⌘\\", async () => {
      await renderApp();

      await userEvent.keyboard("{Meta>}\\{/Meta}");

      expect(mocked.runCommand).toHaveBeenCalledWith("view:toggleSidebar");
    });
  });
});

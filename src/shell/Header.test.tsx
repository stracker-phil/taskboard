import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore, useStore } from "../state/store";
import { connectionView } from "../test/jira";
import { space } from "../test/spaces";
import { listNode } from "../test/views";
import { Header } from "./Header";
import type { Level } from "./levels";

const mocked = vi.mocked(api, true);

const SPACE_LEVEL: Level = { kind: "space", space: space() };
const LIST_LEVEL: Level = { kind: "list", space: space(), node: listNode() };
const EVERYTHING: Level = { kind: "home", node: listNode({ id: 1, kind: "everything", name: "Everything" }) };
const PINNED: Level = { kind: "home", node: listNode({ id: 3, kind: "pinned", name: "Pinned" }) };

const recent = (count: number | null): Level => ({ kind: "home", node: listNode({ id: 2, kind: "recent", name: "Recently changed", count }) });

async function renderHeader(level: Level) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <Header level={level} />
    </QueryClientProvider>,
  );
  await waitFor(() => expect(mocked.connectionGet).toHaveBeenCalled());
  await act(async () => {
    await mocked.connectionGet.mock.results[0].value;
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.connectionGet.mockResolvedValue(connectionView());
});

describe("Header", () => {
  describe("title", () => {
    it("shows a Space's name as the heading", async () => {
      await renderHeader(SPACE_LEVEL);

      expect(screen.getByRole("heading", { level: 1, name: "Website relaunch" })).toBeInTheDocument();
    });

    it("shows a filter list as a breadcrumb button with its Space, then its own name as the heading", async () => {
      await renderHeader(LIST_LEVEL);

      expect(screen.getByRole("button", { name: "Website relaunch" })).toBeInTheDocument();
      expect(screen.getByRole("heading", { level: 1, name: "My open issues" })).toBeInTheDocument();
    });

    it("opens the Space's own level when the breadcrumb is clicked", async () => {
      await renderHeader(LIST_LEVEL);

      await userEvent.click(screen.getByRole("button", { name: "Website relaunch" }));

      expect(mocked.navGo).toHaveBeenCalledWith(11, null);
    });

    it.each([
      ["Everything", EVERYTHING],
      ["Pinned", PINNED],
    ])("shows the name of the Home level %s as the heading", async (name, level) => {
      await renderHeader(level);

      expect(screen.getByRole("heading", { level: 1, name })).toBeInTheDocument();
    });
  });

  describe("Mark all seen", () => {
    it("marks every issue of Recently changed seen", async () => {
      await renderHeader(recent(3));

      await userEvent.click(screen.getByRole("button", { name: "Mark all seen" }));

      expect(mocked.listMarkAllSeen).toHaveBeenCalledWith(2);
    });

    it.each([0, null])("is disabled when the unseen count is %s", async (count) => {
      await renderHeader(recent(count));

      expect(screen.getByRole("button", { name: "Mark all seen" })).toBeDisabled();
    });

    it.each([
      ["Everything", EVERYTHING],
      ["a Space", SPACE_LEVEL],
      ["a filter list", LIST_LEVEL],
    ])("is absent on %s", async (_name, level) => {
      await renderHeader(level);

      expect(screen.queryByRole("button", { name: "Mark all seen" })).not.toBeInTheDocument();
    });

    it("shows an error toast when marking fails", async () => {
      mocked.listMarkAllSeen.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
      await renderHeader(recent(3));

      await userEvent.click(screen.getByRole("button", { name: "Mark all seen" }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not mark them seen: Disk full"));
    });
  });

  describe("Open in Jira", () => {
    it.each([
      ["a Space", SPACE_LEVEL],
      ["a filter list", LIST_LEVEL],
    ])("opens the Space's project page without a double slash for %s", async (_name, level) => {
      await renderHeader(level);

      await userEvent.click(await screen.findByRole("button", { name: /Open in Jira/ }));

      expect(mocked.openUrl).toHaveBeenCalledWith("https://example.atlassian.net/browse/NW");
    });

    it.each([
      ["Everything", EVERYTHING],
      ["Pinned", PINNED],
    ])("is hidden on the Home level %s", async (_name, level) => {
      await renderHeader(level);

      expect(screen.queryByRole("button", { name: /Open in Jira/ })).not.toBeInTheDocument();
    });

    it("is hidden while no Jira site is set up", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl: "" }));

      await renderHeader(SPACE_LEVEL);

      expect(screen.queryByRole("button", { name: /Open in Jira/ })).not.toBeInTheDocument();
    });

    it("shows an error toast when Jira cannot be opened", async () => {
      mocked.openUrl.mockRejectedValue({ kind: "io", message: "No browser", code: null });
      await renderHeader(SPACE_LEVEL);

      await userEvent.click(await screen.findByRole("button", { name: /Open in Jira/ }));

      await waitFor(() => expect(useStore.getState().toasts.map((t) => t.message)).toContain("Could not open Jira: No browser"));
    });
  });
});

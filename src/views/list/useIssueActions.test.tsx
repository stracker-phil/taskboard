import { QueryClientProvider } from "@tanstack/react-query";
import { renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { SETTINGS_KEYS } from "../../queries/settings";
import { resetStore, useStore } from "../../state/store";
import { connectionView } from "../../test/jira";
import { useIssueActions } from "./useIssueActions";

const mocked = vi.mocked(api, true);

const URL = "https://example.atlassian.net/browse/NW-1042";
const toasts = () => useStore.getState().toasts.map((t) => [t.kind, t.message]);

async function renderActions(siteUrl: string) {
  mocked.connectionGet.mockResolvedValue(connectionView({ siteUrl }));
  const client = createQueryClient();
  const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  const hook = renderHook(() => useIssueActions(), { wrapper });
  await waitFor(() => expect(client.getQueryState(SETTINGS_KEYS.connection)?.status).toBe("success"));
  return hook.result;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.copyText.mockResolvedValue(undefined);
});

describe("useIssueActions", () => {
  describe("copyKey", () => {
    it("copies the key and says so", async () => {
      const actions = await renderActions("https://example.atlassian.net/");

      actions.current.copyKey("NW-1042");

      await waitFor(() => expect(toasts()).toEqual([["success", "Copied NW-1042"]]));
      expect(mocked.copyText).toHaveBeenCalledWith("NW-1042");
    });

    it("shows an error instead when the copy fails", async () => {
      mocked.copyText.mockRejectedValue(new Error("Clipboard is not available"));
      const actions = await renderActions("https://example.atlassian.net/");

      actions.current.copyKey("NW-1042");

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not copy: Clipboard is not available"]]));
    });

    it("works without a Jira site", async () => {
      const actions = await renderActions("");

      actions.current.copyKey("NW-1042");

      await waitFor(() => expect(mocked.copyText).toHaveBeenCalledWith("NW-1042"));
    });
  });

  describe("copyUrl", () => {
    it("copies the issue's link on the Jira site and says so", async () => {
      const actions = await renderActions("https://example.atlassian.net/");

      actions.current.copyUrl("NW-1042");

      await waitFor(() => expect(toasts()).toEqual([["success", "Copied the link to NW-1042"]]));
      expect(mocked.copyText).toHaveBeenCalledWith(URL);
    });

    it("does nothing without a Jira site", async () => {
      const actions = await renderActions("");

      actions.current.copyUrl("NW-1042");

      expect(mocked.copyText).not.toHaveBeenCalled();
      expect(toasts()).toEqual([]);
    });
  });

  describe("openInJira", () => {
    it("opens the issue's page on the Jira site", async () => {
      const actions = await renderActions("https://example.atlassian.net/");

      actions.current.openInJira?.("NW-1042");

      expect(mocked.openUrl).toHaveBeenCalledWith(URL);
    });

    it("shows an error when Jira cannot be opened", async () => {
      mocked.openUrl.mockRejectedValue({ kind: "invalid_input", message: "Only https links open", code: null });
      const actions = await renderActions("https://example.atlassian.net/");

      actions.current.openInJira?.("NW-1042");

      await waitFor(() => expect(toasts()).toEqual([["error", "Could not open Jira: Only https links open"]]));
    });

    it("is not offered without a Jira site", async () => {
      const actions = await renderActions("");

      expect(actions.current.openInJira).toBeNull();
    });
  });
});

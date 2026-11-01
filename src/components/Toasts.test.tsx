import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { resetStore, useStore } from "../state/store";
import { Toasts } from "./Toasts";

const show = (...args: Parameters<ReturnType<typeof useStore.getState>["toast"]>) => act(() => useStore.getState().toast(...args));
const advance = (ms: number) => act(() => vi.advanceTimersByTimeAsync(ms));

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("Toasts", () => {
  it("renders nothing while there are no toasts", () => {
    render(<Toasts />);

    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows success and info toasts as status messages", () => {
    render(<Toasts />);

    show("success", "Saved the filter");
    show("info", "Sync is paused");

    const statuses = screen.getAllByRole("status");
    expect(statuses).toHaveLength(2);
    expect(statuses[0]).toHaveTextContent("Saved the filter");
    expect(statuses[1]).toHaveTextContent("Sync is paused");
  });

  it("shows an error toast as an alert with a generic title", () => {
    render(<Toasts />);

    show("error", "Could not reach example.com");

    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("Something went wrong");
    expect(alert).toHaveTextContent("Could not reach example.com");
  });

  describe("repeated toasts", () => {
    it("folds an identical info toast into one with a count", () => {
      render(<Toasts />);

      show("info", "Sync is paused");
      show("info", "Sync is paused");
      show("info", "Sync is paused");

      expect(screen.getAllByRole("status")).toHaveLength(1);
      expect(screen.getByRole("status")).toHaveTextContent("×3");
    });

    it("shows the count in the title of a repeated error", () => {
      render(<Toasts />);

      show("error", "Disk full");
      show("error", "Disk full");

      expect(screen.getByRole("alert")).toHaveTextContent("Something went wrong (×2)");
    });
  });

  describe("error details", () => {
    it("strips the 'Unexpected error:' prefix and has no Details button for a single line", () => {
      render(<Toasts />);

      show("error", "Unexpected error: Board not found");

      const alert = screen.getByRole("alert");
      expect(alert).toHaveTextContent("Board not found");
      expect(alert).not.toHaveTextContent("Unexpected error");
      expect(within(alert).queryByRole("button", { name: "Details" })).not.toBeInTheDocument();
    });

    it("hides the lines after the first behind Details, which toggles them", () => {
      render(<Toasts />);
      show("error", "Request failed\nstatus 500\ntrace id 42");

      const alert = screen.getByRole("alert");
      expect(alert).toHaveTextContent("Request failed");
      expect(alert).not.toHaveTextContent("status 500");

      fireEvent.click(within(alert).getByRole("button", { name: "Details" }));
      expect(alert).toHaveTextContent("status 500");
      expect(alert).toHaveTextContent("trace id 42");

      fireEvent.click(within(alert).getByRole("button", { name: "Hide details" }));
      expect(alert).not.toHaveTextContent("status 500");
    });

    it("cuts a long first line to a headline and moves the full text into the details", () => {
      render(<Toasts />);
      const long = `${"a".repeat(200)}END`;
      show("error", long);

      const alert = screen.getByRole("alert");
      expect(alert).toHaveTextContent(`${"a".repeat(157)}…`);
      expect(alert).not.toHaveTextContent("END");

      fireEvent.click(within(alert).getByRole("button", { name: "Details" }));
      expect(alert).toHaveTextContent(long);
    });
  });

  describe("dismissal", () => {
    it("removes a toast when its Dismiss button is clicked, leaving the others", () => {
      render(<Toasts />);
      show("info", "First note");
      show("info", "Second note");

      fireEvent.click(within(screen.getByText("First note").closest("[role=status]") as HTMLElement).getByRole("button", { name: "Dismiss" }));

      expect(screen.queryByText("First note")).not.toBeInTheDocument();
      expect(screen.getByText("Second note")).toBeInTheDocument();
    });

    it.each([
      ["success", 2500],
      ["info", 4000],
      ["error", 8000],
    ] as const)("dismisses a %s toast by itself after %i ms", async (kind, ms) => {
      render(<Toasts />);
      show(kind, "Something to read");

      await advance(ms - 1);
      expect(screen.getByText("Something to read")).toBeInTheDocument();

      await advance(1);
      expect(screen.queryByText("Something to read")).not.toBeInTheDocument();
    });
  });

  describe("with an action", () => {
    it("shows the action button, runs it on click and dismisses the toast", () => {
      const run = vi.fn();
      render(<Toasts />);
      show("success", "Moved to Done", { label: "Change back to To Do", run });

      fireEvent.click(screen.getByRole("button", { name: "Change back to To Do" }));

      expect(run).toHaveBeenCalled();
      expect(screen.queryByText("Moved to Done")).not.toBeInTheDocument();
    });

    it("stays for 10 seconds instead of the kind's usual time", async () => {
      render(<Toasts />);
      show("success", "Moved to Done", { label: "Change back", run: vi.fn() });

      await advance(9999);
      expect(screen.getByText("Moved to Done")).toBeInTheDocument();

      await advance(1);
      expect(screen.queryByText("Moved to Done")).not.toBeInTheDocument();
    });

    it("stays for the action's own time when it sets one", async () => {
      render(<Toasts />);
      show("success", "Moved to Done", { label: "Change back", run: vi.fn(), ttlMs: 30_000 });

      await advance(29_999);
      expect(screen.getByText("Moved to Done")).toBeInTheDocument();

      await advance(1);
      expect(screen.queryByText("Moved to Done")).not.toBeInTheDocument();
    });
  });
});

import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { resetStore, useStore } from "../state/store";
import { useErrorToasts } from "./useErrorToasts";

function rejectUnhandled(reason: unknown) {
  window.dispatchEvent(Object.assign(new Event("unhandledrejection"), { reason }));
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("useErrorToasts", () => {
  it("shows an uncaught window error as an error toast", () => {
    renderHook(() => useErrorToasts());

    window.dispatchEvent(new ErrorEvent("error", { message: "Cannot read properties of undefined" }));

    expect(useStore.getState().toasts).toMatchObject([
      { kind: "error", message: "Unexpected error: Cannot read properties of undefined" },
    ]);
  });

  it.each([
    "ResizeObserver loop completed with undelivered notifications.",
    "ResizeObserver loop limit exceeded",
  ])("ignores the browser notice %j", (message) => {
    renderHook(() => useErrorToasts());

    window.dispatchEvent(new ErrorEvent("error", { message }));

    expect(useStore.getState().toasts).toEqual([]);
  });

  it("still reports an error that merely mentions ResizeObserver", () => {
    renderHook(() => useErrorToasts());

    window.dispatchEvent(new ErrorEvent("error", { message: "Failed in ResizeObserver loop handler" }));

    expect(useStore.getState().toasts).toMatchObject([{ kind: "error" }]);
  });

  it.each([
    { name: "an app error", reason: { kind: "io", message: "Disk full", code: null }, text: "Disk full" },
    { name: "an Error", reason: new Error("Boom"), text: "Boom" },
    { name: "a plain string", reason: "Plain failure", text: "Plain failure" },
  ])("shows an unhandled rejection with $name as an error toast", ({ reason, text }) => {
    renderHook(() => useErrorToasts());

    rejectUnhandled(reason);

    expect(useStore.getState().toasts).toMatchObject([{ kind: "error", message: `Unexpected error: ${text}` }]);
  });

  it("stops reporting once the component unmounts", () => {
    const view = renderHook(() => useErrorToasts());
    view.unmount();

    window.dispatchEvent(new ErrorEvent("error", { message: "Late failure" }));
    rejectUnhandled(new Error("Late rejection"));

    expect(useStore.getState().toasts).toEqual([]);
  });
});

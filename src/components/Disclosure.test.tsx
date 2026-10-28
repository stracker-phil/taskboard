import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { startMotionTimers, stubMotion } from "../test/motion";
import { Disclosure } from "./Disclosure";

let spy: { mockRestore(): void } | undefined;

afterEach(() => {
  spy?.mockRestore();
  spy = undefined;
  vi.useRealTimers();
});

const ui = (open: boolean) => (
  <Disclosure open={open}>
    <p>Details</p>
  </Disclosure>
);

describe("Disclosure", () => {
  it("shows its children, interactive, while open", () => {
    render(ui(true));

    expect(screen.getByText("Details")).toBeInTheDocument();
    expect(screen.getByText("Details").closest("[inert]")).toBeNull();
  });

  it("renders nothing while closed", () => {
    render(ui(false));

    expect(screen.queryByText("Details")).not.toBeInTheDocument();
  });

  it("removes the children at once on close when there is no motion time", () => {
    const { rerender } = render(ui(true));

    rerender(ui(false));

    expect(screen.queryByText("Details")).not.toBeInTheDocument();
  });

  describe("with motion", () => {
    it("keeps the children on close, inert, and removes them once the time has passed", async () => {
      spy = stubMotion({ "--motion-slow": "200ms" });
      startMotionTimers();
      const { rerender } = render(ui(true));

      rerender(ui(false));

      expect(screen.getByText("Details").closest("[inert]")).not.toBeNull();
      await act(() => vi.advanceTimersByTimeAsync(300));
      expect(screen.queryByText("Details")).not.toBeInTheDocument();
    });

    it("makes the children interactive again when reopened during the exit", async () => {
      spy = stubMotion({ "--motion-slow": "200ms" });
      startMotionTimers();
      const { rerender } = render(ui(true));
      rerender(ui(false));

      rerender(ui(true));
      await act(() => vi.advanceTimersByTimeAsync(500));

      expect(screen.getByText("Details").closest("[inert]")).toBeNull();
    });
  });
});

import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { startMotionTimers, stubMotion } from "../test/motion";
import { motionMs, usePresence } from "./motion";

let spy: { mockRestore(): void } | undefined;

afterEach(() => {
  spy?.mockRestore();
  spy = undefined;
  vi.useRealTimers();
});

describe("motionMs", () => {
  it.each([
    ["150ms", 150],
    ["0.2s", 200],
    [" 300ms ", 300],
    ["1s", 1000],
    ["0ms", 0],
    ["", 0],
    ["fast", 0],
  ])("reads %j as %d ms", (value, expected) => {
    spy = stubMotion({ "--motion-base": value });

    expect(motionMs("--motion-base")).toBe(expected);
  });

  it("is 0 for a token the stylesheet does not define", () => {
    expect(motionMs("--motion-slow")).toBe(0);
  });
});

describe("usePresence", () => {
  const render = (shown: boolean) => renderHook(({ shown }) => usePresence(shown, "--motion-base"), { initialProps: { shown } });

  it("is unmounted while hidden and open while shown", () => {
    spy = stubMotion({ "--motion-base": "200ms" });

    expect(render(false).result.current.mounted).toBe(false);
    expect(render(true).result.current).toEqual({ mounted: true, state: "open" });
  });

  it("stays mounted as leaving after it is hidden, then unmounts once the time has passed", async () => {
    spy = stubMotion({ "--motion-base": "200ms" });
    startMotionTimers();
    const { result, rerender } = render(true);

    rerender({ shown: false });

    expect(result.current).toEqual({ mounted: true, state: "leaving" });
    await act(() => vi.advanceTimersByTimeAsync(100));
    expect(result.current.mounted).toBe(true);
    await act(() => vi.advanceTimersByTimeAsync(200));
    expect(result.current.mounted).toBe(false);
  });

  it("returns to open and stays mounted when shown again during the exit", async () => {
    spy = stubMotion({ "--motion-base": "200ms" });
    startMotionTimers();
    const { result, rerender } = render(true);
    rerender({ shown: false });
    await act(() => vi.advanceTimersByTimeAsync(100));

    rerender({ shown: true });
    await act(() => vi.advanceTimersByTimeAsync(500));

    expect(result.current).toEqual({ mounted: true, state: "open" });
  });

  it("unmounts at once when there is no motion time", () => {
    const { result, rerender } = render(true);

    rerender({ shown: false });

    expect(result.current.mounted).toBe(false);
  });
});

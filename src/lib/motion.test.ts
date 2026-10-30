import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { startMotionTimers, stubMotion } from "../test/motion";
import { motionMs, useGlide, usePresence } from "./motion";

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

describe("useGlide", () => {
  const animate = vi.fn();
  let lefts: Record<string, number>;
  let row: HTMLElement;
  let width: number;

  function child(id: string) {
    const element = document.createElement("span");
    Object.defineProperty(element, "offsetLeft", { get: () => lefts[id] });
    return element;
  }

  function renderRow(scope: string) {
    return renderHook(({ scope }) => useGlide({ current: row }, scope), { initialProps: { scope } });
  }

  beforeEach(() => {
    animate.mockClear();
    HTMLElement.prototype.animate = animate;
    lefts = { a: 0, b: 50 };
    row = document.createElement("div");
    width = 300;
    Object.defineProperty(row, "clientWidth", { get: () => width });
    row.append(child("a"), child("b"));
    spy = stubMotion({ "--motion-slow": "200ms" });
    startMotionTimers();
  });

  afterEach(() => {
    delete (HTMLElement.prototype as Partial<HTMLElement>).animate;
  });

  it("glides a child that moved while the scope stays the same", () => {
    const { rerender } = renderRow("view-1");
    lefts.b = 80;

    rerender({ scope: "view-1" });

    expect(animate).toHaveBeenCalledTimes(1);
    expect(animate).toHaveBeenCalledWith([{ transform: "translateX(-30px)" }, { transform: "none" }], expect.objectContaining({ duration: 200 }));
    expect(row).not.toHaveClass("motion-off");
  });

  it("fades in a child that is new while the scope stays the same", () => {
    const { rerender } = renderRow("view-1");
    lefts.c = 100;
    row.append(child("c"));

    rerender({ scope: "view-1" });

    expect(animate).toHaveBeenCalledWith(expect.arrayContaining([expect.objectContaining({ opacity: 0 })]), expect.objectContaining({ duration: 200 }));
  });

  it("neither glides nor fades children when the scope changes, and turns the row's transitions off", () => {
    const { rerender } = renderRow("view-1");
    lefts.b = 80;
    lefts.c = 100;
    row.append(child("c"));

    rerender({ scope: "view-2" });

    expect(animate).not.toHaveBeenCalled();
    expect(row).toHaveClass("motion-off");
  });

  it("turns the row's transitions back on two frames after the scope changed", () => {
    const frames: FrameRequestCallback[] = [];
    const raf = vi.spyOn(globalThis, "requestAnimationFrame").mockImplementation((callback) => frames.push(callback));
    const { rerender } = renderRow("view-1");
    rerender({ scope: "view-2" });

    expect(row).toHaveClass("motion-off");
    frames.shift()!(0);
    expect(row).toHaveClass("motion-off");
    frames.shift()!(16);

    expect(row).not.toHaveClass("motion-off");
    raf.mockRestore();
  });

  it("glides again on the render after a scope change", () => {
    const { rerender } = renderRow("view-1");
    rerender({ scope: "view-2" });
    lefts.b = 80;

    rerender({ scope: "view-2" });

    expect(animate).toHaveBeenCalledTimes(1);
  });

  it("moves children at once when the row's width changed", () => {
    const { rerender } = renderRow("view-1");
    width = 400;
    lefts.b = 80;

    rerender({ scope: "view-1" });

    expect(animate).not.toHaveBeenCalled();
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

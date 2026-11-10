import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { useAutoGrow } from "./useAutoGrow";

function field(metrics: { scrollHeight: number; offsetHeight?: number; clientHeight?: number }) {
  const el = document.createElement("textarea");
  for (const [key, value] of Object.entries(metrics)) {
    Object.defineProperty(el, key, { configurable: true, get: () => value });
  }
  return el;
}

describe("useAutoGrow", () => {
  it("sets the height to the content height", () => {
    const el = field({ scrollHeight: 90 });

    renderHook(() => useAutoGrow({ current: el }, "text"));

    expect(el.style.height).toBe("90px");
  });

  it("adds the border to the content height", () => {
    const el = field({ scrollHeight: 90, offsetHeight: 102, clientHeight: 100 });

    renderHook(() => useAutoGrow({ current: el }, "text"));

    expect(el.style.height).toBe("92px");
  });

  it("refits when the text changes", () => {
    const el = field({ scrollHeight: 40 });
    const ref = { current: el };
    const { rerender } = renderHook(({ value }) => useAutoGrow(ref, value), { initialProps: { value: "a" } });
    Object.defineProperty(el, "scrollHeight", { configurable: true, get: () => 80 });

    rerender({ value: "a\nb" });

    expect(el.style.height).toBe("80px");
  });

  it("does nothing while no element is attached", () => {
    expect(() => renderHook(() => useAutoGrow({ current: null }, "text"))).not.toThrow();
  });
});

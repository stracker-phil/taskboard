import { vi } from "vitest";
import type { MotionToken } from "../lib/motion";

/** Makes the root element's motion tokens read as `tokens` (e.g. `{ "--motion-slow": "200ms" }`); other tokens
 *  stay unset. Restore the returned spy after the test. */
export function stubMotion(tokens: Partial<Record<MotionToken, string>>) {
  const original = window.getComputedStyle.bind(window);
  return vi.spyOn(globalThis, "getComputedStyle").mockImplementation((element, pseudo) => {
    const style = original(element, pseudo);
    if (element !== document.documentElement) return style;
    return { getPropertyValue: (name: string) => tokens[name as MotionToken] ?? style.getPropertyValue(name) } as CSSStyleDeclaration;
  });
}

/** Fake timers that also drive `requestAnimationFrame` and `performance.now`, and keep running on their own
 *  so `userEvent` does not stall. Restore with `vi.useRealTimers()`. */
export function startMotionTimers() {
  vi.useFakeTimers({
    shouldAdvanceTime: true,
    toFake: ["setTimeout", "clearTimeout", "requestAnimationFrame", "cancelAnimationFrame", "performance", "Date"],
  });
}

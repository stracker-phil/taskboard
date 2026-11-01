import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, vi } from "vitest";
import { cleanup, configure } from "@testing-library/react";

// `waitFor`/`findBy` wait longer than the 1 s default: a full parallel run slows the heavy view tests.
configure({ asyncUtilTimeout: 3000 });

afterEach(cleanup);

// The core install (scopes and settings pages), imported lazily so a test file's `vi.mock("../api")` is already in place when the store loads.
beforeEach(async () => {
  const { registerCoreScopes } = await import("./commands");
  const { registerCoreSettingsPages } = await import("./components/settings/pages");
  registerCoreScopes();
  registerCoreSettingsPages();
});

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= ResizeObserverStub as unknown as typeof ResizeObserver;

window.matchMedia ??= ((query: string) => ({
  matches: false,
  media: query,
  onchange: null,
  addEventListener: () => {},
  removeEventListener: () => {},
  addListener: () => {},
  removeListener: () => {},
  dispatchEvent: () => false,
})) as typeof window.matchMedia;

Object.defineProperty(navigator, "clipboard", {
  configurable: true,
  value: { writeText: vi.fn().mockResolvedValue(undefined) },
});

// The app runs on macOS only: `Mod` is ⌘.
Object.defineProperty(navigator, "platform", { configurable: true, value: "MacIntel" });

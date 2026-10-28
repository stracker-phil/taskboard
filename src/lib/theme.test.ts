import { QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import { createElement, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type Appearance } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { settings } from "../test/settings";
import { applyInitialLook, schemeFor, useAppearance } from "./theme";

const mocked = vi.mocked(api, true);
const originalMatchMedia = window.matchMedia;

/** A matchMedia whose dark-mode query starts at `dark` and can be flipped like macOS does. */
function stubSystemScheme(dark: boolean) {
  let matches = dark;
  const handlers = new Set<() => void>();
  window.matchMedia = vi.fn((query: string) => ({
    get matches() {
      return matches;
    },
    media: query,
    addEventListener: (_: string, handler: () => void) => handlers.add(handler),
    removeEventListener: (_: string, handler: () => void) => handlers.delete(handler),
  })) as unknown as typeof window.matchMedia;
  return {
    set(next: boolean) {
      matches = next;
      act(() => handlers.forEach((h) => h()));
    },
  };
}

const scheme = () => document.documentElement.dataset.scheme;

beforeEach(() => {
  vi.clearAllMocks();
  delete document.documentElement.dataset.scheme;
});

afterEach(() => {
  window.matchMedia = originalMatchMedia;
});

describe("schemeFor", () => {
  it.each<[Appearance, boolean, string]>([
    ["system", false, "light"],
    ["system", true, "dark"],
    ["light", false, "light"],
    ["light", true, "light"],
    ["dark", false, "dark"],
    ["dark", true, "dark"],
  ])("with %s appearance and system dark = %s it is %s", (appearance, systemDark, expected) => {
    expect(schemeFor(appearance, systemDark)).toBe(expected);
  });
});

describe("applyInitialLook", () => {
  it.each([
    [false, "light"],
    [true, "dark"],
  ])("with system dark = %s sets the %s scheme on the root element", (dark, expected) => {
    stubSystemScheme(dark);

    applyInitialLook();

    expect(scheme()).toBe(expected);
  });
});

describe("useAppearance", () => {
  function mount(appearance: Appearance | undefined) {
    if (appearance) mocked.getSettings.mockResolvedValue(settings({ appearance }));
    const wrapper = ({ children }: { children: ReactNode }) => createElement(QueryClientProvider, { client: createQueryClient() }, children);
    renderHook(() => useAppearance(), { wrapper });
  }

  it("applies the scheme of the Appearance setting", async () => {
    stubSystemScheme(false);

    mount("dark");

    await waitFor(() => expect(scheme()).toBe("dark"));
  });

  it("follows macOS while the setting is Automatic", async () => {
    const system = stubSystemScheme(false);
    mount("system");
    await waitFor(() => expect(scheme()).toBe("light"));

    system.set(true);
    await waitFor(() => expect(scheme()).toBe("dark"));

    system.set(false);
    await waitFor(() => expect(scheme()).toBe("light"));
  });

  it.each<[Appearance, string]>([
    ["light", "light"],
    ["dark", "dark"],
  ])("ignores a macOS change while the setting is %s", async (appearance, expected) => {
    const system = stubSystemScheme(appearance === "light");
    mount(appearance);
    await waitFor(() => expect(scheme()).toBe(expected));

    system.set(appearance !== "light");

    expect(scheme()).toBe(expected);
  });
});

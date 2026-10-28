import { useEffect, useState } from "react";
import type { Appearance } from "../api";
import { useSettings } from "../queries/settings";

/** The palette's scheme (`palettes/<scheme>.css`). */
export type Scheme = "light" | "dark";

const DARK_QUERY = "(prefers-color-scheme: dark)";

const systemDark = () => window.matchMedia?.(DARK_QUERY).matches ?? false;

/** Automatic follows macOS; Light and Dark are fixed. */
export function schemeFor(appearance: Appearance, darkSystem: boolean): Scheme {
  if (appearance === "system") return darkSystem ? "dark" : "light";
  return appearance;
}

function applyScheme(scheme: Scheme) {
  document.documentElement.dataset.scheme = scheme;
}

/** Sets `data-scheme` on <html> before the first render (from macOS, until the settings load), so the first
 *  paint has colours. */
export function applyInitialLook() {
  applyScheme(systemDark() ? "dark" : "light");
}

/** Keeps the window's scheme in step with the Appearance setting and, on Automatic, with macOS. */
export function useAppearance() {
  const { data: settings } = useSettings();
  const [dark, setDark] = useState(systemDark);
  useEffect(() => {
    const media = window.matchMedia?.(DARK_QUERY);
    if (!media) return;
    const follow = () => setDark(media.matches);
    media.addEventListener("change", follow);
    return () => media.removeEventListener("change", follow);
  }, []);
  const appearance = settings?.appearance ?? "system";
  useEffect(() => applyScheme(schemeFor(appearance, dark)), [appearance, dark]);
}

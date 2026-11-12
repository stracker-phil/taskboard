import { useEffect, useState } from "react";
import type { Accent, Appearance } from "../api";
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

/** Changes the look at once. WebKit leaves a color with a transition stale when only an inherited custom property
 *  changes, so transitions pause while the window restyles. */
function setLook(key: "scheme" | "accent", value: string) {
  const root = document.documentElement;
  if (root.dataset[key] === value) return;
  root.classList.add("restyling");
  root.dataset[key] = value;
  // Restyles now, while nothing transitions.
  void root.offsetHeight;
  requestAnimationFrame(() => root.classList.remove("restyling"));
}

function applyScheme(scheme: Scheme) {
  setLook("scheme", scheme);
}

/** Sets `data-scheme` on <html> before the first render (from macOS, until the settings load), so the first
 *  paint has colors. */
export function applyInitialLook() {
  applyScheme(systemDark() ? "dark" : "light");
  setLook("accent", "system");
}

/** Keeps the window's scheme in step with the Appearance setting (on Automatic, with macOS) and its accent with the
 *  app's; `spaceAccent`, the shown Space's own, replaces the app's. */
export function useAppearance(spaceAccent: Accent | null = null) {
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
  const accent = spaceAccent ?? settings?.accent ?? "system";
  useEffect(() => setLook("accent", accent), [accent]);
}

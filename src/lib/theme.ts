/** The palette's scheme (`palettes/<scheme>.css`). Taskboard has only the light one until the dark palette lands. */
export type Scheme = "light";

/** Sets `data-scheme` on <html> before the first render, so the first paint has colours. */
export function applyInitialLook() {
  document.documentElement.dataset.scheme = "light" satisfies Scheme;
}

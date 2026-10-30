import { type RefObject, useCallback, useEffect, useRef, useState } from "react";
import { findMatches, searchTerms } from "./findMatches";

const MATCH = "find-match";
const CURRENT = "find-current";

/** The page's highlight registry; absent where the browser lacks the Custom Highlight API (tests). */
const registry = () => (typeof CSS !== "undefined" && "highlights" in CSS ? CSS.highlights : null);

function paint(ranges: Range[], current: number) {
  const highlights = registry();
  if (!highlights) return;
  highlights.set(MATCH, new Highlight(...ranges));
  const shown = ranges[current] ? new Highlight(ranges[current]) : new Highlight();
  shown.priority = 1;
  highlights.set(CURRENT, shown);
}

function unpaint() {
  registry()?.delete(MATCH);
  registry()?.delete(CURRENT);
}

const scrollsY = (element: Element) => /(auto|scroll)/.test(getComputedStyle(element).overflowY) && element.scrollHeight > element.clientHeight;

/** Scrolls the match's own scroll area until the match sits mid-way, unless it is in view already. */
function reveal(range: Range) {
  let area = range.startContainer.parentElement;
  while (area && !scrollsY(area)) area = area.parentElement;
  if (!area) return;
  const match = range.getBoundingClientRect();
  const view = area.getBoundingClientRect();
  if (match.top >= view.top && match.bottom <= view.bottom) return;
  area.scrollTop += match.top - view.top - (view.height - match.height) / 2;
}

/** Highlights every word of `query` under `rootRef` and steps through the matches. It follows the content as it
 *  changes (comments loading, the next issue) and never touches the DOM it searches. */
export function useFindInPage(rootRef: RefObject<HTMLElement | null>, query: string) {
  const [status, setStatus] = useState({ count: 0, current: 0 });
  // `revealed` is false until Enter first brings the current match into view.
  const found = useRef({ key: "", ranges: [] as Range[], current: 0, revealed: false });

  const search = useCallback(
    (text: string) => {
      const root = rootRef.current;
      const terms = searchTerms(text);
      const key = terms.join(" ");
      const ranges = root ? findMatches(root, terms) : [];
      const same = key === found.current.key;
      const current = same ? Math.min(found.current.current, Math.max(ranges.length - 1, 0)) : 0;
      found.current = { key, ranges, current, revealed: same && found.current.revealed };
      if (ranges.length > 0) paint(ranges, current);
      else unpaint();
      setStatus({ count: ranges.length, current });
    },
    [rootRef],
  );

  useEffect(() => {
    search(query);
    const root = rootRef.current;
    if (!root || searchTerms(query).length === 0) return;
    // WebKit paints highlights only on selectable text, and the app's text isn't selectable.
    root.dataset.finding = "";
    let frame = 0;
    const observer = new MutationObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => search(query));
    });
    observer.observe(root, { childList: true, subtree: true, characterData: true });
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
      delete root.dataset.finding;
    };
  }, [rootRef, query, search]);

  useEffect(() => unpaint, []);

  /** Enter: shows the current match first, then the next (or, backwards, the previous) one. */
  const step = useCallback(
    (text: string, backwards: boolean) => {
      if (searchTerms(text).join(" ") !== found.current.key) search(text);
      const { ranges, current, revealed } = found.current;
      if (ranges.length === 0) return;
      const next = revealed ? (current + (backwards ? -1 : 1) + ranges.length) % ranges.length : current;
      found.current = { ...found.current, current: next, revealed: true };
      paint(ranges, next);
      reveal(ranges[next]);
      setStatus({ count: ranges.length, current: next });
    },
    [search],
  );

  return { ...status, step };
}

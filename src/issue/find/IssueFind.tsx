import { type RefObject, useRef, useState } from "react";
import { usePageAction } from "../../commands/pageActions";
import { TextFilter } from "../../shell/toolbar/TextFilter";
import { searchTerms } from "./findMatches";
import { useFindInPage } from "./useFindInPage";

/** The Issue view's find: highlights each word under `rootRef`; Enter steps to the next match, ⇧Enter back. */
export function IssueFind({ rootRef }: { rootRef: RefObject<HTMLElement | null> }) {
  const [query, setQuery] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const { count, current, step } = useFindInPage(rootRef, query);
  // ⌘F finds in the issue, as it filters the view in the List.
  usePageAction("toolbar.search", () => {
    inputRef.current?.focus();
    inputRef.current?.select();
  });
  const status = searchTerms(query).length === 0 ? "" : `${count === 0 ? 0 : current + 1}/${count}`;
  return <TextFilter value={query} onChange={setQuery} inputRef={inputRef} label="Find in this issue" onEnter={step} status={status} />;
}

import { ArrowLeft, ArrowRight } from "lucide-react";
import { usePageAction } from "../commands/pageActions";
import { useShortcuts } from "../commands/useShortcuts";
import { useNav, useTravel } from "../queries/sidebar";

/** Back and Forward through the active tab's places and issues, beside the traffic lights in every layout. */
export function NavArrows() {
  const { data: nav } = useNav();
  const travel = useTravel();
  const { title } = useShortcuts();
  const back = nav?.canBack ? () => void travel("back") : null;
  const forward = nav?.canForward ? () => void travel("forward") : null;
  usePageAction("history.back", back);
  usePageAction("history.forward", forward);
  return (
    <div className="nav-arrows">
      <button type="button" className="nav-arrows__btn" aria-label="Back" title={title("Back", "history.back")} disabled={!back} onClick={back ?? undefined}>
        <ArrowLeft size={16} strokeWidth={1.8} aria-hidden="true" />
      </button>
      <button type="button" className="nav-arrows__btn" aria-label="Forward" title={title("Forward", "history.forward")} disabled={!forward} onClick={forward ?? undefined}>
        <ArrowRight size={16} strokeWidth={1.8} aria-hidden="true" />
      </button>
    </div>
  );
}

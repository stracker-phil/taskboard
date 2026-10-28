import { X } from "lucide-react";
import { useState } from "react";
import { clockTime } from "../lib/relativeTime";
import { useConnection } from "../queries/settings";
import { useSyncOverview } from "../queries/sync";
import { runBusCommand } from "../state/requests";

const DISMISSED_KEY = "taskboard.tokenBannerDismissed";
const today = () => new Date().toDateString();

function dismissedToday(): boolean {
  try {
    return localStorage.getItem(DISMISSED_KEY) === today();
  } catch {
    return false;
  }
}

const openConnection = () => runBusCommand("app:settings:connection", "Could not open Settings");

/** Under the content header (artboard 8, panel 4): red while Jira refuses the token (sync paused), amber in the
 *  14 days before it expires (dismissed for the rest of the day), red again once the expiry date passed. The app stays usable on the cache. */
export function TokenBanner() {
  const { data: overview } = useSyncOverview();
  const { data: connection } = useConnection();
  const [dismissed, setDismissed] = useState(dismissedToday);
  if (overview?.state.state === "paused") {
    const since = overview.lastFullAt ? ` Showing cached data from ${clockTime(overview.lastFullAt)}.` : "";
    return (
      <div className="banner banner--error" role="alert">
        <span className="banner__dot" aria-hidden="true" />
        <span className="banner__text">Jira rejected the token.{since}</span>
        <button type="button" className="banner__primary" onClick={openConnection}>
          Update token
        </button>
      </div>
    );
  }
  const expiry = connection?.expiry;
  if (expiry?.state === "expired") {
    return (
      <div className="banner banner--error" role="alert">
        <span className="banner__dot" aria-hidden="true" />
        <span className="banner__text">Your Jira token has expired; sync will stop until you replace it.</span>
        <button type="button" className="banner__primary" onClick={openConnection}>
          Update token
        </button>
      </div>
    );
  }
  if (expiry?.state !== "soon" || dismissed) return null;
  const when = expiry.days === 0 ? "today" : `in ${expiry.days} ${expiry.days === 1 ? "day" : "days"}`;
  const dismiss = () => {
    try {
      localStorage.setItem(DISMISSED_KEY, today());
    } catch {
      // Without storage the banner hides until the next start.
    }
    setDismissed(true);
  };
  return (
    <div className="banner banner--warn" role="status">
      <span className="banner__dot" aria-hidden="true" />
      <span className="banner__text">Your Jira token expires {when}.</span>
      <button type="button" className="banner__secondary" onClick={openConnection}>
        Replace
      </button>
      <button type="button" className="banner__close" aria-label="Dismiss for today" onClick={dismiss}>
        <X size={14} strokeWidth={2} aria-hidden="true" />
      </button>
    </div>
  );
}

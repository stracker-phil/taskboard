import { api, type IssueDetail } from "../api";
import { formatLocalDate } from "../lib/time";
import { usePinOf } from "../queries/pins";
import { attempt } from "../state/requests";

/** An issue Jira deleted or hid from the owner (artboard 8, card 6): the cached copy, Remove from cache, Unpin. */
export function GoneNotice({ issue, onRemoved }: { issue: IssueDetail; onRemoved: () => void }) {
  const pin = usePinOf("issue", issue.id);
  const remove = async () => {
    if ((await attempt(api.issueForget(issue.id), "Could not remove it from the cache")) !== null) onRemoved();
  };
  return (
    <div className="gone-notice">
      <p className="gone-notice__text">Deleted in Jira or you lost access. Last synced {formatLocalDate(issue.syncedAt)}. This is the cached copy.</p>
      <div className="gone-notice__actions">
        <button type="button" className="btn-secondary btn-secondary--small" onClick={() => void remove()}>
          Remove from cache
        </button>
        {pin && (
          <button type="button" className="empty-level__ghost" onClick={() => void attempt(api.pinRemove(pin.id), "Could not unpin it")}>
            Unpin
          </button>
        )}
      </div>
    </div>
  );
}

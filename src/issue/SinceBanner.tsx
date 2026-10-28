import type { Since } from "../api";

/** The blue summary of what changed since the owner last looked; the Issue view adds Mark seen. */
export function SinceBanner({ since, onMarkSeen }: { since: Since; onMarkSeen?: () => void }) {
  return (
    <div className={onMarkSeen ? "since-banner since-banner--detail" : "since-banner"} role="status">
      <span className="since-banner__dot" aria-hidden="true" />
      <span className="since-banner__text">
        <strong>{since.label}:</strong> {since.text}
      </span>
      {onMarkSeen && (
        <button type="button" className="since-banner__btn" onClick={onMarkSeen}>
          Mark seen
        </button>
      )}
    </div>
  );
}

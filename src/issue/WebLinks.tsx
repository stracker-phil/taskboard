import { GitPullRequest, Link } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { api, type WebLink } from "../api";
import { useIssueWebLinks, webLinksKey } from "../queries/issues";
import { attempt } from "../state/requests";

/** The issue's web links (pull requests, documents), fetched again whenever the issue is shown. Hidden when
 *  there are none. Only https links open, in the default browser; the rest show as text. */
export function WebLinks({ id, variant }: { id: string; variant: "issue" | "peek" }) {
  const client = useQueryClient();
  const { data: links = [] } = useIssueWebLinks(id);
  useEffect(() => {
    // A failure (offline) leaves the cached links; Rust logs it. New links are read again and set directly: an
    // invalidation would join a first read still in flight and keep its old answer.
    void api
      .issueSyncWebLinks(id)
      .then((changed) => (changed ? api.issueWebLinks(id).then((links) => client.setQueryData(webLinksKey(id), links)) : undefined))
      .catch(() => {});
  }, [id, client]);
  if (links.length === 0) return null;
  return (
    <section className={`web-links web-links--${variant}`} aria-label="Web links">
      <h2>Web links</h2>
      {links.map((link) => (
        <WebLinkRow key={link.id} link={link} />
      ))}
    </section>
  );
}

function WebLinkRow({ link }: { link: WebLink }) {
  const Icon = link.kind === "pullRequest" ? GitPullRequest : Link;
  const content = (
    <>
      <Icon size={14} strokeWidth={1.9} className="web-link__icon" aria-hidden="true" />
      <span className="web-link__title">{link.title}</span>
      {(link.reference ?? link.host) && <span className="web-link__meta">{link.reference ?? link.host}</span>}
    </>
  );
  if (!link.openable) {
    return (
      <div className="web-link web-link--inert" title={link.url}>
        {content}
      </div>
    );
  }
  return (
    <button type="button" className="web-link" title={link.url} onClick={() => void attempt(api.openUrl(link.url), "Could not open the link")}>
      {content}
    </button>
  );
}

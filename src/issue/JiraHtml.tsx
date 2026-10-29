import { type MouseEvent, useMemo } from "react";
import { api } from "../api";
import { sanitizeJiraHtml } from "../lib/jiraHtml";
import { useConnection } from "../queries/settings";
import { attempt } from "../state/requests";

/** Jira's HTML (a description, a comment), sanitized; its links open in the default browser. */
export function JiraHtml({ html, className }: { html: string; className?: string }) {
  const { data: connection } = useConnection();
  const site = connection?.connection.siteUrl;
  const clean = useMemo(() => sanitizeJiraHtml(html, site), [html, site]);
  const openLink = (e: MouseEvent) => {
    const link = e.target instanceof Element ? e.target.closest("a") : null;
    if (!link) return;
    e.preventDefault();
    const href = link.getAttribute("href");
    if (href) void attempt(api.openUrl(href), "Could not open the link");
  };
  return <div className={["jira-html", className].filter(Boolean).join(" ")} onClick={openLink} dangerouslySetInnerHTML={{ __html: clean }} />;
}

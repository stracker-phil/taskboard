/** An issue's or project's page on the Jira site (`/browse/KEY`); null without a site. */
export function browseUrl(siteUrl: string | undefined, key: string): string | null {
  const site = siteUrl?.replace(/\/+$/, "");
  return site ? `${site}/browse/${encodeURIComponent(key)}` : null;
}

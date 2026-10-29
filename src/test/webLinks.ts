import type { WebLink } from "../api";

/** An openable web link to a document; override per test. */
export const webLink = (patch: Partial<WebLink> = {}): WebLink => ({
  id: "30001",
  url: "https://docs.example.com/guides/postcodes",
  title: "Postcode guide",
  host: "docs.example.com",
  kind: "link",
  reference: null,
  openable: true,
  ...patch,
});

/** An openable pull request link. */
export const pullRequestLink = (patch: Partial<WebLink> = {}): WebLink =>
  webLink({
    id: "30002",
    url: "https://github.com/example/shop/pull/481",
    title: "Validate DACH postcodes",
    host: "github.com",
    kind: "pullRequest",
    reference: "shop#481",
    ...patch,
  });

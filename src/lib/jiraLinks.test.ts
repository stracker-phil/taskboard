import { describe, expect, it } from "vitest";
import { browseUrl } from "./jiraLinks";

describe("browseUrl", () => {
  it.each(["https://example.atlassian.net", "https://example.atlassian.net/", "https://example.atlassian.net///"])(
    "points to the issue's page on %s",
    (site) => {
      expect(browseUrl(site, "NW-1042")).toBe("https://example.atlassian.net/browse/NW-1042");
    },
  );

  it("encodes the key", () => {
    expect(browseUrl("https://example.atlassian.net", "NW 1/2?")).toBe("https://example.atlassian.net/browse/NW%201%2F2%3F");
  });

  it.each([undefined, "", "/"])("is null without a site (%s)", (site) => {
    expect(browseUrl(site, "NW-1042")).toBeNull();
  });
});

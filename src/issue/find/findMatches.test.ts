import { afterEach, describe, expect, it } from "vitest";
import { findMatches, searchTerms } from "./findMatches";

function page(html: string): HTMLElement {
  const root = document.createElement("div");
  root.innerHTML = html;
  document.body.append(root);
  return root;
}

const texts = (ranges: Range[]) => ranges.map((range) => range.toString());

afterEach(() => {
  document.body.innerHTML = "";
});

describe("searchTerms", () => {
  it("splits the query on whitespace and lowercases the words", () => {
    expect(searchTerms("  Payment \t GATEWAY ")).toEqual(["payment", "gateway"]);
  });

  it("keeps each word once", () => {
    expect(searchTerms("Retry retry RETRY")).toEqual(["retry"]);
  });

  it.each(["", "   "])("gives no words for the query %j", (query) => {
    expect(searchTerms(query)).toEqual([]);
  });
});

describe("findMatches", () => {
  it("finds every word of the query regardless of case", () => {
    const root = page("<p>Payment flow through the GATEWAY</p>");

    expect(texts(findMatches(root, searchTerms("payment gateway")))).toEqual(["Payment", "GATEWAY"]);
  });

  it("returns matches across several elements in document order", () => {
    const root = page("<h2>Gateway timeout</h2><ul><li>Retry the payment</li></ul><p>Log the gateway id</p>");

    expect(texts(findMatches(root, searchTerms("payment gateway")))).toEqual(["Gateway", "payment", "gateway"]);
  });

  it("finds repeated occurrences within one text node", () => {
    const root = page("<p>retry, retry and retry again</p>");

    expect(texts(findMatches(root, ["retry"]))).toEqual(["retry", "retry", "retry"]);
  });

  it("merges overlapping matches into one", () => {
    const root = page("<p>payment</p>");

    expect(texts(findMatches(root, searchTerms("pay payment")))).toEqual(["payment"]);
  });

  it("merges a word that overlaps the end of another into one span", () => {
    const root = page("<p>checkout</p>");

    expect(texts(findMatches(root, ["check", "kout"]))).toEqual(["checkout"]);
  });

  it("finds nothing for an empty query", () => {
    const root = page("<p>payment gateway</p>");

    expect(findMatches(root, searchTerms(""))).toEqual([]);
  });

  it("finds nothing when no word occurs", () => {
    const root = page("<p>payment gateway</p>");

    expect(findMatches(root, searchTerms("invoice"))).toEqual([]);
  });

  describe("neighboring matches", () => {
    it("merge into one when only a space lies between them, whatever the word order of the query", () => {
      const root = page("<p>Validate the checkout now</p>");

      const ranges = findMatches(root, searchTerms("checkout the"));

      expect(texts(ranges)).toEqual(["the checkout"]);
    });

    it("merge across inline elements on the same line", () => {
      const root = page("<p><strong>Payment</strong> gateway</p>");

      const ranges = findMatches(root, searchTerms("payment gateway"));

      expect(texts(ranges)).toEqual(["Payment gateway"]);
    });

    it("merge when tabs or no-break spaces lie between them", () => {
      const root = page("<p>payment\t gateway</p>");

      expect(texts(findMatches(root, searchTerms("payment gateway")))).toEqual(["payment\t gateway"]);
    });

    it("merge three in a row into one", () => {
      const root = page("<p>Pick the guest checkout today</p>");

      const ranges = findMatches(root, searchTerms("the guest checkout"));

      expect(texts(ranges)).toEqual(["the guest checkout"]);
    });

    it.each([
      ["punctuation", "<p>payment, gateway</p>"],
      ["another word", "<p>payment the gateway</p>"],
    ])("stay separate when %s lies between them", (_, html) => {
      const root = page(html);

      const ranges = findMatches(root, searchTerms("payment gateway"));

      expect(texts(ranges)).toEqual(["payment", "gateway"]);
    });

    it("stay separate in different block elements even with nothing between them", () => {
      const root = page("<p>payment</p><p>gateway</p>");

      const ranges = findMatches(root, searchTerms("payment gateway"));

      expect(texts(ranges)).toEqual(["payment", "gateway"]);
    });
  });

  it("searches only below the given root", () => {
    const root = page("<p>inside payment</p>");
    const outside = document.createElement("p");
    outside.textContent = "outside payment";
    document.body.append(outside);

    expect(texts(findMatches(root, ["payment"]))).toEqual(["payment"]);
  });
});

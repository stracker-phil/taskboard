import { describe, expect, it } from "vitest";
import { sanitizeJiraHtml } from "./jiraHtml";

const SITE = "https://example.atlassian.net/";

/** `site` null means no site at all. */
function clean(html: string, site: string | null = SITE): HTMLElement {
  const holder = document.createElement("div");
  holder.innerHTML = sanitizeJiraHtml(html, site ?? undefined);
  return holder;
}

describe("sanitizeJiraHtml", () => {
  describe("dangerous markup", () => {
    it.each(["script", "style", "iframe", "object", "form"])("removes <%s> elements", (tag) => {
      const out = clean(`<p>Kept</p><${tag}>alert(1)</${tag}>`);

      expect(out.querySelector(tag)).toBeNull();
      expect(out.textContent).toContain("Kept");
    });

    it("removes event handler attributes, inline styles and data attributes", () => {
      const out = clean('<p onclick="x()" style="color:red" data-id="7" title="Hi">Text</p>');

      const p = out.querySelector("p");
      expect(p?.getAttribute("onclick")).toBeNull();
      expect(p?.getAttribute("style")).toBeNull();
      expect(p?.getAttribute("data-id")).toBeNull();
      expect(p?.getAttribute("title")).toBe("Hi");
    });
  });

  it("keeps text formatting, lists, tables, code and links", () => {
    const out = clean(
      '<p><strong>a</strong><em>b</em></p><ul><li>c</li></ul><table><tbody><tr><td>d</td></tr></tbody></table><pre><code>e</code></pre><a href="https://example.com/x">f</a>',
    );

    for (const selector of ["p", "strong", "em", "ul li", "table td", "pre code", "a"]) expect(out.querySelector(selector), selector).not.toBeNull();
  });

  describe("inline code and lists", () => {
    it.each(["code", "tt"])("keeps an inline <%s> element with its text", (tag) => {
      const out = clean(`<p>Call <${tag}>Shop()-&gt;cart-&gt;get_totals()</${tag}></p>`);

      expect(out.querySelector(tag)?.textContent).toBe("Shop()->cart->get_totals()");
    });

    it("keeps the start number of an ordered list", () => {
      const out = clean('<ol start="3"><li>Third</li><li>Fourth</li></ol>');

      expect(out.querySelector("ol")?.getAttribute("start")).toBe("3");
    });
  });

  describe("class names", () => {
    it.each([
      ["div", "adf-panel adf-panel--warning"],
      ["div", "adf-panel adf-panel--custom"],
      ["span", "adf-status adf-status--green"],
      ["span", "adf-mention"],
      ["div", "adf-expand"],
      ["span", "adf-expand__title"],
    ])("keeps the allowed classes of a <%s> with class %j", (tag, classes) => {
      const out = clean(`<${tag} class="${classes}">Text</${tag}>`);

      expect(out.querySelector(tag)?.getAttribute("class")).toBe(classes);
    });

    it("removes unknown classes but keeps allowed ones on the same element", () => {
      const out = clean('<span class="adf-status evil">Done</span>');

      expect(out.querySelector("span")?.getAttribute("class")).toBe("adf-status");
    });

    it.each(["user-hover", "aui-lozenge", "user-hover aui-lozenge"])("removes the class attribute when only unknown classes (%s) are left", (classes) => {
      const out = clean(`<a class="${classes}" href="https://example.com/u">Ada</a>`);

      expect(out.querySelector("a")?.hasAttribute("class")).toBe(false);
      expect(out.querySelector("a")?.textContent).toBe("Ada");
    });

    it("keeps the chip class on an image chip", () => {
      const out = clean('<img class="evil" src="https://example.com/a.png" alt="Logo">');

      expect(out.querySelector("a")?.getAttribute("class")).toBe("jira-chip");
    });
  });

  describe("links", () => {
    it.each([
      ["javascript:", "javascript:alert(1)"],
      ["data:", "data:text/html;base64,PHA+"],
      ["http:", "http://example.com/page"],
      ["credentials", "https://user:secret@example.com/page"],
    ])("keeps the text of a %s link but drops its href", (_label, href) => {
      const link = clean(`<a href="${href}">Open it</a>`).querySelector("a");

      expect(link?.textContent).toBe("Open it");
      expect(link?.hasAttribute("href")).toBe(false);
    });

    it("keeps an absolute https link", () => {
      expect(clean('<a href="https://example.com/doc">Doc</a>').querySelector("a")?.getAttribute("href")).toBe("https://example.com/doc");
    });

    it("resolves a relative link against the Jira site", () => {
      expect(clean('<a href="/browse/NW-1">NW-1</a>').querySelector("a")?.getAttribute("href")).toBe("https://example.atlassian.net/browse/NW-1");
    });

    it("resolves a relative link against a site given without a trailing slash", () => {
      expect(clean('<a href="/browse/NW-1">NW-1</a>', "https://example.atlassian.net").querySelector("a")?.getAttribute("href")).toBe(
        "https://example.atlassian.net/browse/NW-1",
      );
    });

    it("drops the href of a relative link when there is no site", () => {
      const link = clean('<a href="/browse/NW-1">NW-1</a>', null).querySelector("a");

      expect(link?.textContent).toBe("NW-1");
      expect(link?.hasAttribute("href")).toBe(false);
    });
  });

  describe("images", () => {
    it("turns an image into a chip linking to its address, named by its alt text", () => {
      const out = clean('<img src="https://example.com/files/shot.png" alt="Checkout screenshot">');

      const chip = out.querySelector("a.jira-chip");
      expect(chip?.textContent).toBe("Image · Checkout screenshot");
      expect(chip?.getAttribute("href")).toBe("https://example.com/files/shot.png");
      expect(out.querySelector("img")).toBeNull();
    });

    it("names the chip after the file name when there is no alt text", () => {
      const out = clean('<img src="https://example.com/files/flow%20chart.png">');

      expect(out.querySelector("a.jira-chip")?.textContent).toBe("Image · flow chart.png");
    });

    it("resolves a relative image address against the Jira site", () => {
      const out = clean('<img src="/secure/attachment/10/mock.png" alt="Mock">');

      expect(out.querySelector("a.jira-chip")?.getAttribute("href")).toBe("https://example.atlassian.net/secure/attachment/10/mock.png");
    });

    it.each([
      ["a data: address", "data:image/png;base64,AAAA"],
      ["an http: address", "http://example.com/a.png"],
      ["a missing address", ""],
    ])("turns an image with %s into a chip without a link", (_label, src) => {
      const out = clean(`<img src="${src}" alt="Logo">`);

      const chip = out.querySelector("span.jira-chip");
      expect(chip?.textContent).toBe("Image · Logo");
      expect(chip?.hasAttribute("href")).toBe(false);
      expect(out.querySelector("img")).toBeNull();
    });

    it("never leaves an <img> behind", () => {
      const out = clean('<p><img src="https://example.com/a.png"><img src="x"><img></p>');

      expect(out.querySelector("img")).toBeNull();
      expect(out.querySelectorAll(".jira-chip")).toHaveLength(3);
    });

    describe("attachment placeholders", () => {
      it("keeps the id, alt text and size of an attachment placeholder and nothing else", () => {
        const out = clean(
          '<img data-attachment-id="501" alt="Shot" width="640" height="360" src="https://example.com/a.png" class="evil" title="T" onerror="x()">',
        );

        const img = out.querySelector("img");
        expect(img?.getAttributeNames().sort()).toEqual(["alt", "data-attachment-id", "height", "width"]);
        expect(img?.getAttribute("data-attachment-id")).toBe("501");
        expect(img?.getAttribute("alt")).toBe("Shot");
        expect(img?.getAttribute("width")).toBe("640");
        expect(img?.getAttribute("height")).toBe("360");
      });

      it.each([
        ["width", "wide"],
        ["height", "100%"],
      ])("drops a placeholder's %s that is not digits (%s)", (attribute, value) => {
        const img = clean(`<img data-attachment-id="501" alt="Shot" ${attribute}="${value}">`).querySelector("img");

        expect(img?.getAttribute("data-attachment-id")).toBe("501");
        expect(img?.hasAttribute(attribute)).toBe(false);
      });

      it.each(["abc", "12a", "", "-1"])("turns an image whose attachment id is %j into a chip", (id) => {
        const out = clean(`<img data-attachment-id="${id}" alt="Shot">`);

        expect(out.querySelector("img")).toBeNull();
        expect(out.querySelector(".jira-chip")?.textContent).toBe("Image · Shot");
      });

      it.each(["data-attachment-id", "width", "height"])("removes %s from elements other than images", (attribute) => {
        const out = clean(`<p ${attribute}="5">Text</p>`);

        expect(out.querySelector("p")?.hasAttribute(attribute)).toBe(false);
      });
    });
  });
});

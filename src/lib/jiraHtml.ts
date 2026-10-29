import createDOMPurify from "dompurify";

/** Jira's HTML is untrusted (ADR 0027): only text formatting, lists, tables, code and links survive, with a
 *  fixed set of class names; scripts, styles, frames, forms and event handlers never do. */
const ALLOWED_TAGS = [
  "p", "br", "hr", "div", "span", "strong", "b", "em", "i", "u", "s", "del", "ins", "sub", "sup", "small",
  "h1", "h2", "h3", "h4", "h5", "h6", "blockquote", "ul", "ol", "li", "dl", "dt", "dd",
  "pre", "code", "tt", "kbd", "table", "thead", "tbody", "tfoot", "tr", "th", "td", "caption", "a", "img",
];
const ALLOWED_ATTR = ["href", "src", "alt", "title", "colspan", "rowspan", "start", "class"];
/** Only the inline image placeholder Rust writes keeps these (ADR 0039); anything else loses them. */
const PLACEHOLDER_ATTR = ["data-attachment-id", "width", "height"];
const DIGITS = /^\d+$/;
/** The only class names that survive: the ones Rust's ADF renderer writes (`src-tauri/src/adf/`). */
const ALLOWED_CLASSES = new Set([
  "jira-chip", "adf-mention", "adf-expand", "adf-expand__title", "adf-panel", "adf-status",
  ...["info", "note", "warning", "success", "error", "tip", "custom"].map((tone) => `adf-panel--${tone}`),
  ...["neutral", "purple", "blue", "red", "yellow", "green"].map((color) => `adf-status--${color}`),
]);

const purify = createDOMPurify(window);

/** The file name at the end of an image's address, for its chip. */
function imageName(src: string, alt: string | null): string {
  if (alt?.trim()) return alt.trim();
  const last = decodeURIComponent(new URL(src).pathname.split("/").filter(Boolean).pop() ?? "");
  return last || "image";
}

/** An absolute `https` address on any host, resolved against the Jira site; null for anything else. */
function webAddress(value: string | null, base: string | undefined): string | null {
  if (!value) return null;
  try {
    const url = base ? new URL(value, base) : new URL(value);
    return url.protocol === "https:" && !url.username && !url.password ? url.href : null;
  } catch {
    return null;
  }
}

/** `<img data-attachment-id="…">` with digits: an image attached to the issue, which `JiraHtml` fills from Rust. */
function isPlaceholder(element: Element): boolean {
  return element.tagName === "IMG" && DIGITS.test(element.getAttribute("data-attachment-id") ?? "");
}

/** Keeps the placeholder's id, alt and a size in digits; drops `src` and everything else. */
function keepPlaceholder(image: Element) {
  for (const { name, value } of Array.from(image.attributes)) {
    const kept = name === "data-attachment-id" || name === "alt" || ((name === "width" || name === "height") && DIGITS.test(value));
    if (!kept) image.removeAttribute(name);
  }
}

/**
 * Sanitizes Jira HTML for display: relative links point at the Jira site, links that aren't `https` lose
 * their address, and every image becomes a link chip ("Image · name"), so the webview never loads remote
 * content (ADR 0020). The one exception is the placeholder for an image attached to the issue
 * (`<img data-attachment-id="…" alt width height>`, never a `src`), which `JiraHtml` fills with a `data:` URL
 * (ADR 0039). Links open in the browser through `JiraHtml`'s click handler.
 */
export function sanitizeJiraHtml(html: string, siteUrl?: string): string {
  const base = siteUrl ? `${siteUrl.replace(/\/+$/, "")}/` : undefined;
  const fragment = purify.sanitize(html, {
    ALLOWED_TAGS,
    ALLOWED_ATTR: [...ALLOWED_ATTR, ...PLACEHOLDER_ATTR],
    ALLOW_DATA_ATTR: false,
    RETURN_DOM_FRAGMENT: true,
  });
  for (const element of Array.from(fragment.querySelectorAll("[class]"))) {
    const kept = Array.from(element.classList).filter((name) => ALLOWED_CLASSES.has(name));
    if (kept.length) element.className = kept.join(" ");
    else element.removeAttribute("class");
  }
  for (const element of Array.from(fragment.querySelectorAll(PLACEHOLDER_ATTR.map((name) => `[${name}]`).join(",")))) {
    if (isPlaceholder(element)) keepPlaceholder(element);
    else for (const name of PLACEHOLDER_ATTR) element.removeAttribute(name);
  }
  for (const image of Array.from(fragment.querySelectorAll("img"))) {
    if (isPlaceholder(image)) continue;
    const src = webAddress(image.getAttribute("src"), base);
    const chip = fragment.ownerDocument.createElement(src ? "a" : "span");
    chip.className = "jira-chip";
    chip.textContent = `Image · ${src ? imageName(src, image.getAttribute("alt")) : (image.getAttribute("alt") ?? "image")}`;
    if (src) chip.setAttribute("href", src);
    image.replaceWith(chip);
  }
  for (const link of Array.from(fragment.querySelectorAll("a"))) {
    const href = webAddress(link.getAttribute("href"), base);
    if (href) link.setAttribute("href", href);
    else link.removeAttribute("href");
  }
  const holder = fragment.ownerDocument.createElement("div");
  holder.append(fragment);
  return holder.innerHTML;
}

/** Whether Jira's HTML has any text or image to show; `<p> </p>` has none. */
export function hasContent(html: string | null): boolean {
  if (!html) return false;
  const body = new DOMParser().parseFromString(html, "text/html").body;
  return (body.textContent?.trim() ?? "") !== "" || body.querySelector("img") !== null;
}

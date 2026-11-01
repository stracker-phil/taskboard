/** jsdom has no `isContentEditable`; the key handling asks for it to tell a text field from the page. */
export function emulateContentEditable() {
  Object.defineProperty(HTMLElement.prototype, "isContentEditable", {
    configurable: true,
    get(this: HTMLElement) {
      const value = this.getAttribute("contenteditable");
      return value !== null && value !== "false";
    },
  });
}

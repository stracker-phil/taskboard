import { afterEach, describe, expect, it, vi } from "vitest";
import { menuAnchor } from "./menuAnchor";

function add(html: string) {
  const host = document.createElement("div");
  host.innerHTML = html;
  document.body.append(host);
  return [...host.children] as HTMLElement[];
}

/** jsdom has no layout: each call answers with the next element, in the order the anchors are tested. */
const hitInOrder = (...hits: (Element | null)[]) => {
  const stub = vi.fn();
  hits.forEach((hit) => stub.mockReturnValueOnce(hit));
  document.elementFromPoint = stub;
};

afterEach(() => {
  document.body.innerHTML = "";
  Reflect.deleteProperty(document, "elementFromPoint");
});

describe("menuAnchor", () => {
  it("returns the first matching element when there is no layout to hit-test", () => {
    const [first] = add('<button data-assign-menu="10042"></button><button data-assign-menu="10042"></button>');

    expect(menuAnchor("data-assign-menu", "10042")).toBe(first);
  });

  it("skips a covered match for a later one that is in sight", () => {
    const [, visible, cover] = add('<button data-assign-menu="10042"></button><button data-assign-menu="10042"></button><div></div>');
    hitInOrder(cover, visible);

    expect(menuAnchor("data-assign-menu", "10042")).toBe(visible);
  });

  it("counts a match as in sight when the point hits one of its children", () => {
    const [button] = add('<button data-assign-menu="10042"><span>Alex</span></button>');
    hitInOrder(button.querySelector("span"));

    expect(menuAnchor("data-assign-menu", "10042")).toBe(button);
  });

  it("returns the issue's row when every match is covered", () => {
    const [row, , cover] = add('<div data-issue-id="10042"></div><span data-assign-menu="10042"></span><div></div>');
    hitInOrder(cover, row);

    expect(menuAnchor("data-assign-menu", "10042")).toBe(row);
  });

  it("returns a point near the top of the window when nothing is in sight", () => {
    const [, , cover] = add('<div data-issue-id="10042"></div><span data-assign-menu="10042"></span><div></div>');
    hitInOrder(cover, cover);

    expect(menuAnchor("data-assign-menu", "10042")).toEqual({ x: window.innerWidth / 2, y: 120 });
  });

  it("returns the fallback point when the issue is not on screen at all", () => {
    expect(menuAnchor("data-assign-menu", "10042")).toEqual({ x: window.innerWidth / 2, y: 120 });
  });

  it("ignores matches of other issues and other attributes", () => {
    add('<span data-assign-menu="10043"></span><span data-status-menu="10042"></span><div data-issue-id="10043"></div>');

    expect(menuAnchor("data-assign-menu", "10042")).toEqual({ x: window.innerWidth / 2, y: 120 });
  });

  it("matches an id that needs CSS escaping", () => {
    const [button] = add('<button data-priority-menu="10042:1"></button>');

    expect(menuAnchor("data-priority-menu", "10042:1")).toBe(button);
  });
});

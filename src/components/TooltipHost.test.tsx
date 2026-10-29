import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TooltipHost } from "./TooltipHost";

function renderHost() {
  render(
    <>
      <button type="button" title="Refresh the list" />
      <button type="button" title="Sort the rows">
        Sort
      </button>
      <button type="button" title="Open the settings" />
      <span>No tip here</span>
      <TooltipHost />
    </>,
  );
}

const iconOnly = () => screen.getByRole("button", { name: "Refresh the list" });
const second = () => screen.getByRole("button", { name: "Open the settings" });
const wait = (ms: number) => act(() => vi.advanceTimersByTimeAsync(ms));

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("TooltipHost", () => {
  it("shows the title as a tooltip after the pointer rests on the element", async () => {
    renderHost();

    fireEvent.pointerOver(iconOnly());
    await wait(300);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    await wait(200);

    expect(screen.getByRole("tooltip")).toHaveTextContent("Refresh the list");
  });

  it("shows nothing when the pointer leaves before the delay is over", async () => {
    renderHost();

    fireEvent.pointerOver(iconOnly());
    await wait(200);
    fireEvent.pointerOver(screen.getByText("No tip here"));
    await wait(600);

    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("moves the title to data-tooltip so the native tooltip never shows", () => {
    renderHost();

    fireEvent.pointerOver(iconOnly());

    expect(iconOnly()).not.toHaveAttribute("title");
    expect(iconOnly()).toHaveAttribute("data-tooltip", "Refresh the list");
  });

  it("keeps an icon-only element named by giving it the title as aria-label", () => {
    renderHost();

    fireEvent.pointerOver(iconOnly());

    expect(screen.getByRole("button", { name: "Refresh the list" })).toBe(iconOnly());
  });

  it("leaves the name of an element with its own text alone", () => {
    renderHost();

    fireEvent.pointerOver(screen.getByRole("button", { name: "Sort" }));

    expect(screen.getByRole("button", { name: "Sort" })).not.toHaveAttribute("aria-label");
  });

  it("shows the next element's tooltip at once while one is showing", async () => {
    renderHost();
    fireEvent.pointerOver(iconOnly());
    await wait(450);

    fireEvent.pointerOver(second());

    expect(screen.getByRole("tooltip")).toHaveTextContent("Open the settings");
  });

  it("shows the next element's tooltip at once shortly after one was hidden", async () => {
    renderHost();
    fireEvent.pointerOver(iconOnly());
    await wait(450);
    fireEvent.pointerOver(screen.getByText("No tip here"));
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    await wait(200);

    fireEvent.pointerOver(second());

    expect(screen.getByRole("tooltip")).toHaveTextContent("Open the settings");
  });

  it("waits the full delay again once the tooltips have been gone for a while", async () => {
    renderHost();
    fireEvent.pointerOver(iconOnly());
    await wait(450);
    fireEvent.pointerOver(screen.getByText("No tip here"));
    await wait(1000);

    fireEvent.pointerOver(second());

    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  describe("text cut off with an ellipsis", () => {
    const TITLE = "A very long issue title that does not fit";

    /** jsdom has no layout: the element reports the widths a browser would give it. */
    function ellipsis(element: HTMLElement, scrollWidth: number, clientWidth: number) {
      element.style.textOverflow = "ellipsis";
      Object.defineProperty(element, "scrollWidth", { configurable: true, value: scrollWidth });
      Object.defineProperty(element, "clientWidth", { configurable: true, value: clientWidth });
      return element;
    }

    const hover = async (element: Element) => {
      fireEvent.pointerOver(element);
      await wait(450);
    };

    it("shows its full text as the tooltip", async () => {
      render(
        <>
          <span>{TITLE}</span>
          <TooltipHost />
        </>,
      );

      await hover(ellipsis(screen.getByText(TITLE), 300, 100));

      expect(screen.getByRole("tooltip")).toHaveTextContent(TITLE);
    });

    it("shows no tooltip when the text fits", async () => {
      render(
        <>
          <span>{TITLE}</span>
          <TooltipHost />
        </>,
      );

      await hover(ellipsis(screen.getByText(TITLE), 100, 100));

      expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    });

    it("takes precedence over a title around it", async () => {
      render(
        <>
          <button type="button" title="Open the issue">
            <span>{TITLE}</span>
          </button>
          <TooltipHost />
        </>,
      );

      await hover(ellipsis(screen.getByText(TITLE), 300, 100));

      expect(screen.getByRole("tooltip")).toHaveTextContent(TITLE);
    });

    it("falls back to the title when the text fits", async () => {
      render(
        <>
          <button type="button" title="Open the issue">
            <span>{TITLE}</span>
          </button>
          <TooltipHost />
        </>,
      );

      await hover(ellipsis(screen.getByText(TITLE), 100, 100));

      expect(screen.getByRole("tooltip")).toHaveTextContent("Open the issue");
    });

    it("shows the cut-off text when the pointer is on an element inside it", async () => {
      render(
        <>
          <span>
            <b>Bold</b> {TITLE}
          </span>
          <TooltipHost />
        </>,
      );
      const bold = screen.getByText("Bold");
      ellipsis(bold.parentElement!, 300, 100);

      await hover(bold);

      expect(screen.getByRole("tooltip")).toHaveTextContent(`Bold ${TITLE}`);
    });

    it("ignores a cut-off text more than three levels above the pointer", async () => {
      render(
        <>
          <div>
            <div>
              <div>
                <div>
                  <i>Deep</i>
                </div>
              </div>
            </div>
          </div>
          <TooltipHost />
        </>,
      );
      const deep = screen.getByText("Deep");
      ellipsis(deep.parentElement!.parentElement!.parentElement!.parentElement!, 300, 100);

      await hover(deep);

      expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    });
  });

  it.each([
    ["a press", (el: HTMLElement) => fireEvent.pointerDown(el)],
    ["a key", (el: HTMLElement) => fireEvent.keyDown(el, { key: "a" })],
    ["leaving the element", () => fireEvent.pointerOver(screen.getByText("No tip here"))],
    ["scrolling", (el: HTMLElement) => fireEvent.wheel(el)],
  ])("hides the tooltip on %s", async (_name, hide) => {
    renderHost();
    fireEvent.pointerOver(iconOnly());
    await wait(450);
    expect(screen.getByRole("tooltip")).toBeInTheDocument();

    hide(iconOnly());

    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });
});

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

import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef, useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Popover } from "./Popover";

function Harness({ onClose }: { onClose: () => void }) {
  const anchor = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  return (
    <>
      <button ref={anchor} type="button" onClick={() => setOpen((o) => !o)}>
        Columns
      </button>
      <button type="button">Elsewhere</button>
      {open && (
        <Popover
          label="Columns for this view"
          anchor={anchor}
          className="demo"
          onClose={() => {
            onClose();
            setOpen(false);
          }}
        >
          <button type="button">First choice</button>
          <button type="button">Second choice</button>
        </Popover>
      )}
    </>
  );
}

async function openPopover(onClose = vi.fn()) {
  render(<Harness onClose={onClose} />);
  await userEvent.click(screen.getByRole("button", { name: "Columns" }));
  return onClose;
}

describe("Popover", () => {
  it("moves focus to the first control in the panel when it opens", async () => {
    await openPopover();

    expect(screen.getByRole("dialog", { name: "Columns for this view" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "First choice" })).toHaveFocus();
  });

  it("closes on Escape and returns focus to the anchor button", async () => {
    const onClose = await openPopover();

    await userEvent.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Columns" })).toHaveFocus();
  });

  it("closes when the mouse goes down outside the panel", async () => {
    const onClose = await openPopover();

    fireEvent.mouseDown(screen.getByRole("button", { name: "Elsewhere" }));

    expect(onClose).toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("stays open when the mouse goes down inside the panel", async () => {
    const onClose = await openPopover();

    fireEvent.mouseDown(screen.getByRole("button", { name: "Second choice" }));

    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("leaves a press on the anchor to the anchor's own toggle", async () => {
    const onClose = await openPopover();

    fireEvent.mouseDown(screen.getByRole("button", { name: "Columns" }));

    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });
});

describe("Popover placement", () => {
  const PANEL = { left: 0, right: 1000 };

  /** Lays the content panel out at `PANEL` and the popover's box at `left`..`right`, `width` wide before any scaling. */
  function layout(box: { left: number; right: number; width: number }) {
    const rect = (left: number, right: number) => ({ left, right, top: 0, bottom: 40, width: right - left, height: 40, x: left, y: 0, toJSON: () => ({}) });
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      if (this.classList.contains("popover")) return rect(box.left, box.right);
      if (this.classList.contains("content-panel")) return rect(PANEL.left, PANEL.right);
      return rect(0, 0);
    });
    vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockReturnValue(box.width);
  }

  function renderPopover(inPanel = true) {
    const popover = (
      <Popover label="Columns for this view" anchor={{ current: null }} className="demo" onClose={vi.fn()}>
        <button type="button">First choice</button>
      </Popover>
    );
    render(inPanel ? <div className="content-panel">{popover}</div> : popover);
    return screen.getByRole("dialog", { name: "Columns for this view" });
  }

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("keeps its own placement while it fits inside the content panel", () => {
    layout({ left: 100, right: 400, width: 300 });

    const dialog = renderPopover();

    expect(dialog.style.left).toBe("");
    expect(dialog.style.right).toBe("");
  });

  it.each([
    ["past the right edge", { left: 800, right: 1100, width: 300 }, { left: "auto", right: "0px", origin: "top right" }],
    ["past the left edge", { left: -50, right: 250, width: 300 }, { left: "0px", right: "auto", origin: "top left" }],
    ["within the edge margin of the right side", { left: 700, right: 995, width: 295 }, { left: "auto", right: "0px", origin: "top right" }],
    ["past the right edge only at full size, while the opening animation still shrinks it", { left: 810, right: 1000, width: 300 }, { left: "auto", right: "0px", origin: "top right" }],
  ])("opens toward the other side of its button when it reaches %s", (_case, box, expected) => {
    layout(box);

    const dialog = renderPopover();

    expect(dialog.style.left).toBe(expected.left);
    expect(dialog.style.right).toBe(expected.right);
    expect(dialog.style.transformOrigin).toBe(expected.origin);
  });

  it("measures against the window when it is not inside a content panel", () => {
    layout({ left: window.innerWidth - 100, right: window.innerWidth + 200, width: 300 });

    const dialog = renderPopover(false);

    expect(dialog.style.right).toBe("0px");
    expect(dialog.style.left).toBe("auto");
  });
});

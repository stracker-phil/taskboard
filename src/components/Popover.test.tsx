import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef, useState } from "react";
import { describe, expect, it, vi } from "vitest";
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

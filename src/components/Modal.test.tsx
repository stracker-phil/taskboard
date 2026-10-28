import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { Modal } from "./Modal";

function Host() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Open
      </button>
      {open && (
        <Modal role="alertdialog" labelledBy="t" onClose={() => setOpen(false)}>
          <h2 id="t">Delete it?</h2>
          <button type="button">First</button>
          <button type="button">Last</button>
        </Modal>
      )}
    </>
  );
}

describe("Modal", () => {
  it("focuses its first control when it opens", async () => {
    render(<Host />);

    await userEvent.click(screen.getByRole("button", { name: "Open" }));

    expect(screen.getByRole("alertdialog", { name: "Delete it?" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "First" })).toHaveFocus();
  });

  it("closes on Escape and returns focus to the opener", async () => {
    render(<Host />);
    await userEvent.click(screen.getByRole("button", { name: "Open" }));

    await userEvent.keyboard("{Escape}");

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open" })).toHaveFocus();
  });

  it("closes when the scrim is pressed", async () => {
    render(<Host />);
    await userEvent.click(screen.getByRole("button", { name: "Open" }));

    await userEvent.pointer({ keys: "[MouseLeft]", target: screen.getByRole("alertdialog").parentElement as HTMLElement });

    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("stays open when the sheet itself is pressed", async () => {
    render(<Host />);
    await userEvent.click(screen.getByRole("button", { name: "Open" }));

    await userEvent.click(screen.getByRole("heading", { name: "Delete it?" }));

    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
  });

  it("keeps Tab inside, wrapping from the last control to the first and back", async () => {
    render(<Host />);
    await userEvent.click(screen.getByRole("button", { name: "Open" }));

    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Last" })).toHaveFocus();
    await userEvent.tab();
    expect(screen.getByRole("button", { name: "First" })).toHaveFocus();
    await userEvent.tab({ shift: true });
    expect(screen.getByRole("button", { name: "Last" })).toHaveFocus();
  });
});

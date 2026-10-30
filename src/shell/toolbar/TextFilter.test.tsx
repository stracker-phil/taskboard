import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { TextFilter } from "./TextFilter";

function Harness({ onEnter, onChange, label }: { onEnter?: (text: string, backwards: boolean) => void; onChange?: (text: string) => void; label?: string }) {
  const [value, setValue] = useState("");
  return (
    <TextFilter
      value={value}
      label={label}
      onEnter={onEnter}
      onChange={(text) => {
        setValue(text);
        onChange?.(text);
      }}
    />
  );
}

const field = (name = "Filter this view") => screen.getByRole("textbox", { name });

describe("TextFilter", () => {
  it("is named Filter this view by default", () => {
    render(<Harness />);

    expect(field()).toHaveAttribute("placeholder", "Filter this view");
  });

  it("takes its name from the label", () => {
    render(<Harness label="Find in this issue" />);

    expect(field("Find in this issue")).toBeInTheDocument();
    expect(screen.queryByRole("textbox", { name: "Filter this view" })).not.toBeInTheDocument();
  });

  describe("Enter", () => {
    it("reports the typed text, not backwards", async () => {
      const onEnter = vi.fn();
      render(<Harness onEnter={onEnter} />);

      await userEvent.type(field(), "payment{Enter}");

      expect(onEnter).toHaveBeenCalledWith("payment", false);
    });

    it("reports backwards with Shift", async () => {
      const onEnter = vi.fn();
      render(<Harness onEnter={onEnter} />);

      await userEvent.type(field(), "payment{Shift>}{Enter}{/Shift}");

      expect(onEnter).toHaveBeenCalledWith("payment", true);
    });
  });

  describe("Escape", () => {
    it("clears the text and reports an empty one", async () => {
      const onChange = vi.fn();
      render(<Harness onChange={onChange} />);
      await userEvent.type(field(), "payment");

      await userEvent.keyboard("{Escape}");

      expect(field()).toHaveValue("");
      expect(onChange).toHaveBeenLastCalledWith("");
    });

    it("keeps the focus while it clears", async () => {
      render(<Harness />);
      await userEvent.type(field(), "payment{Escape}");

      expect(field()).toHaveFocus();
    });

    it("leaves an empty field", async () => {
      render(<Harness />);
      await userEvent.click(field());
      expect(field()).toHaveFocus();

      await userEvent.keyboard("{Escape}");

      expect(field()).not.toHaveFocus();
    });
  });
});

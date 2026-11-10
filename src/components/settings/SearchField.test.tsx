import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { SearchField } from "./SearchField";

function Harness({ initial = "", onChange }: { initial?: string; onChange?: (value: string) => void }) {
  const [value, setValue] = useState(initial);
  return (
    <SearchField
      label="Search people"
      value={value}
      onChange={(next) => {
        setValue(next);
        onChange?.(next);
      }}
    />
  );
}

describe("SearchField", () => {
  it("is a textbox named and hinted by its label", () => {
    render(<Harness />);

    expect(screen.getByRole("textbox", { name: "Search people" })).toHaveAttribute("placeholder", "Search people");
  });

  it("shows the given value", () => {
    render(<Harness initial="Ada" />);

    expect(screen.getByRole("textbox", { name: "Search people" })).toHaveValue("Ada");
  });

  it("reports the new text as the user types", async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);

    await userEvent.type(screen.getByRole("textbox", { name: "Search people" }), "Ben");

    expect(onChange).toHaveBeenLastCalledWith("Ben");
    expect(screen.getByRole("textbox", { name: "Search people" })).toHaveValue("Ben");
  });
});

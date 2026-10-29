import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Style } from "../../api";
import { type PillVariant, StatusPill } from "./StatusPill";

describe("StatusPill", () => {
  it("shows the status name as text only, with no marker inside", () => {
    render(<StatusPill style="review" name="In Review" />);

    const pill = screen.getByText("In Review");
    expect(pill).toHaveClass("status-pill");
    expect(pill.childElementCount).toBe(0);
  });

  it.each<Style>(["backlog", "todo", "progress", "review", "blocked", "done"])("carries the %s style class", (style) => {
    render(<StatusPill style={style} name="Status" />);

    expect(screen.getByText("Status")).toHaveClass(`status-pill--${style}`);
  });

  it.each<PillVariant>(["peek", "activity", "tab", "vertical"])("adds the %s variant class", (variant) => {
    render(<StatusPill style="done" name="Status" variant={variant} />);

    expect(screen.getByText("Status")).toHaveClass(`status-pill--${variant}`);
  });

  it("adds no variant class for the default variant", () => {
    render(<StatusPill style="done" name="Status" />);

    expect(screen.getByText("Status").className).toBe("status-pill status-pill--done");
  });
});

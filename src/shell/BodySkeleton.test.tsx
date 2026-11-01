import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BodySkeleton } from "./BodySkeleton";

describe("BodySkeleton", () => {
  it.each(["list", "table", "board"] as const)("announces a busy region labelled Loading issues for the %s layout", (layout) => {
    render(<BodySkeleton layout={layout} />);

    expect(screen.getByLabelText("Loading issues")).toHaveAttribute("aria-busy", "true");
  });

  it.each(["list", "table"] as const)("stands in with 8 rows for the %s layout", (layout) => {
    render(<BodySkeleton layout={layout} />);

    expect(screen.getByLabelText("Loading issues").children).toHaveLength(8);
  });

  it("stands in with 3 columns of 3 cards for the board layout", () => {
    render(<BodySkeleton layout="board" />);

    const columns = Array.from(screen.getByLabelText("Loading issues").children);
    expect(columns).toHaveLength(3);
    expect(columns.map((column) => column.children.length)).toEqual([3, 3, 3]);
  });
});

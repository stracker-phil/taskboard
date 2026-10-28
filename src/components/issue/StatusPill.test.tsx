import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Style } from "../../api";
import { StatusPill } from "./StatusPill";
import { StatusRing } from "./StatusRing";

describe("StatusPill", () => {
  it("shows the status name with a ring inside", () => {
    const { container } = render(<StatusPill style="review" name="In Review" />);

    expect(screen.getByText("In Review")).toBeInTheDocument();
    expect(container.querySelector(".status-ring")).not.toBeNull();
  });
});

describe("StatusRing", () => {
  it("is the dashed todo ring for To do", () => {
    const { container } = render(<StatusRing style="todo" />);

    expect(container.querySelector(".status-ring")).toHaveClass("status-ring--todo");
  });

  it.each<Style>(["progress", "review", "blocked", "done"])("is not the todo ring for %s", (style) => {
    const { container } = render(<StatusRing style={style} />);

    expect(container.querySelector(".status-ring")).not.toHaveClass("status-ring--todo");
    expect(container.querySelector(".status-ring")).toHaveClass(`status-ring--${style}`);
  });

  it("is hidden from assistive technology", () => {
    const { container } = render(<StatusRing style="done" />);

    expect(container.querySelector(".status-ring")).toHaveAttribute("aria-hidden", "true");
  });
});

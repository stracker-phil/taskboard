import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Style } from "../../api";
import { StatusSquare } from "./StatusSquare";

describe("StatusSquare", () => {
  it.each<Style>(["backlog", "todo", "progress", "review", "blocked", "done"])("renders one marker with the %s style class", (style) => {
    const { container } = render(<StatusSquare style={style} />);

    expect(container.children).toHaveLength(1);
    expect(container.firstElementChild).toHaveClass("status-square", `status-square--${style}`);
  });

  it("is hidden from assistive technology", () => {
    const { container } = render(<StatusSquare style="done" />);

    expect(container.firstElementChild).toHaveAttribute("aria-hidden", "true");
  });
});

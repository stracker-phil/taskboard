import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CommentCount } from "./CommentCount";

describe("CommentCount", () => {
  it("renders nothing for no comments", () => {
    const { container } = render(<CommentCount count={0} />);

    expect(container).toBeEmptyDOMElement();
  });

  it.each([
    [1, "1 comment"],
    [3, "3 comments"],
  ])("labels %i comments as %s", (count, label) => {
    render(<CommentCount count={count} />);

    expect(screen.getByLabelText(label)).toHaveTextContent(String(count));
  });
});

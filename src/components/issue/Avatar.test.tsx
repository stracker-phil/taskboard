import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ALEX, SAM } from "../../test/issues";
import { Avatar, AvatarStack } from "./Avatar";

describe("Avatar", () => {
  it("shows the initials and is named after the person", () => {
    render(<Avatar person={ALEX} />);

    expect(screen.getByRole("img", { name: "Alex Morgan" })).toHaveTextContent("AM");
  });
});

describe("AvatarStack", () => {
  it("shows one avatar per person in order", () => {
    render(<AvatarStack people={[ALEX, SAM]} />);

    expect(screen.getAllByRole("img").map((a) => a.getAttribute("aria-label"))).toEqual(["Alex Morgan", "Sam Lee"]);
  });

  it("shows nothing for nobody", () => {
    render(<AvatarStack people={[]} />);

    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });
});

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SegmentedControl } from "./SegmentedControl";

describe("SegmentedControl", () => {
  it("presses the button of the current value, capitalized", () => {
    render(<SegmentedControl label="Token kind" options={["classic", "scoped"] as const} value="scoped" onChange={() => {}} />);

    const group = screen.getByRole("group", { name: "Token kind" });
    expect(group).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Scoped" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "Classic" })).toHaveAttribute("aria-pressed", "false");
  });

  it("presses none for a null value", () => {
    render(<SegmentedControl label="Token kind" options={["classic", "scoped"] as const} value={null} onChange={() => {}} />);

    expect(screen.getAllByRole("button").map((b) => b.getAttribute("aria-pressed"))).toEqual(["false", "false"]);
  });

  it("reports the chosen option", async () => {
    const onChange = vi.fn();
    render(<SegmentedControl label="Token kind" options={["classic", "scoped"] as const} value="classic" onChange={onChange} />);

    await userEvent.click(screen.getByRole("button", { name: "Scoped" }));

    expect(onChange).toHaveBeenCalledWith("scoped");
  });

  it("reads custom labels instead of the option names", () => {
    render(<SegmentedControl label="Mode" options={["a", "b"] as const} value="a" labels={{ a: "Alpha", b: "Beta" }} onChange={() => {}} />);

    expect(screen.getByRole("button", { name: "Alpha" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Beta" })).toBeInTheDocument();
  });

  it("disables every button when disabled", () => {
    render(<SegmentedControl label="Mode" options={["a", "b"] as const} value="a" disabled onChange={() => {}} />);

    expect(screen.getByRole("button", { name: "A" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "B" })).toBeDisabled();
  });
});

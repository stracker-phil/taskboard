import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef, type ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import type { MeRule, UserField } from "../../api";
import { MePopover } from "./MePopover";

const RULES: MeRule[] = [
  { rule: { kind: "field", field: "reporter" }, on: true },
  { rule: { kind: "field", field: "customfield_10099" }, on: false },
  { rule: { kind: "mentioned" }, on: false },
  { rule: { kind: "subtasks" }, on: true },
];
const FIELDS: UserField[] = [{ field: "reporter", name: "Reporter" }];

function Harness(props: Partial<ComponentProps<typeof MePopover>>) {
  const anchor = useRef<HTMLButtonElement>(null);
  return (
    <>
      <button ref={anchor} type="button">
        Anchor
      </button>
      <MePopover rules={RULES} fields={FIELDS} onRule={vi.fn()} everywhere={false} onScope={vi.fn()} onAddFields={vi.fn()} anchor={anchor} onClose={vi.fn()} {...props} />
    </>
  );
}

describe("MePopover", () => {
  it("lists each rule by its label, with a field the cache no longer knows going by its id", () => {
    render(<Harness />);

    expect(screen.getByRole("button", { name: "Reporter" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "customfield_10099" })).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByRole("button", { name: "Mentioned in a comment" })).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByRole("button", { name: "Subtasks of my issues" })).toHaveAttribute("aria-pressed", "true");
  });

  it("hands the rule with the opposite switch position when a row is clicked", async () => {
    const onRule = vi.fn();
    render(<Harness onRule={onRule} />);

    await userEvent.click(screen.getByRole("button", { name: "Mentioned in a comment" }));
    await userEvent.click(screen.getByRole("button", { name: "Reporter" }));

    expect(onRule).toHaveBeenCalledWith(RULES[2], true);
    expect(onRule).toHaveBeenCalledWith(RULES[0], false);
  });

  it.each([
    [false, "Only this view", "All views"],
    [true, "All views", "Only this view"],
  ])("marks the scope as pressed when everywhere is %s", (everywhere, pressed, other) => {
    render(<Harness everywhere={everywhere} />);

    expect(screen.getByRole("button", { name: new RegExp(`^${pressed}`) })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: new RegExp(`^${other}`) })).toHaveAttribute("aria-pressed", "false");
  });

  it.each([
    ["All views", true],
    ["Only this view", false],
  ])("reports %s as the chosen scope", async (name, everywhere) => {
    const onScope = vi.fn();
    render(<Harness everywhere={!everywhere} onScope={onScope} />);

    await userEvent.click(screen.getByRole("button", { name: new RegExp(`^${name}`) }));

    expect(onScope).toHaveBeenCalledWith(everywhere);
  });

  it("asks to open the field settings from Fields…", async () => {
    const onAddFields = vi.fn();
    render(<Harness onAddFields={onAddFields} />);

    await userEvent.click(screen.getByRole("button", { name: "Fields…" }));

    expect(onAddFields).toHaveBeenCalled();
  });
});

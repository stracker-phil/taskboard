import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { InlineRename } from "./InlineRename";

function renderField(value = "Epics") {
  const onSave = vi.fn();
  const onDone = vi.fn();
  render(
    <>
      <InlineRename value={value} label="Filter list name" onSave={onSave} onDone={onDone} />
      <button type="button">Elsewhere</button>
    </>,
  );
  return { field: screen.getByRole("textbox", { name: "Filter list name" }), onSave, onDone };
}

describe("InlineRename", () => {
  it("starts focused with the current name", () => {
    const { field } = renderField("Epics");

    expect(field).toHaveValue("Epics");
    expect(field).toHaveFocus();
  });

  it("saves the trimmed name and finishes on enter", async () => {
    const { field, onSave, onDone } = renderField();

    await userEvent.clear(field);
    await userEvent.type(field, "  Backlog  {Enter}");

    expect(onSave).toHaveBeenCalledWith("Backlog");
    expect(onDone).toHaveBeenCalled();
  });

  it("saves the name when the field loses focus", async () => {
    const { field, onSave, onDone } = renderField();

    await userEvent.clear(field);
    await userEvent.type(field, "Backlog");
    await userEvent.click(screen.getByRole("button", { name: "Elsewhere" }));

    expect(onSave).toHaveBeenCalledWith("Backlog");
    expect(onDone).toHaveBeenCalled();
  });

  it("discards the edit and finishes on escape", async () => {
    const { field, onSave, onDone } = renderField();

    await userEvent.clear(field);
    await userEvent.type(field, "Backlog{Escape}");

    expect(onSave).not.toHaveBeenCalled();
    expect(onDone).toHaveBeenCalled();
  });

  it.each([
    ["the name is unchanged", "  Epics  "],
    ["the name is empty", ""],
    ["the name is only spaces", "   "],
  ])("finishes without saving when %s", async (_case, typed) => {
    const { field, onSave, onDone } = renderField("Epics");

    await userEvent.clear(field);
    if (typed) await userEvent.type(field, typed);
    await userEvent.keyboard("{Enter}");

    expect(onSave).not.toHaveBeenCalled();
    expect(onDone).toHaveBeenCalled();
  });
});

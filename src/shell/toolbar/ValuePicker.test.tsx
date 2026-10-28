import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Choice } from "../../api";
import { ValuePicker } from "./ValuePicker";

const CHOICES: Choice[] = [
  { id: "1", name: "To Do" },
  { id: "3", name: "In Progress" },
  { id: "4", name: "Done" },
];

function setup(selected: string[] = [], choices: Choice[] = CHOICES) {
  const onChange = vi.fn();
  render(
    <div>
      <p>Elsewhere</p>
      <ValuePicker id="values" choices={choices} selected={selected} onChange={onChange} />
    </div>,
  );
  return { onChange, field: screen.getByRole("button", { name: /./, expanded: false }) };
}

const optionNames = () => screen.getAllByRole("option").map((o) => o.textContent);

describe("ValuePicker", () => {
  describe("the field", () => {
    it("asks to choose when nothing is picked", () => {
      const { field } = setup();

      expect(field).toHaveTextContent("Choose…");
      expect(field).toHaveAttribute("aria-haspopup", "listbox");
    });

    it("shows a single picked name without a count", () => {
      const { field } = setup(["3"]);

      expect(field).toHaveTextContent("In Progress");
      expect(field).not.toHaveTextContent("1");
    });

    it("joins several picked names and shows how many", () => {
      const { field } = setup(["1", "4"]);

      expect(field).toHaveTextContent("To Do, Done");
      expect(field).toHaveTextContent("2");
    });

    it("falls back to the raw value for a value no choice has", () => {
      const { field } = setup(["gone"]);

      expect(field).toHaveTextContent("gone");
    });
  });

  describe("the panel", () => {
    it("opens on click with the search box focused", async () => {
      const { field } = setup();

      await userEvent.click(field);

      expect(field).toHaveAttribute("aria-expanded", "true");
      expect(screen.getByRole("listbox")).toHaveAttribute("aria-multiselectable", "true");
      expect(screen.getByLabelText("Search values")).toHaveFocus();
      expect(optionNames()).toEqual(["To Do", "In Progress", "Done"]);
    });

    it("marks the picked options as selected and lists them first", async () => {
      const { field } = setup(["4", "1"]);

      await userEvent.click(field);

      expect(optionNames()).toEqual(["To Do", "Done", "In Progress"]);
      expect(screen.getByRole("option", { name: "Done" })).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("option", { name: "In Progress" })).toHaveAttribute("aria-selected", "false");
    });

    it("closes when the field is clicked again", async () => {
      const { field } = setup();
      await userEvent.click(field);

      await userEvent.click(field);

      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    });

    it("closes on a mousedown outside", async () => {
      const { field } = setup();
      await userEvent.click(field);

      await userEvent.click(screen.getByText("Elsewhere"));

      expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    });

    it("stays open on a mousedown inside", async () => {
      const { field } = setup();
      await userEvent.click(field);

      await userEvent.click(screen.getByLabelText("Search values"));

      expect(screen.getByRole("listbox")).toBeInTheDocument();
    });
  });

  describe("picking", () => {
    it("reports a ticked value together with the picked ones, in the choices' order", async () => {
      const { field, onChange } = setup(["4"]);
      await userEvent.click(field);

      await userEvent.click(screen.getByRole("option", { name: "To Do" }));

      expect(onChange).toHaveBeenLastCalledWith(["1", "4"]);
    });

    it("reports the remaining values when a picked one is unticked", async () => {
      const { field, onChange } = setup(["1", "4"]);
      await userEvent.click(field);

      await userEvent.click(screen.getByRole("option", { name: "To Do" }));

      expect(onChange).toHaveBeenLastCalledWith(["4"]);
    });

    it("reports an empty list when the last value is unticked", async () => {
      const { field, onChange } = setup(["3"]);
      await userEvent.click(field);

      await userEvent.click(screen.getByRole("option", { name: "In Progress" }));

      expect(onChange).toHaveBeenLastCalledWith([]);
    });
  });

  describe("search", () => {
    it("filters the options by name, ignoring case", async () => {
      const { field } = setup();
      await userEvent.click(field);

      await userEvent.type(screen.getByLabelText("Search values"), "pRoG");

      expect(optionNames()).toEqual(["In Progress"]);
    });

    it("says so when no value matches", async () => {
      const { field } = setup();
      await userEvent.click(field);

      await userEvent.type(screen.getByLabelText("Search values"), "zzz");

      expect(screen.getByText("No value matches.")).toBeInTheDocument();
      expect(screen.queryAllByRole("option")).toHaveLength(0);
    });
  });
});

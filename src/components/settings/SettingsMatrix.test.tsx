import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { type MatrixCell, type MatrixColumn, type MatrixRow, SettingsMatrix } from "./SettingsMatrix";

const columns: MatrixColumn[] = [
  { key: "read", header: "Read" },
  { key: "write", header: "Write" },
];

const cell = (label: string, patch: Partial<MatrixCell> = {}): MatrixCell => ({ checked: false, label, ...patch });

function rows(patch: Partial<Record<string, Partial<MatrixCell>>> = {}): MatrixRow[] {
  return [
    { key: "ada", header: "Ada Example", cells: [cell("Ada reads", patch.adaRead), cell("Ada writes", patch.adaWrite)] },
    { key: "ben", header: "Ben Sample", cells: [cell("Ben reads", patch.benRead), cell("Ben writes", patch.benWrite)] },
  ];
}

describe("SettingsMatrix", () => {
  const onToggle = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the corner, one column header per column and one row header per row", () => {
    render(<SettingsMatrix corner="Person" columns={columns} rows={rows()} onToggle={onToggle} />);

    expect(screen.getByRole("table")).toBeInTheDocument();
    expect(screen.getAllByRole("columnheader").map((h) => h.textContent)).toEqual(["Person", "Read", "Write"]);
    expect(screen.getAllByRole("rowheader").map((h) => h.textContent)).toEqual(["Ada Example", "Ben Sample"]);
  });

  it("names each box by its label and reflects checked", () => {
    render(<SettingsMatrix corner="Person" columns={columns} rows={rows({ adaRead: { checked: true } })} onToggle={onToggle} />);

    expect(screen.getByRole("checkbox", { name: "Ada reads" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Ada writes" })).not.toBeChecked();
  });

  it("reports the row key, column key and new value when a box is clicked", async () => {
    render(<SettingsMatrix corner="Person" columns={columns} rows={rows({ benWrite: { checked: true } })} onToggle={onToggle} />);

    await userEvent.click(screen.getByRole("checkbox", { name: "Ada writes" }));
    expect(onToggle).toHaveBeenLastCalledWith("ada", "write", true);

    await userEvent.click(screen.getByRole("checkbox", { name: "Ben writes" }));
    expect(onToggle).toHaveBeenLastCalledWith("ben", "write", false);
  });

  it("keeps the row order when a box gets ticked", () => {
    const { rerender } = render(<SettingsMatrix corner="Person" columns={columns} rows={rows()} onToggle={onToggle} />);
    const order = () => screen.getAllByRole("rowheader").map((h) => h.textContent);
    expect(order()).toEqual(["Ada Example", "Ben Sample"]);

    rerender(<SettingsMatrix corner="Person" columns={columns} rows={rows({ benRead: { checked: true } })} onToggle={onToggle} />);

    expect(order()).toEqual(["Ada Example", "Ben Sample"]);
    expect(screen.getByRole("checkbox", { name: "Ben reads" })).toBeChecked();
  });

  it("renders rows in the order given", () => {
    render(<SettingsMatrix corner="Person" columns={columns} rows={[...rows()].reverse()} onToggle={onToggle} />);

    expect(screen.getAllByRole("rowheader").map((h) => h.textContent)).toEqual(["Ben Sample", "Ada Example"]);
  });

  it("renders no box for a column the row has no cell for", () => {
    const short: MatrixRow[] = [{ key: "ada", header: "Ada Example", cells: [cell("Ada reads")] }];
    render(<SettingsMatrix corner="Person" columns={columns} rows={short} onToggle={onToggle} />);

    const row = screen.getByRole("row", { name: /Ada Example/ });
    expect(within(row).getAllByRole("checkbox")).toHaveLength(1);
    expect(screen.queryByRole("checkbox", { name: "Ada writes" })).not.toBeInTheDocument();
  });

  it("renders an empty cell without a box for an undefined cell", () => {
    const gap: MatrixRow[] = [{ key: "ada", header: "Ada Example", cells: [undefined, cell("Ada writes")] }];
    render(<SettingsMatrix corner="Person" columns={columns} rows={gap} onToggle={onToggle} />);

    const row = screen.getByRole("row", { name: /Ada Example/ });
    expect(within(row).getAllByRole("cell")).toHaveLength(2);
    expect(within(row).getAllByRole("checkbox").map((c) => c.getAttribute("aria-label"))).toEqual(["Ada writes"]);
    expect(screen.queryByRole("checkbox", { name: "Ada reads" })).not.toBeInTheDocument();
  });

  it("shows the hint as the title of the cell", () => {
    const hinted: MatrixRow[] = [{ key: "ada", header: "Ada Example", cells: [cell("Ada reads", { hint: "Inherited from the team" }), cell("Ada writes")] }];
    render(<SettingsMatrix corner="Person" columns={columns} rows={hinted} onToggle={onToggle} />);

    expect(screen.getByRole("checkbox", { name: "Ada reads" }).closest("td")).toHaveAttribute("title", "Inherited from the team");
    expect(screen.getByRole("checkbox", { name: "Ada writes" }).closest("td")).not.toHaveAttribute("title");
  });

  it("shows a column title on its header", () => {
    const titled: MatrixColumn[] = [{ key: "read", header: "Read", title: "Can view issues" }];
    render(<SettingsMatrix corner="Person" columns={titled} rows={[]} onToggle={onToggle} />);

    expect(screen.getByRole("columnheader", { name: "Read" })).toHaveAttribute("title", "Can view issues");
  });
});

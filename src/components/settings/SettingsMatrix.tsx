import type { ReactNode } from "react";

export interface MatrixColumn {
  key: string;
  header: ReactNode;
  title?: string;
}

export interface MatrixCell {
  checked: boolean;
  label: string;
  hint?: string;
  /** Unticked, the box is dimmed: the row has nothing to do with this column yet. */
  quiet?: boolean;
}

export interface MatrixRow {
  key: string;
  header: ReactNode;
  cells: MatrixCell[];
}

/** A table of checkboxes with a sticky header row and first column. Rows render in the order given, so a caller
 *  keeps them still while boxes are ticked. */
interface SettingsMatrixProps {
  corner: string;
  columns: MatrixColumn[];
  rows: MatrixRow[];
  onToggle: (rowKey: string, columnKey: string, next: boolean) => void;
}

export function SettingsMatrix({ corner, columns, rows, onToggle }: SettingsMatrixProps) {
  return (
    <div className="settings-matrix scroller">
      <table>
        <thead>
          <tr>
            <th scope="col" className="settings-matrix__row">
              {corner}
            </th>
            {columns.map((column) => (
              <th key={column.key} scope="col" title={column.title} className="settings-matrix__column">
                {column.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.key}>
              <th scope="row" className="settings-matrix__row">
                <span className="settings-matrix__name">{row.header}</span>
              </th>
              {columns.map((column, i) => {
                const cell = row.cells[i];
                const cellClass = cell?.quiet ? "settings-matrix__cell settings-matrix__cell--quiet" : "settings-matrix__cell";
                return (
                  <td key={column.key} className={cellClass} title={cell?.hint}>
                    {cell && (
                      <input
                        type="checkbox"
                        className="settings-matrix__check"
                        aria-label={cell.label}
                        checked={cell.checked}
                        onChange={(e) => onToggle(row.key, column.key, e.target.checked)}
                      />
                    )}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

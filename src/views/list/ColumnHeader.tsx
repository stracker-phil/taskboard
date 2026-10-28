import { useColumnsCatalogue } from "../../queries/views";
import { columnDef } from "./columns";

export function ColumnHeader({ columns }: { columns: string[] }) {
  const { data: catalogue = [] } = useColumnsCatalogue();
  const name = (id: string) => catalogue.find((c) => c.id === id)?.name ?? id;
  return (
    <div className="list-columns" role="row">
      <span className="list-columns__name" role="columnheader">
        Name
      </span>
      {columns.map((id) => (
        <span key={id} role="columnheader" style={{ width: columnDef(id).width }}>
          {name(id)}
        </span>
      ))}
    </div>
  );
}

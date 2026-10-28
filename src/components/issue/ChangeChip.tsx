/** What changed since the owner last looked ("2 new comments"). */
export function ChangeChip({ note }: { note: string }) {
  return <span className="change-chip">{note}</span>;
}

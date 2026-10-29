import type { Label } from "../../api";

/** An issue's labels as tags: the Labels column, and after the title when the view shows them inline. */
export function LabelChips({ labels, className }: { labels: Label[]; className: string }) {
  if (labels.length === 0) return null;
  return (
    <span className={className}>
      {labels.map((label) => (
        <span key={label.name} className={`label-chip label-chip--${label.tint}`}>
          {label.name}
        </span>
      ))}
    </span>
  );
}

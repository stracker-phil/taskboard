/** The subtask glyph in lucide's style: a circle top left, a line down that bends right into a circle bottom
 *  right. */
export function SubtaskIcon({ size = 14 }: { size?: number }) {
  return (
    <svg
      className="subtask-icon"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="6" cy="5" r="2.5" />
      <path d="M6 7.5V13a5 5 0 0 0 5 5h4.5" />
      <circle cx="18" cy="18" r="2.5" />
    </svg>
  );
}

/** An issue's direct children (an epic's stories, a story's subtasks) after its title, in the style of the
 *  comment count. */
export function ChildCount({ count }: { count: number }) {
  if (count <= 0) return null;
  const label = count === 1 ? "1 child issue" : `${count} child issues`;
  return (
    <span className="subtask-count" aria-label={label} title={label}>
      <SubtaskIcon size={12} />
      {count}
    </span>
  );
}

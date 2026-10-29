const FLAGS = ["highest", "high", "medium", "low", "low"] as const;

/** The flag of a priority level (1 Highest … 5 Lowest); Lowest shares Low's color. */
export function PriorityFlag({ level, size = 14 }: { level: number; size?: number }) {
  const flag = FLAGS[Math.min(Math.max(level, 1), 5) - 1];
  return (
    <svg className={`priority-flag priority-flag--${flag}`} width={size} height={size} viewBox="0 0 24 24" strokeWidth={1.5} strokeLinejoin="round" aria-hidden="true">
      <path d="M5 21V4h12l-2 4 2 4H5" />
    </svg>
  );
}

import type { ReactNode } from "react";
import { usePresence } from "../lib/motion";

/** Content that grows open and shrinks closed instead of appearing at once; it stays mounted while closing. */
export function Disclosure({ open, children }: { open: boolean; children: ReactNode }) {
  const { mounted, state } = usePresence(open, "--motion-slow");
  if (!mounted) return null;
  return (
    <div className={`disclosure disclosure--${state}`} inert={state === "leaving"}>
      <div className="disclosure__inner">{children}</div>
    </div>
  );
}

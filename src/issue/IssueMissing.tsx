import { toAppError } from "../api";

/** Why an issue isn't shown: gone from the cache, or Rust's reason for any other failure. */
export function IssueMissing({ error, className }: { error: unknown; className: string }) {
  const err = toAppError(error);
  return <p className={className}>{err.kind === "not_found" ? "This issue is no longer in the cache." : `Could not load this issue: ${err.message}`}</p>;
}

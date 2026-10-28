import { useState } from "react";
import { toAppError, type AppError } from "../../api";

/** What the last action of a group said, under its buttons. */
export interface ActionResult {
  ok: boolean;
  text: string;
}

/** Field errors by `field.path` (`siteUrl`, `sync.focusedMinutes`, …). */
export type FieldErrors = Record<string, string>;

/** The field error of an `invalid_input` rejection, or null for any other error. */
export function fieldErrorOf(err: AppError): FieldErrors | null {
  return err.kind === "invalid_input" && err.field ? { [err.field.path]: err.field.message } : null;
}

/** Runs one action of a group at a time; `busy` while it runs. The action's rejection reaches `onError` as an
 *  `AppError`. */
export function useBusyAction(): [boolean, (action: () => Promise<void>, onError: (err: AppError) => void) => Promise<void>] {
  const [busy, setBusy] = useState(false);
  const run = async (action: () => Promise<void>, onError: (err: AppError) => void) => {
    setBusy(true);
    try {
      await action();
    } catch (e) {
      onError(toAppError(e));
    } finally {
      setBusy(false);
    }
  };
  return [busy, run];
}

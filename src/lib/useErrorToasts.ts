import { useEffect } from "react";
import { toAppError } from "../api";
import { useStore } from "../state/store";

/** Shows an uncaught error or rejected promise of the page as an error toast (which also logs it). */
export function useErrorToasts() {
  useEffect(() => {
    const report = (message: string) => useStore.getState().toast("error", `Unexpected error: ${message}`);
    const onError = (e: ErrorEvent) => report(e.message);
    const onRejection = (e: PromiseRejectionEvent) => report(toAppError(e.reason).message);
    window.addEventListener("error", onError);
    window.addEventListener("unhandledrejection", onRejection);
    return () => {
      window.removeEventListener("error", onError);
      window.removeEventListener("unhandledrejection", onRejection);
    };
  }, []);
}

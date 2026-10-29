import { useEffect } from "react";
import { toAppError } from "../api";
import { useStore } from "../state/store";

/** A browser notice, not a failure: a resize observer's callback changed a size it observes, so the
 *  rest is delivered next frame. Measured rows trigger it while a panel opens. */
const RESIZE_OBSERVER_NOTICE = /^ResizeObserver loop/;

/** Shows an uncaught error or rejected promise of the page as an error toast (which also logs it). */
export function useErrorToasts() {
  useEffect(() => {
    const report = (message: string) => useStore.getState().toast("error", `Unexpected error: ${message}`);
    const onError = (e: ErrorEvent) => {
      if (!RESIZE_OBSERVER_NOTICE.test(e.message)) report(e.message);
    };
    const onRejection = (e: PromiseRejectionEvent) => report(toAppError(e.reason).message);
    window.addEventListener("error", onError);
    window.addEventListener("unhandledrejection", onRejection);
    return () => {
      window.removeEventListener("error", onError);
      window.removeEventListener("unhandledrejection", onRejection);
    };
  }, []);
}

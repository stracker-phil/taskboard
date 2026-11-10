import { api } from "../api";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

/** Copies the text and confirms it with a toast (`done`); a failure shows an error toast instead. Subscribes to
 *  nothing, so every row can use it. */
export function copyWithToast(text: string, done: string) {
  void attempt(api.copyText(text), "Could not copy").then((copied) => {
    if (copied !== null) useStore.getState().toast("success", done);
  });
}

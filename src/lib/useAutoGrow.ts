import { useLayoutEffect, type RefObject } from "react";

/** Fits the textarea's height to its text; CSS min- and max-height still bound it. */
export function useAutoGrow(ref: RefObject<HTMLTextAreaElement | null>, value: string) {
  useLayoutEffect(() => {
    const field = ref.current;
    if (!field) return;
    field.style.height = "auto";
    const border = field.offsetHeight - field.clientHeight;
    field.style.height = `${field.scrollHeight + border}px`;
  }, [ref, value]);
}

import { useEffect, type RefObject } from "react";

/** Focus moves into `panel` and back to `opener` when it goes; a press outside both, and outside `alsoInside`,
 *  calls `onDismiss`. */
export function useDismiss(panel: RefObject<HTMLElement | null>, opener: RefObject<HTMLElement | null>, onDismiss: () => void, alsoInside: string) {
  useEffect(() => {
    const element = panel.current;
    element?.focus();
    const back = opener.current;
    return () => {
      const active = document.activeElement;
      if (!active || active === document.body || element?.contains(active)) back?.focus();
    };
  }, [panel, opener]);

  useEffect(() => {
    const onPress = (e: MouseEvent) => {
      const target = e.target as Node;
      if (panel.current?.contains(target) || opener.current?.contains(target)) return;
      if (target instanceof Element && target.closest(alsoInside)) return;
      onDismiss();
    };
    window.addEventListener("mousedown", onPress, true);
    return () => window.removeEventListener("mousedown", onPress, true);
  }, [panel, opener, onDismiss, alsoInside]);
}

import { type RefObject, useEffect, useState } from "react";

/** Becomes true once the element has been on screen, and stays so; true at once where nothing can observe it. */
export function useVisible(ref: RefObject<Element | null>): boolean {
  const [visible, setVisible] = useState(() => typeof IntersectionObserver === "undefined");
  useEffect(() => {
    const element = ref.current;
    if (visible || !element) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) setVisible(true);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref, visible]);
  return visible;
}

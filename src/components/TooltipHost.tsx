import { useEffect, useState } from "react";

/** How long the pointer rests before a tooltip shows. */
const SHOW_DELAY_MS = 450;
/** After one tooltip, the next shows at once for this long (moving along a toolbar). */
const WARM_MS = 600;
const GAP = 6;
const EDGE = 8;

interface Tip {
  text: string;
  rect: DOMRect;
}

/** Takes an element's `title` over as `data-tooltip`, so macOS's own small, slow tooltip never shows. An
 *  element named only by its title keeps that name as `aria-label`. */
function adopt(element: Element): string | null {
  const title = element.getAttribute("title");
  if (title) {
    element.removeAttribute("title");
    element.setAttribute("data-tooltip", title);
    if (!element.hasAttribute("aria-label") && !element.textContent?.trim()) element.setAttribute("aria-label", title);
  }
  return element.getAttribute("data-tooltip");
}

const owner = (target: EventTarget | null) => (target instanceof Element ? target.closest("[title], [data-tooltip]") : null);

/** One tooltip for the whole window: styled, after a short rest, at once while moving between controls, and on
 *  keyboard focus. Any element with a `title` gets it. */
export function TooltipHost() {
  const [tip, setTip] = useState<Tip | null>(null);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    let current: Element | null = null;
    let hiddenAt = 0;
    const hide = () => {
      clearTimeout(timer);
      if (current) hiddenAt = Date.now();
      current = null;
      setTip(null);
    };
    const show = (element: Element, delay: number) => {
      const text = adopt(element);
      if (!text) return;
      clearTimeout(timer);
      current = element;
      const reveal = () => {
        if (current === element && element.isConnected) setTip({ text, rect: element.getBoundingClientRect() });
      };
      if (delay === 0) reveal();
      else timer = setTimeout(reveal, delay);
    };
    const warm = () => Date.now() - hiddenAt < WARM_MS;
    const onOver = (e: PointerEvent) => {
      const element = owner(e.target);
      if (element === current) return;
      const wasShowing = current !== null;
      hide();
      if (element) show(element, wasShowing || warm() ? 0 : SHOW_DELAY_MS);
    };
    const onFocus = (e: FocusEvent) => {
      const element = owner(e.target);
      if (element && e.target instanceof Element && e.target.matches(":focus-visible")) show(element, SHOW_DELAY_MS);
    };
    document.addEventListener("pointerover", onOver);
    document.addEventListener("focusin", onFocus);
    for (const event of ["pointerdown", "keydown", "focusout", "wheel"] as const) document.addEventListener(event, hide, true);
    window.addEventListener("blur", hide);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("pointerover", onOver);
      document.removeEventListener("focusin", onFocus);
      for (const event of ["pointerdown", "keydown", "focusout", "wheel"] as const) document.removeEventListener(event, hide, true);
      window.removeEventListener("blur", hide);
    };
  }, []);

  if (!tip) return null;
  // Below the control, centred; above it when there's no room underneath.
  const below = tip.rect.bottom + GAP + 28 < window.innerHeight;
  const left = Math.min(Math.max(tip.rect.left + tip.rect.width / 2, EDGE), window.innerWidth - EDGE);
  const style = below ? { left, top: tip.rect.bottom + GAP } : { left, bottom: window.innerHeight - tip.rect.top + GAP };
  return (
    <div className={below ? "tooltip" : "tooltip tooltip--above"} role="tooltip" style={style}>
      {tip.text}
    </div>
  );
}

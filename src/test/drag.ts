import { fireEvent } from "@testing-library/react";

// jsdom may lack PointerEvent; the stub keeps the mouse fields (clientX, button) that the drag reads.
window.PointerEvent ??= class extends MouseEvent {} as unknown as typeof PointerEvent;

type Modifiers = { metaKey?: boolean; ctrlKey?: boolean; altKey?: boolean; shiftKey?: boolean };

/** The pointer gestures of a drag: a press on an element, moves and the release on the window. */
export const pointer = {
  press: (el: Element, init: Modifiers = {}, x = 10, y = 10) => fireEvent.pointerDown(el, { button: 0, clientX: x, clientY: y, ...init }),
  moveTo: (x: number, y: number) => fireEvent.pointerMove(window, { clientX: x, clientY: y }),
  release: (x = 40, y = 40) => fireEvent.pointerUp(window, { button: 0, clientX: x, clientY: y }),
  /** jsdom has no layout: this is the element the pointer is over from now on (null: over nothing). */
  over: (el: Element | null) => {
    document.elementFromPoint = () => el;
  },
  /** Undoes `over`; call it in `afterEach`. */
  reset: () => {
    Reflect.deleteProperty(document, "elementFromPoint");
  },
};

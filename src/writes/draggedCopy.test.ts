import { afterEach, describe, expect, it } from "vitest";
import { followDraggedCopy } from "./draggedCopy";

describe("followDraggedCopy", () => {
  afterEach(() => {
    document.body.replaceChildren();
  });

  it("draws the copy fully opaque and untranslated at the element's rect, leaving the original's styles alone", () => {
    const view = document.createElement("div");
    const row = document.createElement("div");
    row.textContent = "Fix the demo login";
    row.style.opacity = "0.3";
    row.style.transform = "translateY(120px)";
    row.getBoundingClientRect = () => new DOMRect(40, 200, 600, 32);
    document.body.append(view, row);

    const drag = followDraggedCopy(row, { x: 50, y: 210 }, view, "y");

    const copy = document.body.querySelector<HTMLElement>(".drag-copy")!;
    expect(copy.textContent).toBe("Fix the demo login");
    expect(copy.style.opacity).toBe("");
    expect(copy.style.transform).toBe("");
    expect(copy.style.left).toBe("40px");
    expect(copy.style.top).toBe("200px");
    expect(copy.style.width).toBe("600px");
    expect(copy.style.height).toBe("32px");
    expect(row.style.opacity).toBe("0.3");
    expect(row.style.transform).toBe("translateY(120px)");

    drag.remove();
  });
});

import { beforeEach, describe, expect, it } from "vitest";
import { applyInitialLook } from "./theme";

describe("applyInitialLook", () => {
  beforeEach(() => {
    delete document.documentElement.dataset.scheme;
  });

  it("sets the light scheme on the root element", () => {
    applyInitialLook();
    expect(document.documentElement.dataset.scheme).toBe("light");
  });
});

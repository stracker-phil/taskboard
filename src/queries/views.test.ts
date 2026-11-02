import { describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { NO_VIEW_NARROWING } from "../state/slices/narrowing";
import { view } from "../test/views";
import { requestNarrowing } from "./views";

describe("requestNarrowing", () => {
  const saved = view({ meRules: [{ kind: "mentioned" }] });

  it("sends the view's saved Me rules when none are drafted", () => {
    expect(requestNarrowing(NO_VIEW_NARROWING, saved).meOn).toEqual([{ kind: "mentioned" }]);
  });

  it("sends the drafted Me rules over the saved ones", () => {
    const drafted = { ...NO_VIEW_NARROWING, meRules: [{ kind: "subtasks" } as const] };

    expect(requestNarrowing(drafted, saved).meOn).toEqual([{ kind: "subtasks" }]);
  });

  it("sends a drafted empty set of rules rather than the saved ones", () => {
    expect(requestNarrowing({ ...NO_VIEW_NARROWING, meRules: [] }, saved).meOn).toEqual([]);
  });

  it("sends a drafted null (follow the app) rather than the saved rules", () => {
    expect(requestNarrowing({ ...NO_VIEW_NARROWING, meRules: null }, saved).meOn).toBeNull();
  });

  it("sends null for a view that saved no rules and has no draft", () => {
    expect(requestNarrowing(NO_VIEW_NARROWING, view()).meOn).toBeNull();
  });
});

import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import type { DropTarget, GroupMark } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { ALEX } from "../test/issues";
import { colorClass, DropZone, zoneOf, type Zone } from "./DropZone";
import type { Drag } from "./useIssueDrag";

const target = (key: string, action: string, mark: GroupMark | null = null): DropTarget => ({ key, label: key, action, mark });
const IN_REVIEW = target("3", "Set status to In Review", { kind: "status", style: "review" });

function drag(patch: Partial<Drag> = {}): Drag {
  return {
    issue: { id: "10042", key: "NW-1042", summary: "Fix the login redirect", fromKey: "1" },
    targets: { targets: [IN_REVIEW, target("4", "Set status to Done", { kind: "status", style: "done" })], note: null },
    over: null,
    ...patch,
  };
}

function renderZone(zone: Zone, props: { className?: string } = {}) {
  const { container } = render(
    <QueryClientProvider client={createQueryClient()}>
      <DropZone zone={zone} {...props} />
    </QueryClientProvider>,
  );
  return container.firstElementChild as HTMLElement;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("DropZone", () => {
  it("shows the action of a zone that takes the drop and marks it with the group's key", () => {
    const el = renderZone({ state: "ok", target: IN_REVIEW, over: false });

    expect(screen.getByText("Set status to In Review")).toBeInTheDocument();
    expect(el).toHaveAttribute("data-drop-key", "3");
  });

  it.each([
    ["a zone that can't take the drop", { state: "no", target: null, over: false }],
    ["a zone still waiting for Jira", { state: "wait", target: null, over: false }],
  ] as [string, Zone][])("offers no action and no drop key for %s", (_, zone) => {
    const el = renderZone(zone);

    expect(el).not.toHaveAttribute("data-drop-key");
    expect(el).toBeEmptyDOMElement();
  });

  it.each([
    ["ok", "drop-zone--ok"],
    ["no", "drop-zone--no"],
    ["wait", "drop-zone--wait"],
  ] as const)("styles the %s state as %s", (state, cls) => {
    const el = renderZone({ state, target: state === "ok" ? IN_REVIEW : null, over: false });

    expect(el).toHaveClass("drop-zone", cls);
  });

  it("is solid only while the pointer is over it", () => {
    const rest = renderZone({ state: "ok", target: IN_REVIEW, over: false });
    expect(rest).not.toHaveClass("drop-zone--over");

    const hover = renderZone({ state: "ok", target: IN_REVIEW, over: true });
    expect(hover).toHaveClass("drop-zone--over");
  });

  it("takes the color of the target's mark and the caller's class", () => {
    const el = renderZone({ state: "ok", target: IN_REVIEW, over: false }, { className: "list-group__drop" });

    expect(el).toHaveClass("drop-color--status-review", "list-group__drop");
  });

  it("shows a person's avatar beside the action when the group is a person", () => {
    renderZone({ state: "ok", target: target("demo-account-1", "Assign to Alex Morgan", { kind: "person", person: ALEX }), over: false });

    expect(screen.getByText("Assign to Alex Morgan")).toBeInTheDocument();
    expect(screen.getByText("AM")).toBeInTheDocument();
  });
});

describe("zoneOf", () => {
  it.each([
    ["without a drag", null, "3"],
    ["for the issue's own group", drag(), "1"],
  ])("is null %s", (_, current, key) => {
    expect(zoneOf(current, key)).toBeNull();
  });

  it("waits while Jira has not answered", () => {
    expect(zoneOf(drag({ targets: null }), "3")).toEqual({ state: "wait", target: null, over: false });
  });

  it("takes the drop for a group Jira offers, and is over it when the pointer is", () => {
    expect(zoneOf(drag(), "3")).toEqual({ state: "ok", target: IN_REVIEW, over: false });
    expect(zoneOf(drag({ over: "3" }), "3")).toEqual({ state: "ok", target: IN_REVIEW, over: true });
  });

  it("can't take the drop for a group the workflow doesn't allow, even under the pointer", () => {
    expect(zoneOf(drag({ over: "9" }), "9")).toEqual({ state: "no", target: null, over: false });
  });
});

describe("colorClass", () => {
  it.each([
    [null, "drop-color--plain"],
    [{ kind: "status", style: "done" }, "drop-color--status-done"],
    [{ kind: "person", person: ALEX }, "drop-color--avatar-1"],
    [{ kind: "priority", level: 2 }, "drop-color--priority-2"],
    [{ kind: "priority", level: 0 }, "drop-color--priority-1"],
    [{ kind: "priority", level: 9 }, "drop-color--priority-4"],
    [{ kind: "epic", color: 3 }, "drop-color--plain"],
  ] as [GroupMark | null, string][])("maps %j to %s", (mark, expected) => {
    expect(colorClass(mark)).toBe(expected);
  });
});

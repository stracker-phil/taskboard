import { render, screen } from "@testing-library/react";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import { row } from "../../test/issues";
import { BoardCard } from "./BoardCard";

const actions: ComponentProps<typeof BoardCard>["actions"] = {
  copyKey: vi.fn(),
  openInJira: null,
  markSeen: vi.fn(),
  markUnread: vi.fn(),
  togglePin: vi.fn(),
  toggleMine: null,
  changeStatus: null,
  assignTo: null,
  changePriority: null,
};

const card = (updatedAt: string) => <BoardCard row={row({ updatedAt })} fields={[]} onOpen={vi.fn()} actions={actions} />;

describe("BoardCard", () => {
  it("does not flash when it mounts", () => {
    render(card("2026-10-28T10:00:00.000Z"));

    expect(screen.getByRole("article")).not.toHaveClass("flash");
  });

  it("flashes when its issue is updated while it stays mounted", () => {
    const { rerender } = render(card("2026-10-28T10:00:00.000Z"));

    rerender(card("2026-10-28T11:30:00.000Z"));

    expect(screen.getByRole("article")).toHaveClass("flash");
  });
});

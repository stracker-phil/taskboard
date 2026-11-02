import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import type { Style } from "../api";
import { StatusSquareButton } from "./StatusSquareButton";

type Props = ComponentProps<typeof StatusSquareButton>;

const ISSUE: Props["issue"] = { id: "10042", key: "NW-1042", style: "progress" satisfies Style, statusName: "In Progress" };

function renderSquare(changeStatus: Props["changeStatus"], onPick?: Props["onPick"]) {
  const onClick = vi.fn();
  const onDoubleClick = vi.fn();
  render(
    <div onClick={onClick} onDoubleClick={onDoubleClick}>
      <StatusSquareButton issue={ISSUE} changeStatus={changeStatus} onPick={onPick} />
    </div>,
  );
  return { onClick, onDoubleClick };
}

describe("StatusSquareButton", () => {
  describe("with a status change handler", () => {
    it("is a menu button named after the issue and its status", () => {
      renderSquare(vi.fn());

      expect(screen.getByRole("button", { name: "Change the status of NW-1042 (In Progress)" })).toHaveAttribute("aria-haspopup", "menu");
    });

    it("names the current status in its tooltip", () => {
      renderSquare(vi.fn());

      expect(screen.getByRole("button")).toHaveAttribute("title", "In Progress");
    });

    it("hands the issue and the button itself to the handler and picks the issue when clicked", async () => {
      const changeStatus = vi.fn();
      const onPick = vi.fn();
      renderSquare(changeStatus, onPick);

      const button = screen.getByRole("button");
      await userEvent.click(button);

      expect(changeStatus).toHaveBeenCalledWith(ISSUE, button);
      expect(onPick).toHaveBeenCalledWith("10042");
    });

    it("works without a pick handler", async () => {
      const changeStatus = vi.fn();
      renderSquare(changeStatus);

      await userEvent.click(screen.getByRole("button"));

      expect(changeStatus).toHaveBeenCalled();
    });

    it("keeps a click from reaching the parent", async () => {
      const { onClick } = renderSquare(vi.fn());

      await userEvent.click(screen.getByRole("button"));

      expect(onClick).not.toHaveBeenCalled();
    });

    it("keeps a double-click from reaching the parent", async () => {
      const { onDoubleClick } = renderSquare(vi.fn());

      await userEvent.dblClick(screen.getByRole("button"));

      expect(onDoubleClick).not.toHaveBeenCalled();
    });
  });

  describe("without a status change handler", () => {
    it("renders only the plain square, whose tooltip names the status", () => {
      renderSquare(null);

      expect(screen.queryByRole("button")).not.toBeInTheDocument();
      expect(screen.getByTitle("In Progress")).toHaveClass("status-square", "status-square--progress");
    });
  });
});

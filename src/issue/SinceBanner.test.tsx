import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SinceBanner } from "./SinceBanner";

const SINCE = { label: "Since yesterday", text: "2 new comments, moved to Review." };

describe("SinceBanner", () => {
  it("shows the label and the summary of what changed", () => {
    render(<SinceBanner since={SINCE} />);

    expect(screen.getByRole("status")).toHaveTextContent("Since yesterday: 2 new comments, moved to Review.");
  });

  it("offers no Mark as read without a handler", () => {
    render(<SinceBanner since={SINCE} />);

    expect(screen.queryByRole("button", { name: "Mark as read" })).not.toBeInTheDocument();
  });

  it("calls the handler when Mark as read is clicked", async () => {
    const onMarkSeen = vi.fn();
    render(<SinceBanner since={SINCE} onMarkSeen={onMarkSeen} />);

    await userEvent.click(screen.getByRole("button", { name: "Mark as read" }));

    expect(onMarkSeen).toHaveBeenCalled();
  });
});

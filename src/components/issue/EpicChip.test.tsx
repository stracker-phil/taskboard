import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EpicChip } from "./EpicChip";

describe("EpicChip", () => {
  it("shows the epic name and keeps the full name as its tooltip", () => {
    const name = "Security Vulnerabilities (reported during August 2026)";

    render(<EpicChip epic={{ id: "9", name, color: 2 }} />);

    expect(screen.getByText(name)).toBeInTheDocument();
    expect(screen.getByTitle(name)).toHaveTextContent(name);
  });
});

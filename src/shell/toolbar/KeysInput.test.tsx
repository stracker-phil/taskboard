import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Condition } from "../../api";
import { KeysInput } from "./KeysInput";

const mocked = vi.mocked(api, true);

const conditionOf = (...keys: string[]): Condition => ({ field: "issueKey", op: keys.length > 1 ? "in" : "is", value: keys.length > 1 ? keys : keys[0] });

function Harness({ initial }: { initial: Condition }) {
  const [condition, setCondition] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  return (
    <>
      <label htmlFor="keys">Keys</label>
      <KeysInput id="keys" condition={condition} onChange={setCondition} onError={setError} />
      {error && <p>{error}</p>}
    </>
  );
}

const input = () => screen.getByLabelText("Keys");

beforeEach(() => {
  vi.clearAllMocks();
});

describe("KeysInput", () => {
  it("shows the stored keys joined by commas", () => {
    render(<KeysInput id="keys" condition={conditionOf("NW-1", "NW-2")} onChange={vi.fn()} onError={vi.fn()} />);

    expect(screen.getByPlaceholderText("NW-123, NW-124")).toHaveValue("NW-1, NW-2");
  });

  it("shows the new keys when the stored value changes from outside", () => {
    const props = { id: "keys", onChange: vi.fn(), onError: vi.fn() };
    const { rerender } = render(<KeysInput {...props} condition={conditionOf("NW-1")} />);

    rerender(<KeysInput {...props} condition={conditionOf("NW-2", "NW-3")} />);

    expect(screen.getByPlaceholderText("NW-123, NW-124")).toHaveValue("NW-2, NW-3");
  });

  it("replaces typed but unapplied text when the stored value changes from outside", async () => {
    const props = { id: "keys", onChange: vi.fn(), onError: vi.fn() };
    const { rerender } = render(<KeysInput {...props} condition={conditionOf("NW-1")} />);
    await userEvent.type(screen.getByPlaceholderText("NW-123, NW-124"), ", nw-9");

    rerender(<KeysInput {...props} condition={conditionOf("NW-4")} />);

    expect(screen.getByPlaceholderText("NW-123, NW-124")).toHaveValue("NW-4");
  });

  it("shows the keys as Rust normalized them once applied", async () => {
    mocked.filterParseKeys.mockResolvedValue(["NW-5", "NW-6"]);
    render(<Harness initial={conditionOf("NW-1")} />);

    await userEvent.clear(input());
    await userEvent.type(input(), "nw-5 nw-6{Enter}");

    await vi.waitFor(() => expect(input()).toHaveValue("NW-5, NW-6"));
  });

  it("shows the refusal and keeps the typed text", async () => {
    mocked.filterParseKeys.mockRejectedValue({ kind: "invalid_input", message: "Not an issue key: abc" });
    render(<Harness initial={conditionOf("NW-1")} />);

    await userEvent.clear(input());
    await userEvent.type(input(), "abc{Enter}");

    expect(await screen.findByText("Not an issue key: abc")).toBeInTheDocument();
    expect(input()).toHaveValue("abc");
  });

  it("asks Rust nothing when the text still equals the stored keys on blur", async () => {
    render(<Harness initial={conditionOf("NW-1")} />);

    await userEvent.click(input());
    await userEvent.tab();

    expect(mocked.filterParseKeys).not.toHaveBeenCalled();
    expect(input()).toHaveValue("NW-1");
  });
});

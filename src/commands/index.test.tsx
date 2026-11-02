import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { installCommands, useCommandKeys } from ".";
import { onIssueKeys, type IssueKeyHandlers } from "./issueKeys";

const mocked = vi.mocked(api, true);

function Keys() {
  useCommandKeys();
  return null;
}

let release: (() => void) | null = null;

function takeIssueKeys() {
  const handlers = { move: vi.fn(), togglePeek: vi.fn(), open: vi.fn(), hasSelection: () => true, peekOpen: () => false };
  release = onIssueKeys(handlers as unknown as IssueKeyHandlers);
  return handlers;
}

async function renderKeys() {
  render(
    <>
      <Keys />
      <input type="checkbox" aria-label="Pick NW-1001" />
    </>,
  );
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("keybindings_changed", expect.any(Function)));
}

beforeEach(() => {
  vi.clearAllMocks();
  installCommands("main");
});

afterEach(() => {
  release?.();
  release = null;
});

describe("keys on a focused control", () => {
  it.each([
    ["Space", " "],
    ["Enter", "{Enter}"],
  ])("%s on a focused checkbox runs no command", async (_label, key) => {
    const handlers = takeIssueKeys();
    await renderKeys();
    screen.getByRole("checkbox", { name: "Pick NW-1001" }).focus();

    await userEvent.keyboard(key);

    expect(handlers.togglePeek).not.toHaveBeenCalled();
    expect(handlers.open).not.toHaveBeenCalled();
  });

  it("Space ticks the focused checkbox", async () => {
    takeIssueKeys();
    await renderKeys();
    const box = screen.getByRole("checkbox", { name: "Pick NW-1001" });
    box.focus();

    await userEvent.keyboard(" ");

    expect(box).toBeChecked();
  });

  it("Space with no control focused still toggles the preview", async () => {
    const handlers = takeIssueKeys();
    await renderKeys();

    await userEvent.keyboard(" ");

    expect(handlers.togglePeek).toHaveBeenCalled();
  });
});

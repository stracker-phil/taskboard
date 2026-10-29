import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
import { issueDetail } from "../../test/issues";
import { HoverPeek } from "./HoverPeek";

const mocked = vi.mocked(api, true);

const advance = (ms: number) => act(() => vi.advanceTimersByTimeAsync(ms));
const peek = (key: string) => screen.findByRole("tooltip", { name: `Peek at ${key}` });

function renderPeek() {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      <HoverPeek />
      <div data-issue-id="1">NW-1042</div>
      <div data-issue-id="2">
        <span>NW-2000 inner</span>
      </div>
      <p>Plain text</p>
    </QueryClientProvider>,
  );
}

const first = () => screen.getByText("NW-1042");

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  vi.useFakeTimers({ shouldAdvanceTime: true });
  mocked.issueGet.mockImplementation(async (id: string) =>
    id === "2" ? issueDetail({ id: "2", key: "NW-2000", summary: "Second issue", descriptionHtml: null }) : issueDetail({ id: "1" }),
  );
});

afterEach(() => {
  vi.useRealTimers();
});

describe("HoverPeek", () => {
  it("shows the summary, status and description of the hovered issue after 150 ms with ⌥ held", async () => {
    renderPeek();
    fireEvent.pointerOver(first(), { altKey: true });
    await advance(150);

    const card = await peek("NW-1042");
    expect(within(card).getByText("Checkout: address validation for DACH postcodes")).toBeInTheDocument();
    expect(within(card).getByText("In Progress")).toBeInTheDocument();
    expect(within(card).getByText("Validate German, Austrian and Swiss postcodes.")).toBeInTheDocument();
  });

  it("shows a placeholder when the issue has no description", async () => {
    renderPeek();
    fireEvent.pointerOver(screen.getByText("NW-2000 inner"), { altKey: true });
    await advance(150);

    const card = await peek("NW-2000");
    expect(within(card).getByText("No description.")).toBeInTheDocument();
  });

  it("shows nothing before the 150 ms rest has passed", async () => {
    renderPeek();
    fireEvent.pointerOver(first(), { altKey: true });
    await advance(149);

    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("shows nothing when ⌥ is not held", async () => {
    renderPeek();
    fireEvent.pointerOver(first());
    await advance(500);

    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  describe("with the pointer resting on an issue", () => {
    it("shows the peek after ⌥ is pressed and the delay has passed", async () => {
      renderPeek();
      fireEvent.pointerOver(first());
      fireEvent.keyDown(window, { key: "Alt" });
      await advance(150);

      expect(await peek("NW-1042")).toBeInTheDocument();
    });

    it("hides the peek when ⌥ is released", async () => {
      renderPeek();
      fireEvent.pointerOver(first(), { altKey: true });
      await advance(150);
      await peek("NW-1042");

      fireEvent.keyUp(window, { key: "Alt" });

      expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    });

    it("hides the peek when the window loses focus", async () => {
      renderPeek();
      fireEvent.pointerOver(first(), { altKey: true });
      await advance(150);
      await peek("NW-1042");

      fireEvent.blur(window);

      expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
    });
  });

  it("hides the peek when the pointer moves onto an element without an issue", async () => {
    renderPeek();
    fireEvent.pointerOver(first(), { altKey: true });
    await advance(150);
    await peek("NW-1042");

    fireEvent.pointerOver(screen.getByText("Plain text"), { altKey: true });

    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("treats a nested child of the marked element as that issue", async () => {
    renderPeek();
    fireEvent.pointerMove(screen.getByText("NW-2000 inner"), { altKey: true });
    await advance(150);

    expect(await peek("NW-2000")).toBeInTheDocument();
  });

  it("switches to another issue when the pointer moves onto its element", async () => {
    renderPeek();
    fireEvent.pointerOver(first(), { altKey: true });
    await advance(150);
    await peek("NW-1042");

    fireEvent.pointerOver(screen.getByText("NW-2000 inner"), { altKey: true });
    await advance(150);

    expect(await peek("NW-2000")).toBeInTheDocument();
    expect(screen.queryByRole("tooltip", { name: "Peek at NW-1042" })).not.toBeInTheDocument();
  });
});

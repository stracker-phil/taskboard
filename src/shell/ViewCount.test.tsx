import { act, render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { narrowingKey } from "../state/slices/narrowing";
import { resetStore, useStore } from "../state/store";
import { boardData } from "../test/board";
import { viewData } from "../test/issues";
import { view } from "../test/views";
import { ViewCount } from "./ViewCount";

const mocked = vi.mocked(api, true);

const LIST = view({ id: 101, name: "List", layout: "list" });
const BOARD = view({ id: 102, name: "Sprint board", layout: "board", position: 1 });

function renderCount(viewId: number) {
  mocked.viewsOf.mockResolvedValue([LIST, BOARD]);
  render(
    <QueryClientProvider client={createQueryClient()}>
      <ViewCount listId={11} viewId={viewId} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("ViewCount", () => {
  it("shows the total of the view's rows", async () => {
    mocked.viewRows.mockResolvedValue(viewData([], [], { total: 42 }));

    renderCount(101);

    expect(await screen.findByText("42 issues")).toBeInTheDocument();
  });

  it("separates the thousands of a large total", async () => {
    mocked.viewRows.mockResolvedValue(viewData([], [], { total: 1234 }));

    renderCount(101);

    expect(await screen.findByText(`${new Intl.NumberFormat().format(1234)} issues`)).toBeInTheDocument();
  });

  it("says 'issue' for a single one", async () => {
    mocked.viewRows.mockResolvedValue(viewData([], [], { total: 1 }));

    renderCount(101);

    expect(await screen.findByText("1 issue")).toBeInTheDocument();
  });

  it("says '0 issues' for an empty view", async () => {
    mocked.viewRows.mockResolvedValue(viewData([], [], { total: 0 }));

    renderCount(101);

    expect(await screen.findByText("0 issues")).toBeInTheDocument();
  });

  it("shows the total of the board for a board view", async () => {
    mocked.viewRows.mockResolvedValue(viewData([], [], { total: 5 }));
    mocked.viewBoard.mockResolvedValue(boardData([], [], { total: 7 }));

    renderCount(102);

    expect(await screen.findByText("7 issues")).toBeInTheDocument();
    expect(mocked.viewRows).not.toHaveBeenCalled();
  });

  it("shows nothing until the first answer arrives", async () => {
    mocked.viewRows.mockReturnValue(new Promise(() => {}));

    renderCount(101);

    await vi.waitFor(() => expect(mocked.viewRows).toHaveBeenCalled());
    expect(screen.queryByText(/issue/)).not.toBeInTheDocument();
  });

  it("shows the new total when the text narrowing changes", async () => {
    mocked.viewRows.mockImplementation(async (_list, _view, narrowing) => viewData([], [], { total: narrowing.text === "pricing" ? 3 : 42 }));
    renderCount(101);
    await screen.findByText("42 issues");

    act(() => useStore.getState().setNarrowing(narrowingKey(11, 101), { text: "pricing" }));

    expect(await screen.findByText("3 issues")).toBeInTheDocument();
  });

  it("shows the new total when the Me chip is turned on", async () => {
    mocked.viewRows.mockImplementation(async (_list, _view, narrowing) => viewData([], [], { total: narrowing.me ? 4 : 42 }));
    renderCount(101);
    await screen.findByText("42 issues");

    act(() => useStore.getState().setNarrowing(narrowingKey(11, 101), { me: true }));

    expect(await screen.findByText("4 issues")).toBeInTheDocument();
  });

  it("keeps the previous number while the next total loads", async () => {
    let finish: (data: ReturnType<typeof viewData>) => void = () => {};
    mocked.viewRows.mockImplementation((_list, _view, narrowing) =>
      narrowing.text === "pricing" ? new Promise((resolve) => (finish = resolve)) : Promise.resolve(viewData([], [], { total: 42 })),
    );
    renderCount(101);
    await screen.findByText("42 issues");

    act(() => useStore.getState().setNarrowing(narrowingKey(11, 101), { text: "pricing" }));
    await vi.waitFor(() => expect(mocked.viewRows).toHaveBeenCalledWith(11, 101, expect.objectContaining({ text: "pricing" })));

    expect(screen.getByText("42 issues")).toBeInTheDocument();
    await act(async () => finish(viewData([], [], { total: 3 })));
    expect(await screen.findByText("3 issues")).toBeInTheDocument();
  });
});

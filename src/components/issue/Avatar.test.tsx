import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({
  ...(await orig<object>()),
  api: (await import("../../test/apiMock")).createApiMock(),
}));

import { api } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { ALEX, SAM } from "../../test/issues";
import { Avatar, AvatarStack } from "./Avatar";

const mocked = vi.mocked(api, true);

const PICTURE = "data:image/png;base64,iVBORw0KGgo=";

function renderWithClient(ui: React.ReactElement) {
  return render(<QueryClientProvider client={createQueryClient()}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.clearAllMocks();
  mocked.userAvatar.mockResolvedValue(null);
});

describe("Avatar", () => {
  it("shows the initials and is named after the person", () => {
    renderWithClient(<Avatar person={ALEX} />);

    expect(screen.getByRole("img", { name: "Alex Morgan" })).toHaveTextContent("AM");
  });

  it("replaces the initials with the profile picture once it is available", async () => {
    mocked.userAvatar.mockResolvedValue(PICTURE);
    renderWithClient(<Avatar person={ALEX} />);

    const avatar = screen.getByRole("img", { name: "Alex Morgan" });
    await waitFor(() => expect(avatar.querySelector("img")).toHaveAttribute("src", PICTURE));
    expect(avatar).not.toHaveTextContent("AM");
  });

  it("keeps the initials when the person has no profile picture", async () => {
    renderWithClient(<Avatar person={ALEX} />);

    const avatar = screen.getByRole("img", { name: "Alex Morgan" });
    await waitFor(() => expect(mocked.userAvatar).toHaveBeenCalled());
    await Promise.resolve();

    expect(avatar).toHaveTextContent("AM");
    expect(avatar.querySelector("img")).toBeNull();
  });
});

describe("AvatarStack", () => {
  it("shows one avatar per person in order", () => {
    renderWithClient(<AvatarStack people={[ALEX, SAM]} />);

    expect(screen.getAllByRole("img").map((a) => a.getAttribute("aria-label"))).toEqual(["Alex Morgan", "Sam Lee"]);
  });

  it("shows nothing for nobody", () => {
    renderWithClient(<AvatarStack people={[]} />);

    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });

  it("shows the picture only on the avatar of the person who has one", async () => {
    mocked.userAvatar.mockImplementation(async (accountId) => (accountId === ALEX.accountId ? PICTURE : null));
    renderWithClient(<AvatarStack people={[ALEX, SAM]} />);

    const alex = screen.getByRole("img", { name: "Alex Morgan" });
    const sam = screen.getByRole("img", { name: "Sam Lee" });
    await waitFor(() => expect(alex.querySelector("img")).toHaveAttribute("src", PICTURE));
    expect(sam).toHaveTextContent("SL");
    expect(sam.querySelector("img")).toBeNull();
  });
});

import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Appearance } from "../../api";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
import { settings } from "../../test/settings";
import { AppearancePage } from "./AppearancePage";

const mocked = vi.mocked(api, true);

async function renderPage(appearance: Appearance = "system") {
  mocked.getSettings.mockResolvedValue(settings({ appearance }));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <AppearancePage />
    </QueryClientProvider>,
  );
  const group = screen.getByRole("group", { name: "Appearance" });
  await within(group).findByRole("button", { pressed: true });
  return group;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("AppearancePage", () => {
  it("offers Automatic, Light and Dark", async () => {
    const group = await renderPage();

    expect(within(group).getAllByRole("button").map((b) => b.textContent)).toEqual(["Automatic", "Light", "Dark"]);
  });

  it.each<[Appearance, string]>([
    ["system", "Automatic"],
    ["light", "Light"],
    ["dark", "Dark"],
  ])("presses only %s as the current appearance", async (appearance, label) => {
    const group = await renderPage(appearance);

    expect(within(group).getByRole("button", { pressed: true })).toHaveTextContent(label);
    expect(within(group).getAllByRole("button", { pressed: true })).toHaveLength(1);
  });

  it.each<[string, Appearance]>([
    ["Automatic", "system"],
    ["Light", "light"],
    ["Dark", "dark"],
  ])("sends %s to Rust as %s when chosen", async (label, value) => {
    const group = await renderPage(value === "light" ? "dark" : "light");

    await userEvent.click(within(group).getByRole("button", { name: label }));

    expect(mocked.appearanceSet).toHaveBeenCalledWith(value);
  });
});

import { QueryClientProvider } from "@tanstack/react-query";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Person } from "../../api";
import { MenuHost } from "../../components/menu";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
import { ALEX, SAM } from "../../test/issues";
import { view } from "../../test/views";
import { PersonChip } from "./PersonChip";

const mocked = vi.mocked(api, true);

function renderChip(person: string | null = null, onChange = vi.fn()) {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <PersonChip listId={11} viewId={101} person={person} onChange={onChange} />
      <MenuHost />
    </QueryClientProvider>,
  );
  return onChange;
}

const chip = () => screen.getByRole("button", { name: /^(Person|Alex|Sam|Cher)/ });

async function openMenu() {
  await vi.waitFor(() => expect(mocked.viewPeople).toHaveBeenCalled());
  await act(async () => {
    await mocked.viewPeople.mock.results[0]?.value;
  });
  await userEvent.click(chip());
  return within(await screen.findByRole("menu", { name: "Person" }));
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.viewsOf.mockResolvedValue([view()]);
  mocked.viewPeople.mockResolvedValue([ALEX, SAM]);
});

describe("PersonChip", () => {
  describe("the chip", () => {
    it("reads Person and is unpressed while nobody is picked", () => {
      renderChip();

      expect(chip()).toHaveTextContent("Person");
      expect(chip()).toHaveAttribute("aria-pressed", "false");
      expect(chip()).toHaveAttribute("aria-haspopup", "menu");
      expect(chip()).toHaveAttribute("title", "Show one person's issues");
    });

    it("shows the picked person's first name, pressed, once the view's people are known", async () => {
      renderChip("demo-account-2");

      const pressed = await screen.findByRole("button", { name: /^Sam/ });

      expect(pressed).toHaveAttribute("aria-pressed", "true");
      expect(pressed).toHaveAttribute("title", "Only issues assigned to Sam Lee");
      expect(pressed).not.toHaveTextContent("Lee");
    });

    it("stays unpressed for an account that is not among the view's people", async () => {
      renderChip("demo-account-9");
      await vi.waitFor(() => expect(mocked.viewPeople).toHaveBeenCalled());

      expect(chip()).toHaveTextContent("Person");
      expect(chip()).toHaveAttribute("aria-pressed", "false");
    });

    it("shows a single-word name whole", async () => {
      const cher: Person = { accountId: "demo-account-5", name: "Cher", initials: "C", color: 4 };
      mocked.viewPeople.mockResolvedValue([cher]);
      renderChip("demo-account-5");

      expect(await screen.findByRole("button", { name: /^Cher/ })).toHaveAttribute("aria-pressed", "true");
    });

    it("asks for the people of its own view only", async () => {
      renderChip();

      await vi.waitFor(() => expect(mocked.viewPeople).toHaveBeenCalledWith(11, 101, expect.anything()));
    });
  });

  describe("the menu", () => {
    it("lists Anyone, then each person, under a caption", async () => {
      renderChip();

      const menu = await openMenu();

      expect(screen.getByText("Issues assigned to")).toBeInTheDocument();
      expect(menu.getAllByRole("menuitemradio").map((row) => row.textContent)).toEqual([expect.stringContaining("Anyone"), expect.stringContaining("Alex Morgan"), expect.stringContaining("Sam Lee")]);
    });

    it("checks Anyone while nobody is picked", async () => {
      renderChip();

      const menu = await openMenu();

      expect(menu.getByRole("menuitemradio", { name: "Anyone" })).toHaveAttribute("aria-checked", "true");
      expect(menu.getByRole("menuitemradio", { name: /Sam Lee/ })).toHaveAttribute("aria-checked", "false");
    });

    it("checks the picked person and not Anyone", async () => {
      renderChip("demo-account-1");
      await screen.findByRole("button", { name: /^Alex/ });

      const menu = await openMenu();

      expect(menu.getByRole("menuitemradio", { name: /Alex Morgan/ })).toHaveAttribute("aria-checked", "true");
      expect(menu.getByRole("menuitemradio", { name: "Anyone" })).toHaveAttribute("aria-checked", "false");
    });

    it("reports the account id of the chosen person and closes the menu", async () => {
      const onChange = renderChip();
      const menu = await openMenu();

      await userEvent.click(menu.getByRole("menuitemradio", { name: /Sam Lee/ }));

      expect(onChange).toHaveBeenCalledWith("demo-account-2");
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("reports null when Anyone is chosen", async () => {
      const onChange = renderChip("demo-account-2");
      await screen.findByRole("button", { name: /^Sam/ });
      const menu = await openMenu();

      await userEvent.click(menu.getByRole("menuitemradio", { name: "Anyone" }));

      expect(onChange).toHaveBeenCalledWith(null);
    });

    it("says there are no teammates yet, in a disabled row, when the view has none", async () => {
      mocked.viewPeople.mockResolvedValue([]);
      renderChip();

      const menu = await openMenu();

      expect(menu.getByRole("menuitem", { name: "No teammates yet" })).toHaveAttribute("aria-disabled", "true");
      expect(menu.getByRole("menuitemradio", { name: "Anyone" })).toBeInTheDocument();
    });
  });
});

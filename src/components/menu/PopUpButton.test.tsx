import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resetStore } from "../../state/store";
import { MenuHost } from ".";
import { PopUpButton } from "./PopUpButton";

const OPTIONS = [
  { value: "open", label: "Open" },
  { value: "done", label: "Done" },
  { value: "later", label: "Later" },
] as const;

type Choice = (typeof OPTIONS)[number]["value"];

interface Extra {
  disabled?: boolean;
  search?: string;
}

function renderButton(onChange = vi.fn(), value: Choice = "open", extra: Extra = {}) {
  render(
    <>
      <label htmlFor="status">Status</label>
      <PopUpButton id="status" value={value} options={OPTIONS} onChange={onChange} {...extra} />
      <MenuHost />
    </>,
  );
  return onChange;
}

/** The open menu carries the same label, so pick the button among the matches. */
const button = () => screen.getAllByLabelText("Status").find((el) => el.tagName === "BUTTON") as HTMLElement;

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("PopUpButton", () => {
  describe("the button", () => {
    it("shows the label of the current choice and no menu", () => {
      renderButton(vi.fn(), "done");

      expect(button()).toHaveTextContent(/^Done$/);
      expect(button()).toHaveAttribute("aria-haspopup", "menu");
      expect(button()).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("shows nothing for a value that is not among the options", () => {
      renderButton(vi.fn(), "gone" as Choice);

      expect(button()).toHaveTextContent(/^$/);
    });
  });

  describe("opening", () => {
    it("opens a menu named by its label with a row per option and the current one checked", async () => {
      renderButton(vi.fn(), "done");

      await userEvent.click(button());

      const menu = screen.getByRole("menu", { name: "Status" });
      expect(menu).toBeInTheDocument();
      expect(screen.getAllByRole("menuitemradio").map((row) => row.textContent)).toEqual(["Open", "Done", "Later"]);
      expect(screen.getByRole("menuitemradio", { name: "Done" })).toHaveAttribute("aria-checked", "true");
      expect(screen.getByRole("menuitemradio", { name: "Open" })).toHaveAttribute("aria-checked", "false");
    });

    it("reports itself expanded while its menu is open", async () => {
      renderButton();

      await userEvent.click(button());

      expect(screen.getByRole("button", { expanded: true })).toBeInTheDocument();
    });

    it.each(["{ArrowDown}", "{ArrowUp}"])("opens on %s while the button has focus", async (key) => {
      renderButton();
      button().focus();

      await userEvent.keyboard(key);

      expect(screen.getByRole("menu", { name: "Status" })).toBeInTheDocument();
    });

    it("closes the menu on a second click on the button", async () => {
      renderButton();
      await userEvent.click(button());

      await userEvent.click(button());

      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
      expect(button()).toHaveAttribute("aria-expanded", "false");
    });

    it("does not open when disabled", async () => {
      renderButton(vi.fn(), "open", { disabled: true });

      await userEvent.click(button());

      expect(button()).toBeDisabled();
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });
  });

  describe("choosing", () => {
    it("passes a different option's value on and closes the menu", async () => {
      const onChange = renderButton();
      await userEvent.click(button());

      await userEvent.click(screen.getByRole("menuitemradio", { name: "Later" }));

      expect(onChange).toHaveBeenCalledWith("later");
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("reports nothing for the current option but closes the menu", async () => {
      const onChange = renderButton();
      await userEvent.click(button());

      await userEvent.click(screen.getByRole("menuitemradio", { name: "Open" }));

      expect(onChange).not.toHaveBeenCalled();
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("chooses the focused row with the keyboard", async () => {
      const onChange = renderButton();
      button().focus();
      await userEvent.keyboard("{ArrowDown}");

      await userEvent.keyboard("{ArrowDown}{Enter}");

      expect(onChange).toHaveBeenCalledWith("done");
    });
  });

  describe("search", () => {
    it("has no search field by default", async () => {
      renderButton();

      await userEvent.click(button());

      expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
    });

    it("gives the menu a search field with the placeholder that narrows the options", async () => {
      const onChange = renderButton(vi.fn(), "open", { search: "Find a status" });
      await userEvent.click(button());

      await userEvent.type(screen.getByPlaceholderText("Find a status"), "lat");

      expect(screen.getAllByRole("menuitemradio").map((row) => row.textContent)).toEqual(["Later"]);
      await userEvent.keyboard("{Enter}");
      expect(onChange).toHaveBeenCalledWith("later");
    });
  });
});

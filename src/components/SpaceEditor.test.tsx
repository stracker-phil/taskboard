import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api, type SpaceNode } from "../api";
import { createQueryClient } from "../queries/queryClient";
import { resetStore } from "../state/store";
import { sidebarTree, space } from "../test/spaces";
import { Toasts } from "./Toasts";
import { SpaceFields, SpacePopover } from "./SpaceEditor";

const mocked = vi.mocked(api, true);

function renderWithClient(ui: React.ReactNode) {
  return render(
    <QueryClientProvider client={createQueryClient()}>
      {ui}
      <Toasts />
    </QueryClientProvider>,
  );
}

async function renderFields(s: SpaceNode, clientOnly = false) {
  renderWithClient(<SpaceFields space={s} clientOnly={clientOnly} />);
  await screen.findAllByRole(clientOnly ? "textbox" : "radiogroup");
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.spaceUpdate.mockResolvedValue(undefined);
  mocked.sidebarTree.mockResolvedValue(sidebarTree([space()]));
});

describe("SpaceFields", () => {
  it("shows Name, Icon and color and Client by default", async () => {
    await renderFields(space());

    expect(screen.getByLabelText("Name")).toBeInTheDocument();
    expect(screen.getByRole("radiogroup", { name: "Icon" })).toBeInTheDocument();
    expect(screen.getByRole("radiogroup", { name: "Color" })).toBeInTheDocument();
    expect(screen.getByLabelText("Client")).toBeInTheDocument();
  });

  describe("Name", () => {
    it("is blank, with the project's name as placeholder, while the Space uses the project's name", async () => {
      await renderFields(space({ name: "Website relaunch" }));

      const name = await screen.findByLabelText("Name");
      expect(name).toHaveValue("");
      expect(name).toHaveAttribute("placeholder", "Website relaunch");
    });

    it("holds the custom name when it differs from the project's", async () => {
      await renderFields(space({ name: "Public site", projectName: "Website relaunch" }));

      expect(screen.getByLabelText("Name")).toHaveValue("Public site");
      expect(screen.getByLabelText("Name")).toHaveAttribute("placeholder", "Website relaunch");
    });

    it("sends the custom name and leaves the other fields as they are", async () => {
      await renderFields(space({ icon: "W", color: "3", client: "Northwind" }));

      await userEvent.type(await screen.findByLabelText("Name"), "Public site");
      await userEvent.tab();

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", { name: "Public site", icon: "W", color: "3", client: "Northwind" });
    });
  });

  describe("Client", () => {
    it("sends the changed client on blur with the other fields unchanged", async () => {
      await renderFields(space({ icon: "W", color: "2", client: null }));

      await userEvent.type(await screen.findByLabelText("Client"), "Northwind");
      await userEvent.tab();

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", { name: null, icon: "W", color: "2", client: "Northwind" });
    });

    it("sends the client on Enter", async () => {
      await renderFields(space());

      await userEvent.type(await screen.findByLabelText("Client"), "Northwind{Enter}");

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", expect.objectContaining({ client: "Northwind" }));
    });

    it("sends null when the field is cleared", async () => {
      await renderFields(space({ client: "Northwind" }));

      await userEvent.clear(await screen.findByLabelText("Client"));
      await userEvent.tab();

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", { name: null, icon: null, color: "1", client: null });
    });

    it("sends nothing when the value is unchanged", async () => {
      await renderFields(space({ client: "Northwind" }));

      await userEvent.click(await screen.findByLabelText("Client"));
      await userEvent.tab();

      expect(mocked.spaceUpdate).not.toHaveBeenCalled();
    });
  });

  describe("Icon", () => {
    it("sends the picked line icon, keeping the color and client", async () => {
      await renderFields(space({ color: "3", client: "Northwind" }));

      await userEvent.click(screen.getByRole("radio", { name: "rocket" }));

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", { name: null, icon: "rocket", color: "3", client: "Northwind" });
    });

    it("sends the typed letter or emoji, keeping only its first character", async () => {
      await renderFields(space({ color: "3" }));

      await userEvent.click(screen.getByRole("button", { name: "Letter or emoji" }));
      await userEvent.type(screen.getByRole("textbox", { name: "Letter or emoji" }), "wx{Enter}");

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", { name: null, icon: "w", color: "3", client: null });
    });

    it("sends null on Reset", async () => {
      await renderFields(space({ icon: "W" }));

      await userEvent.click(screen.getByRole("button", { name: "Reset" }));

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", expect.objectContaining({ icon: null }));
    });

    it("shows the message of Rust's field error and no toast", async () => {
      mocked.spaceUpdate.mockRejectedValue({
        kind: "invalid_input",
        message: "Invalid input",
        field: { path: "icon", message: "Use one letter or digit" },
      });
      await renderFields(space());

      await userEvent.click(screen.getByRole("radio", { name: "rocket" }));

      expect(await screen.findByText("Use one letter or digit")).toBeInTheDocument();
      expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    });
  });

  describe("Color", () => {
    it("sends the color number of the clicked swatch, keeping the icon and client", async () => {
      await renderFields(space({ icon: "W", client: "Northwind" }));

      await userEvent.click(screen.getByRole("radio", { name: "Teal" }));

      expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", { name: null, icon: "W", color: "4", client: "Northwind" });
    });

    it("checks only the current color's swatch", async () => {
      await renderFields(space({ color: "5" }));

      expect(screen.getByRole("radio", { name: "Amber", checked: true })).toBeInTheDocument();
      for (const other of ["Blue", "Slate", "Violet", "Teal"]) {
        expect(screen.getByRole("radio", { name: other, checked: false })).toBeInTheDocument();
      }
    });

    it("offers no No color option", async () => {
      await renderFields(space());

      expect(screen.queryByRole("radio", { name: "No color" })).not.toBeInTheDocument();
    });
  });

  it("shows an error toast naming the Space when a save fails for another reason", async () => {
    mocked.spaceUpdate.mockRejectedValue({ kind: "db", message: "Database is locked" });
    await renderFields(space());

    await userEvent.click(screen.getByRole("radio", { name: "Teal" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not change Website relaunch: Database is locked");
  });

  it("shows only Client when clientOnly is set", async () => {
    await renderFields(space(), true);

    expect(screen.getByLabelText("Client")).toBeInTheDocument();
    expect(screen.queryByLabelText("Name")).not.toBeInTheDocument();
    expect(screen.queryByRole("radiogroup", { name: "Icon" })).not.toBeInTheDocument();
    expect(screen.queryByRole("radiogroup", { name: "Color" })).not.toBeInTheDocument();
  });
});

describe("SpacePopover", () => {
  const onClose = vi.fn();

  function renderPopover() {
    renderWithClient(
      <>
        <button type="button">Outside</button>
        <SpacePopover space={space()} onClose={onClose} />
      </>,
    );
  }

  beforeEach(() => {
    onClose.mockClear();
  });

  it("is the Client label dialog and shows only the Client field", () => {
    renderPopover();

    const dialog = screen.getByRole("dialog", { name: "Client label" });
    expect(within(dialog).getByLabelText("Client")).toBeInTheDocument();
    expect(screen.queryByLabelText("Name")).not.toBeInTheDocument();
    expect(screen.queryByRole("radiogroup", { name: "Color" })).not.toBeInTheDocument();
  });

  it("closes on Escape", async () => {
    renderPopover();

    await userEvent.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalled();
  });

  it("closes on Done", async () => {
    renderPopover();

    await userEvent.click(screen.getByRole("button", { name: "Done" }));

    expect(onClose).toHaveBeenCalled();
  });

  it("closes on a mousedown outside", async () => {
    renderPopover();

    await userEvent.click(screen.getByRole("button", { name: "Outside" }));

    expect(onClose).toHaveBeenCalled();
  });

  it("stays open on a mousedown inside", async () => {
    renderPopover();

    await userEvent.click(screen.getByRole("heading", { name: "Client label" }));
    await userEvent.click(screen.getByLabelText("Client"));

    expect(onClose).not.toHaveBeenCalled();
  });

  it("saves the text being typed before closing on Escape", async () => {
    renderPopover();

    await userEvent.type(screen.getByLabelText("Client"), "Northwind");
    await userEvent.keyboard("{Escape}");

    expect(mocked.spaceUpdate).toHaveBeenCalledWith("10001", expect.objectContaining({ client: "Northwind" }));
    expect(onClose).toHaveBeenCalled();
  });
});

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api } from "../../api";
import { resetStore } from "../../state/store";
import { SidebarFooter } from "./SidebarFooter";

const mocked = vi.mocked(api, true);

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("SidebarFooter", () => {
  it("runs the app:settings command from the Settings button", async () => {
    render(<SidebarFooter />);

    await userEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(mocked.runCommand).toHaveBeenCalledWith("app:settings");
  });

  it("no longer shows the sync state", () => {
    render(<SidebarFooter />);

    expect(screen.getAllByRole("button")).toHaveLength(1);
  });
});

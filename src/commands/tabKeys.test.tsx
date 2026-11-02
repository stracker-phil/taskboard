import { render, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { api } from "../api";
import { installCommands, useCommandKeys } from ".";
import { allCommands } from "./registry";
import { onSelectTab } from "./tabKeys";

const mocked = vi.mocked(api, true);

function Keys() {
  useCommandKeys();
  return null;
}

async function renderKeys() {
  render(<Keys />);
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("keybindings_changed", expect.any(Function)));
}

beforeEach(() => {
  vi.clearAllMocks();
  installCommands("main");
});

describe("tab keys", () => {
  it("registers tab.select.1 to tab.select.9 on ⌘⇧1 to ⌘⇧9", () => {
    const select = allCommands().filter((c) => c.id.startsWith("tab.select."));

    expect(select.map((c) => [c.id, c.defaultBinding])).toEqual(
      Array.from({ length: 9 }, (_, i) => [`tab.select.${i + 1}`, `Mod+Shift+${i + 1}`]),
    );
  });

  it.each([1, 2, 9])("passes the zero-based index of ⌘⇧%i to the shown tab strip", async (n) => {
    const handler = vi.fn();
    const release = onSelectTab(handler);
    await renderKeys();

    await userEvent.keyboard(`{Meta>}{Shift>}[Digit${n}]{/Shift}{/Meta}`);

    expect(handler).toHaveBeenCalledWith(n - 1);
    release();
  });

  it("ignores ⌘⇧1 while no tab strip is shown", async () => {
    const handler = vi.fn();
    onSelectTab(handler)();
    await renderKeys();

    await userEvent.keyboard("{Meta>}{Shift>}[Digit1]{/Shift}{/Meta}");

    expect(handler).not.toHaveBeenCalled();
  });

  it("keeps the newer strip's handler when an older one is released late", async () => {
    const older = vi.fn();
    const newer = vi.fn();
    const releaseOlder = onSelectTab(older);
    const releaseNewer = onSelectTab(newer);
    releaseOlder();
    await renderKeys();

    await userEvent.keyboard("{Meta>}{Shift>}[Digit1]{/Shift}{/Meta}");

    expect(newer).toHaveBeenCalledWith(0);
    expect(older).not.toHaveBeenCalled();
    releaseNewer();
  });
});

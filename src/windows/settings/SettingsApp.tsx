import { useEffect, useState } from "react";
import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { api } from "../../api";
import { useCommandKeys } from "../../commands";
import { SettingsWindow } from "../../components/settings/SettingsWindow";
import { Toasts } from "../../components/Toasts";
import { TooltipHost } from "../../components/TooltipHost";
import { useErrorToasts } from "../../lib/useErrorToasts";
import { useAppearance } from "../../lib/theme";
import { trackWindowFocus } from "../../lib/windowFocus";
import { queryClient as defaultClient } from "../../queries/queryClient";
import { useEvents } from "../../queries/useEvents";

/** The page Rust opened the window at (`?page=`); later pages arrive as `open_settings`. */
function initialPage(): string | undefined {
  return new URLSearchParams(window.location.search).get("page") ?? undefined;
}

/** Shows the pages Rust asks for. */
function useOpenSettings(setPage: (page: string) => void) {
  useEffect(() => {
    const unlisten = api.listen("open_settings", setPage);
    return () => void unlisten.then((fn) => fn()).catch(() => {});
  }, [setPage]);
}

function SettingsRoot() {
  const [page, setPage] = useState(initialPage);
  useOpenSettings(setPage);
  useCommandKeys();
  useEvents();
  useAppearance();
  useErrorToasts();
  useEffect(trackWindowFocus, []);

  return (
    <>
      <SettingsWindow page={page} onPage={setPage} />
      <Toasts />
      <TooltipHost />
    </>
  );
}

/** The Settings window's page; tests pass a fresh query client. */
export function SettingsApp({ client = defaultClient }: { client?: QueryClient }) {
  return (
    <QueryClientProvider client={client}>
      <SettingsRoot />
    </QueryClientProvider>
  );
}

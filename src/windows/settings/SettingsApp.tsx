import { useCallback, useEffect, useState } from "react";
import { QueryClientProvider, type QueryClient } from "@tanstack/react-query";
import { api } from "../../api";
import { useCommandKeys } from "../../commands";
import { SettingsWindow, type SettingsRequest } from "../../components/settings/SettingsWindow";
import { ConfirmHost } from "../../components/ConfirmSheet";
import { MenuHost } from "../../components/menu";
import { Toasts } from "../../components/Toasts";
import { TooltipHost } from "../../components/TooltipHost";
import { useErrorToasts } from "../../lib/useErrorToasts";
import { useAppearance } from "../../lib/theme";
import { trackWindowFocus } from "../../lib/windowFocus";
import { queryClient as defaultClient } from "../../queries/queryClient";
import { useEvents } from "../../queries/useEvents";

/** The page or row Rust opened the window at (`?page=`); later ones arrive as `open_settings`. */
function initialRequest(): SettingsRequest {
  return { id: new URLSearchParams(window.location.search).get("page") ?? undefined };
}

/** Shows the pages and rows Rust asks for. */
function useOpenSettings(open: (id: string) => void) {
  useEffect(() => {
    const unlisten = api.listen("open_settings", open);
    return () => void unlisten.then((fn) => fn()).catch(() => {});
  }, [open]);
}

function SettingsRoot() {
  const [request, setRequest] = useState(initialRequest);
  const open = useCallback((id: string) => setRequest({ id }), []);
  useOpenSettings(open);
  useCommandKeys();
  useEvents();
  useAppearance();
  useErrorToasts();
  useEffect(trackWindowFocus, []);

  return (
    <>
      <SettingsWindow request={request} onOpen={open} />
      <Toasts />
      <TooltipHost />
      <MenuHost />
      <ConfirmHost />
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

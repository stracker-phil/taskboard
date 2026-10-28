import { useEffect } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { useCommandKeys } from "./commands";
import { ConfirmHost } from "./components/ConfirmSheet";
import { Toasts } from "./components/Toasts";
import { useErrorToasts } from "./lib/useErrorToasts";
import { trackWindowFocus } from "./lib/windowFocus";
import { queryClient } from "./queries/queryClient";
import { usePinExpiry } from "./queries/pins";
import { useEvents } from "./queries/useEvents";
import { MainBody } from "./shell/MainBody";
import { Window } from "./shell/Window";
import { useAppEvents } from "./state/useAppEvents";
import { useStore } from "./state/store";

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <MainWindow />
    </QueryClientProvider>
  );
}

function MainWindow() {
  useAppEvents();
  useCommandKeys();
  useEvents();
  usePinExpiry();
  useErrorToasts();
  useEffect(trackWindowFocus, []);
  const sidebarHidden = useStore((s) => s.sidebarHidden);
  return (
    <>
      <Window sidebarHidden={sidebarHidden}>
        <MainBody />
      </Window>
      <ConfirmHost />
      <Toasts />
    </>
  );
}

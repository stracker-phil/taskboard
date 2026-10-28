import { useEffect, useRef } from "react";
import { api } from "../api";

/** How long an issue stays open before leaving it counts as having seen it (J/K skimming counts too). */
const SEEN_AFTER_MS = 1000;

/** Marks the shown issue seen when the owner leaves it, so its banner and new activity stay while they read. */
export function useSeenOnLeave(id: string, hasUnseen: boolean) {
  const unseen = useRef(hasUnseen);
  useEffect(() => {
    unseen.current = hasUnseen;
  }, [hasUnseen]);
  useEffect(() => {
    const opened = Date.now();
    return () => {
      if (unseen.current && Date.now() - opened >= SEEN_AFTER_MS) void api.issuesMarkSeen([id]).catch(() => {});
    };
  }, [id]);
}

import type { LocalDays } from "../api/types";

const DAY_MS = 24 * 60 * 60 * 1000;

/** The owner's day boundaries with overrides; by default the current UTC day in UTC. */
export function localDays(patch: Partial<LocalDays> = {}): LocalDays {
  const today = Math.floor(Date.now() / DAY_MS) * DAY_MS;
  return {
    yesterday: new Date(today - DAY_MS).toISOString(),
    today: new Date(today).toISOString(),
    tomorrow: new Date(today + DAY_MS).toISOString(),
    timeZone: "UTC",
    ...patch,
  };
}

/** Thursday 12 November 2026 in Berlin (UTC+1): each local day starts at 23:00 UTC of the day before. */
export const BERLIN_DAYS: LocalDays = {
  yesterday: "2026-11-10T23:00:00.000Z",
  today: "2026-11-11T23:00:00.000Z",
  tomorrow: "2026-11-12T23:00:00.000Z",
  timeZone: "Europe/Berlin",
};

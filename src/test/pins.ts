import type { PinItem, Pinned, Pins } from "../api/types";
import { SAM } from "./issues";

/** The issue a pin points at (the "Website relaunch" pricing page); override per test. */
export const issueTarget = (patch: Partial<Extract<Pinned, { kind: "issue" }>> = {}): Pinned => ({
  kind: "issue",
  id: "10042",
  key: "NW-1042",
  summary: "Redesign the pricing page",
  style: "progress",
  gone: false,
  ...patch,
});

/** A comment of Sam Lee on NW-1042. */
export const commentTarget = (patch: Partial<Extract<Pinned, { kind: "comment" }>> = {}): Pinned => ({
  kind: "comment",
  id: "20001",
  issueId: "10042",
  issueKey: "NW-1042",
  author: SAM,
  excerpt: "Please check the postcode rules first.",
  ...patch,
});

/** A pinned filter list of the "Website relaunch" Space. */
export const listTarget = (patch: Partial<Extract<Pinned, { kind: "list" }>> = {}): Pinned => ({
  kind: "list",
  id: 21,
  name: "My open issues",
  space: "Website relaunch",
  ...patch,
});

/** An active pin on an issue unless overridden; pass `target: commentTarget()` or `listTarget()` for the others. */
export function pinItem(patch: Partial<PinItem> = {}): PinItem {
  return { id: 1, target: issueTarget(), reason: null, timeLeft: "2d left", expiresAt: null, ...patch };
}

export const pins = (active: PinItem[] = [], recentlyUnpinned: PinItem[] = []): Pins => ({ active, recentlyUnpinned });

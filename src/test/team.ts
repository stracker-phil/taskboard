import type { Person, TeamCell, TeamMatrix } from "../api/types";
import { ALEX, SAM } from "./issues";

export const JORDAN: Person = { accountId: "demo-account-3", name: "Jordan Rivera", initials: "JR", color: 3 };

/** A cell of the Team matrix: nobody active, no activity; override per cell. */
export const teamCell = (patch: Partial<TeamCell> = {}): TeamCell => ({ active: false, assignee: false, involved: false, ...patch });

/** Three people against the "Website relaunch" (10001) and "Lumen iOS app" (10002) Spaces. */
export function teamMatrix(patch: Partial<TeamMatrix> = {}): TeamMatrix {
  return {
    spaces: ["10001", "10002"],
    people: [
      { person: ALEX, cells: [teamCell({ active: true, assignee: true, involved: true }), teamCell()] },
      { person: SAM, cells: [teamCell({ involved: true }), teamCell({ active: true, involved: true })] },
      { person: JORDAN, cells: [teamCell(), teamCell({ involved: true })] },
    ],
    ...patch,
  };
}

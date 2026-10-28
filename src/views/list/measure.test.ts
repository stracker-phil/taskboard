import { describe, expect, it } from "vitest";
import { ALEX, row, SAM } from "../../test/issues";
import { columnDef } from "./columns";
import { canvasMeasure, columnWidths, FONT, fittingColumns, gridTemplate, listColumns, type MeasuredColumn, type MeasureText, widthOf } from "./measure";

/** Seven pixels per character, whatever the font. */
const chars7: MeasureText = (text) => text.length * 7;

const column = (id: string, header: string): MeasuredColumn => ({ id, header, cellWidth: columnDef(id).measure });
/** The pixel widths after the title's `minmax(0, 1fr)`. */
const widths = (template: string) => template.split(" ").slice(2);

describe("listColumns", () => {
  it("gives the title the remaining space and no columns nothing else", () => {
    expect(listColumns([row()], [], chars7)).toBe("minmax(0, 1fr)");
  });

  it("starts with the title column, then one pixel width per column in order", () => {
    const template = listColumns([row()], [column("assignee", "Assignee"), column("priority", "Priority")], chars7);

    expect(template).toMatch(/^minmax\(0, 1fr\) \d+px \d+px$/);
  });

  it("sizes a column to its uppercased header plus tracking when the cells are narrower", () => {
    const low = row({ priority: { name: "Low", level: 1 } });

    // 8 chars * 7 + 0.55 * 8 = 60.4, rounded up; the cell is 14 + 6 + 21 = 41.
    expect(listColumns([low], [column("priority", "Priority")], chars7)).toBe("minmax(0, 1fr) 61px");
  });

  it("sizes a column to its widest cell when that is wider than the header", () => {
    const highest = row({ priority: { name: "Highest", level: 3 } });

    // 14 + 6 + 7 * 7 = 69
    expect(listColumns([highest], [column("priority", "Priority")], chars7)).toBe("minmax(0, 1fr) 69px");
  });

  it("measures the header in capitals", () => {
    const seen: string[] = [];
    const spy: MeasureText = (text, font) => {
      if (font === FONT.header) seen.push(text);
      return text.length * 7;
    };

    listColumns([], [column("due", "Due date")], spy);

    expect(seen).toEqual(["DUE DATE"]);
  });

  it("uses the widest cell over all rows, including rows of collapsed groups and off-screen rows", () => {
    const rows = [
      row({ id: "1", priority: { name: "Low", level: 1 } }),
      row({ id: "2", priority: { name: "Medium", level: 2 } }),
      row({ id: "3", priority: { name: "Highest priority", level: 3 } }),
    ];

    // 14 + 6 + 16 * 7 = 132
    expect(listColumns(rows, [column("priority", "Priority")], chars7)).toBe("minmax(0, 1fr) 132px");
  });

  it("falls back to the header width when no row has a value", () => {
    expect(listColumns([row(), row({ id: "2" })], [column("epic", "Epic")], chars7)).toBe("minmax(0, 1fr) 31px");
  });

  it("changes the template when the columns change", () => {
    // Five avatars are 26 + 4 * 18 = 98px, wider than the Assignee header (61px); Priority stays at its header's 61px.
    const rows = [row({ assignees: [ALEX, SAM, ALEX, SAM, ALEX], priority: { name: "High", level: 2 } })];

    const one = listColumns(rows, [column("assignee", "Assignee")], chars7);
    const two = listColumns(rows, [column("assignee", "Assignee"), column("priority", "Priority")], chars7);
    const swapped = listColumns(rows, [column("priority", "Priority"), column("assignee", "Assignee")], chars7);

    expect(one.split(" ")).toHaveLength(3);
    expect(two.split(" ")).toHaveLength(4);
    expect(swapped).not.toBe(two);
    expect(widths(two)).toEqual(["98px", "61px"]);
    expect(widths(swapped)).toEqual(["61px", "98px"]);
  });

  describe("avatar column", () => {
    it.each([
      [1, "Who", 26],
      [2, "Who", 44],
      [4, "Who", 80],
    ])("is as wide as a stack of %i avatars when that is wider than the header", (count, header, expected) => {
      const people = Array.from({ length: count }, () => ALEX);

      expect(listColumns([row({ assignees: people })], [column("assignee", header)], chars7)).toBe(`minmax(0, 1fr) ${expected}px`);
    });

    it("is as wide as its header when nobody is assigned", () => {
      // 8 * 7 + 0.55 * 8 = 60.4
      expect(listColumns([row()], [column("assignee", "Assignee")], chars7)).toBe("minmax(0, 1fr) 61px");
    });
  });

  it("sizes an epic column to its chip: the name plus 16px of padding", () => {
    const epic = row({ epic: { id: "9", name: "Launch", color: 2 } });

    expect(listColumns([epic], [column("epic", "Epic")], chars7)).toBe("minmax(0, 1fr) 58px");
  });

  it("sizes a status column to the widest status pill, rounded up", () => {
    const rows = [row({ id: "1", statusName: "Done" }), row({ id: "2", statusName: "In Progress" })];

    // 11 * 7 + 3.3 + 22 = 102.3 beats the header: 6 * 7 + 3.3 = 45.3
    expect(listColumns(rows, [column("status", "Status")], chars7)).toBe("minmax(0, 1fr) 103px");
  });

  it("sizes a person custom field to one avatar", () => {
    const reviewed = row({ custom: { customfield_10071: { kind: "person", value: SAM } } });

    expect(listColumns([reviewed], [column("customfield_10071", "RV")], chars7)).toBe("minmax(0, 1fr) 26px");
  });
});

describe("columnWidths", () => {
  it("returns one rounded-up pixel width per column, in order", () => {
    const rows = [row({ priority: { name: "Highest", level: 3 } })];

    expect(columnWidths(rows, [column("priority", "Priority"), column("epic", "Epic")], chars7)).toEqual([69, 31]);
  });

  it("returns nothing for no columns", () => {
    expect(columnWidths([row()], [], chars7)).toEqual([]);
  });
});

describe("gridTemplate", () => {
  it("gives the title the flexible track, then each width in px", () => {
    expect(gridTemplate([61, 98])).toBe("minmax(0, 1fr) 61px 98px");
  });

  it("is only the title track without widths", () => {
    expect(gridTemplate([])).toBe("minmax(0, 1fr)");
  });
});

describe("fittingColumns", () => {
  it("keeps every column when they fit beside the title", () => {
    // 240 + (100 + 20) + (60 + 20) = 440
    expect(fittingColumns([100, 60], 440)).toBe(2);
  });

  it("cuts the first column that overflows by a pixel, and everything after it", () => {
    expect(fittingColumns([100, 60, 10], 439)).toBe(1);
    expect(fittingColumns([100, 60, 10], 119 + 240)).toBe(0);
  });

  it("does not bring back a narrow column behind one that was cut", () => {
    expect(fittingColumns([300, 10], 240 + 100)).toBe(0);
  });

  it("reserves at least 240px for the title", () => {
    expect(fittingColumns([10], 269)).toBe(0);
    expect(fittingColumns([10], 270)).toBe(1);
  });

  it("returns zero without columns, however narrow the list", () => {
    expect(fittingColumns([], 0)).toBe(0);
  });
});

describe("widthOf", () => {
  it("adds the labels' chip padding and a gap between them", () => {
    // (3 * 7 + 16) + 4 + (2 * 7 + 16)
    expect(widthOf.labels(chars7, ["abc", "de"])).toBe(71);
  });

  describe("pill", () => {
    it("is the capitalised name plus tracking, 10px padding and a border on each side", () => {
      // 11 * 7 + 0.3 * 11 + 20 + 2
      expect(widthOf.pill(chars7, "In Progress")).toBeCloseTo(102.3);
    });

    it("measures the name in capitals in the pill font", () => {
      const seen: [string, string][] = [];
      const spy: MeasureText = (text, font) => {
        seen.push([text, font]);
        return text.length * 7;
      };

      widthOf.pill(spy, "In Progress");

      expect(seen).toEqual([["IN PROGRESS", FONT.pill]]);
    });
  });

  it("gives no width to a missing value", () => {
    expect([widthOf.text(chars7, null), widthOf.chip(chars7, null), widthOf.priority(chars7, null), widthOf.avatars(0), widthOf.labels(chars7, [])]).toEqual([0, 0, 0, 0, 0]);
  });
});

describe("canvasMeasure", () => {
  it("estimates seven pixels per character where there is no canvas", () => {
    expect(canvasMeasure()("abcd", FONT.cell)).toBe(28);
  });
});

import type { Layout } from "../api";

const ROWS = 8;
const BOARD_COLUMNS = 3;
const BOARD_CARDS = 3;

/** Text bars of varied, stable widths, so the placeholder reads as rows rather than a block. */
const barWidth = (index: number) => `${40 + ((index * 37) % 45)}%`;

/** What stands in for a view's first issues while they load: rows at the layout's own row height, or Board
 *  columns of cards. It fades in after a moment (`.body-skeleton`), so a quick load never flashes it. */
export function BodySkeleton({ layout }: { layout: Layout }) {
  if (layout === "board") {
    return (
      <div className="body-skeleton body-skeleton--board" aria-busy="true" aria-label="Loading issues">
        {Array.from({ length: BOARD_COLUMNS }, (_, column) => (
          <div key={column} className="body-skeleton__column">
            {Array.from({ length: BOARD_CARDS }, (_, card) => (
              <div key={card} className="body-skeleton__card">
                <span className="body-skeleton__bar" style={{ width: barWidth(column * BOARD_CARDS + card) }} />
              </div>
            ))}
          </div>
        ))}
      </div>
    );
  }
  return (
    <div className={`body-skeleton body-skeleton--${layout}`} aria-busy="true" aria-label="Loading issues">
      {Array.from({ length: ROWS }, (_, row) => (
        <div key={row} className="body-skeleton__row">
          <span className="body-skeleton__bar" style={{ width: barWidth(row) }} />
        </div>
      ))}
    </div>
  );
}

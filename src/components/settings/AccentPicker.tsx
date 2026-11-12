import type { Accent } from "../../api";

/** System (the macOS accent color), then the family's nine hues (ADR 0073). */
export const ACCENTS: Accent[] = ["system", "orange", "gold", "red", "magenta", "purple", "indigo", "blue", "teal", "graphite"];

export const ACCENT_NAMES: Record<Accent, string> = {
  system: "System",
  orange: "Orange",
  gold: "Gold",
  red: "Red",
  magenta: "Magenta",
  purple: "Purple",
  indigo: "Indigo",
  blue: "Blue",
  teal: "Teal",
  graphite: "Graphite",
};

interface Props {
  label: string;
  value: Accent | null;
  onChange: (accent: Accent | null) => void;
  /** Offers a first swatch, ringed in this accent, for following the app's accent (`null`). */
  appAccent?: Accent;
}

/** The accent swatches, as macOS draws them: a click picks one, the picked one has a ring. */
export function AccentPicker({ label, value, onChange, appAccent }: Props) {
  const swatch = (accent: Accent | null, name: string, className: string) => (
    <button
      key={accent ?? "app"}
      type="button"
      role="radio"
      aria-checked={value === accent}
      aria-label={name}
      title={name}
      className={className}
      onClick={() => onChange(accent)}
    />
  );
  return (
    <div className="accent-picker" role="radiogroup" aria-label={label}>
      {appAccent && swatch(null, `Same as the app (${ACCENT_NAMES[appAccent]})`, `accent-swatch accent-swatch--app accent-swatch--${appAccent}`)}
      {ACCENTS.map((accent) => swatch(accent, ACCENT_NAMES[accent], `accent-swatch accent-swatch--${accent}`))}
    </div>
  );
}

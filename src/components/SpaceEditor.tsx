/** A Space's name, icon letter, colour and client label: the Spaces settings page and the sidebar's popover. */
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { api, toAppError, type SpaceNode, type SpaceUpdate } from "../api";
import { SIDEBAR_KEY } from "../queries/sidebar";
import { fail } from "../state/store";
import { Row } from "./settings/controls";
import { BufferedInput, FieldError, TextField } from "./settings/fields";
import { fieldErrorOf, type FieldErrors } from "./settings/useBusyAction";
import { SpaceTile } from "./SpaceTile";

/** The Space palette (`--space-tile-1..12`), by the colour number Rust stores. */
const SPACE_COLORS = [
  { id: "1", label: "Blue" },
  { id: "2", label: "Slate" },
  { id: "3", label: "Violet" },
  { id: "4", label: "Teal" },
  { id: "5", label: "Amber" },
  { id: "6", label: "Red" },
  { id: "7", label: "Pink" },
  { id: "8", label: "Green" },
  { id: "9", label: "Indigo" },
  { id: "10", label: "Cyan" },
  { id: "11", label: "Orange" },
  { id: "12", label: "Stone" },
] as const;

/** Which fields show: all of them (the settings page), or one popover's. */
export type SpaceFieldsFocus = "appearance" | "client";

/** Saves one field with the others as they are; Rust's field errors show under their field. */
function useSpaceUpdate(space: SpaceNode) {
  const client = useQueryClient();
  const [errors, setErrors] = useState<FieldErrors>({});
  const projectName = space.projectName;
  const current: SpaceUpdate = {
    name: space.name === projectName ? null : space.name,
    icon: space.icon,
    color: space.color,
    client: space.client,
  };

  const save = (field: keyof SpaceUpdate, value: string) => {
    setErrors((all) => Object.fromEntries(Object.entries(all).filter(([key]) => key !== field)));
    api.spaceUpdate(space.projectId, { ...current, [field]: value.trim() || null }).then(
      () => void client.invalidateQueries({ queryKey: SIDEBAR_KEY }),
      (e: unknown) => {
        const fieldError = fieldErrorOf(toAppError(e));
        if (fieldError) setErrors((all) => ({ ...all, ...fieldError }));
        else fail(e, `Could not change ${space.name}`);
      },
    );
  };
  return { current, projectName, errors, save };
}

/** The fields of a Space; each saves on its own (text on blur or ↵, a colour on click). */
export function SpaceFields({ space, focus }: { space: SpaceNode; focus?: SpaceFieldsFocus }) {
  const { current, projectName, errors, save } = useSpaceUpdate(space);
  const iconId = useId();
  const showAppearance = focus !== "client";
  return (
    <div className="space-fields">
      {!focus && (
        <TextField
          label="Name"
          description="Empty uses the Jira project's name."
          placeholder={projectName ?? space.name}
          value={current.name ?? ""}
          error={errors.name}
          onCommit={(v) => save("name", v)}
        />
      )}
      {showAppearance && (
        <>
          <Row label="Icon letter" htmlFor={iconId}>
            <div className="settings-field">
              <BufferedInput
                id={iconId}
                className="input space-icon-input"
                maxLength={1}
                placeholder={space.name.charAt(0).toUpperCase()}
                aria-invalid={errors.icon ? true : undefined}
                aria-describedby={errors.icon ? `${iconId}-error` : undefined}
                value={current.icon ?? ""}
                onCommit={(v) => save("icon", v)}
              />
              <FieldError id={`${iconId}-error`} message={errors.icon} />
            </div>
          </Row>
          <Row label="Colour">
            <div className="settings-field">
              <div className="space-colors">
                <SpaceTile space={space} size={22} />
                <div className="space-swatches" role="group" aria-label="Colour">
                  {SPACE_COLORS.map((c) => (
                    <button
                      key={c.id}
                      type="button"
                      className="space-swatch"
                      aria-label={c.label}
                      aria-pressed={space.color === c.id}
                      style={{ background: `var(--space-tile-${c.id})` }}
                      onClick={() => save("color", c.id)}
                    />
                  ))}
                </div>
              </div>
              <FieldError id={`${iconId}-color-error`} message={errors.color} />
            </div>
          </Row>
        </>
      )}
      {focus !== "appearance" && (
        <TextField
          label="Client"
          description="Empty marks the Space as internal."
          placeholder="Internal"
          value={current.client ?? ""}
          error={errors.client}
          onCommit={(v) => save("client", v)}
        />
      )}
    </div>
  );
}

const POPOVER_TITLES: Record<SpaceFieldsFocus, string> = { appearance: "Colour and icon", client: "Client label" };

/** The sidebar's small popover for one Space. Esc, Done or a click outside close it; a field being edited saves
 *  first. */
export function SpacePopover({ space, focus, onClose }: { space: SpaceNode; focus: SpaceFieldsFocus; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  });

  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("input, button")?.focus();
    const close = () => {
      // Blurring commits the field being edited.
      if (ref.current?.contains(document.activeElement)) (document.activeElement as HTMLElement).blur();
      onCloseRef.current();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      close();
    };
    const onPointer = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) close();
    };
    window.addEventListener("keydown", onKey, true);
    document.addEventListener("mousedown", onPointer, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      document.removeEventListener("mousedown", onPointer, true);
    };
  }, []);

  return createPortal(
    <div ref={ref} className="space-popover" role="dialog" aria-labelledby={titleId}>
      <h2 id={titleId}>{POPOVER_TITLES[focus]}</h2>
      <SpaceFields space={space} focus={focus} />
      <div className="space-popover-actions">
        <button type="button" className="btn btn-primary" onClick={onClose}>
          Done
        </button>
      </div>
    </div>,
    document.body,
  );
}

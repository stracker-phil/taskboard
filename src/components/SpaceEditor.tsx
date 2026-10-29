/** A Space's name, icon, color and client label: the Spaces settings page and the sidebar's client label popover. */
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { api, toAppError, type SpaceNode, type SpaceUpdate } from "../api";
import { SIDEBAR_KEY } from "../queries/sidebar";
import { fail } from "../state/store";
import { Row } from "./settings/controls";
import { FieldError, TextField } from "./settings/fields";
import { fieldErrorOf, type FieldErrors } from "./settings/useBusyAction";
import { LookEditor } from "./look/LookEditor";
import type { Look } from "./look/Tile";
import { spaceUpdateOf } from "./spaceUpdates";

/** Saves one field with the others as they are; Rust's field errors show under their field. */
function useSpaceUpdate(space: SpaceNode) {
  const client = useQueryClient();
  const [errors, setErrors] = useState<FieldErrors>({});
  const projectName = space.projectName;
  const current = spaceUpdateOf(space);

  const save = (field: keyof SpaceUpdate, value: string | null) => {
    setErrors((all) => Object.fromEntries(Object.entries(all).filter(([key]) => key !== field)));
    saveUpdate({ ...current, [field]: value?.trim() || null });
  };
  const saveLook = (look: Look) => {
    setErrors((all) => Object.fromEntries(Object.entries(all).filter(([key]) => key !== "icon" && key !== "color")));
    saveUpdate({ ...current, icon: look.icon, color: look.color });
  };
  const saveUpdate = (update: SpaceUpdate) => {
    api.spaceUpdate(space.projectId, update).then(
      () => void client.invalidateQueries({ queryKey: SIDEBAR_KEY }),
      (e: unknown) => {
        const fieldError = fieldErrorOf(toAppError(e));
        if (fieldError) setErrors((all) => ({ ...all, ...fieldError }));
        else fail(e, `Could not change ${space.name}`);
      },
    );
  };
  return { current, projectName, errors, save, saveLook };
}

/** The fields of a Space, or only its client label; each saves on its own (text on blur or ↵, the icon and color
 *  on click). */
export function SpaceFields({ space, clientOnly = false }: { space: SpaceNode; clientOnly?: boolean }) {
  const { current, projectName, errors, save, saveLook } = useSpaceUpdate(space);
  return (
    <div className="space-fields">
      {!clientOnly && (
        <TextField
          label="Name"
          description="Empty uses the Jira project's name."
          placeholder={projectName ?? space.name}
          value={current.name ?? ""}
          error={errors.name}
          onCommit={(v) => save("name", v)}
        />
      )}
      {!clientOnly && (
        <Row label="Icon and color">
          <div className="settings-field">
            <LookEditor look={{ icon: current.icon, color: current.color ?? "1" }} fallback={space.name.charAt(0)} onChange={saveLook} />
            <FieldError id={`space-${space.projectId}-icon-error`} message={errors.icon ?? errors.color} />
          </div>
        </Row>
      )}
      <TextField
        label="Client"
        description="Empty marks the Space as internal."
        placeholder="Internal"
        value={current.client ?? ""}
        error={errors.client}
        onCommit={(v) => save("client", v)}
      />
    </div>
  );
}

/** The sidebar's popover for a Space's client label. Esc, Done or a click outside close it; a field being edited saves
 *  first. */
export function SpacePopover({ space, onClose }: { space: SpaceNode; onClose: () => void }) {
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
      <h2 id={titleId}>Client label</h2>
      <SpaceFields space={space} clientOnly />
      <div className="space-popover-actions">
        <button type="button" className="btn btn-primary" onClick={onClose}>
          Done
        </button>
      </div>
    </div>,
    document.body,
  );
}

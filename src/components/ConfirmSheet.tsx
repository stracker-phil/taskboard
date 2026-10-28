import { useId } from "react";
import { useStore } from "../state/store";
import { Modal } from "./Modal";

interface Props {
  title: string;
  text: string;
  action: string;
  onCancel: () => void;
  onConfirm: () => void;
}

/** A confirmation before a destructive action: Cancel (also Escape) and the red action button. */
export function ConfirmSheet({ title, text, action, onCancel, onConfirm }: Props) {
  const id = useId();
  return (
    <Modal role="alertdialog" labelledBy={`${id}-title`} describedBy={`${id}-text`} onClose={onCancel}>
      <h2 id={`${id}-title`}>{title}</h2>
      <p id={`${id}-text`}>{text}</p>
      <div className="actions">
        <button type="button" className="btn" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className="btn btn-danger" onClick={onConfirm}>
          {action}
        </button>
      </div>
    </Modal>
  );
}

/** Shows the question `useStore.getState().confirm` asked, in whichever window mounts it. */
export function ConfirmHost() {
  const confirmation = useStore((s) => s.confirmation);
  if (!confirmation) return null;
  return (
    <ConfirmSheet
      title={confirmation.title}
      text={confirmation.text}
      action={confirmation.action}
      onCancel={() => confirmation.answer(false)}
      onConfirm={() => confirmation.answer(true)}
    />
  );
}

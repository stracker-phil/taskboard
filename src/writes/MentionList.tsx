import { useLayoutEffect, useRef, useState, type RefObject } from "react";
import { createPortal } from "react-dom";
import type { Person } from "../api";
import { Avatar } from "../components/issue/Avatar";
import { place } from "../components/menu/placement";
import { caretBox } from "./caretBox";

interface Props {
  people: Person[];
  active: number;
  input: RefObject<HTMLTextAreaElement | null>;
  /** Where the `@` is: the list opens under its line, or above when there is no room. */
  mentionStart: number;
  onHover: (index: number) => void;
  onPick: (person: Person) => void;
}

/** The team members matching a typed `@name`. Focus stays in the comment field, which moves through the list. */
export function MentionList({ people, active, input, mentionStart, onHover, onPick }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<{ left: number; top: number } | null>(null);

  useLayoutEffect(() => {
    const panel = ref.current;
    if (!panel || !input.current) return;
    const size = { width: panel.offsetWidth, height: panel.offsetHeight };
    const view = { width: window.innerWidth, height: window.innerHeight };
    setAt(place({ below: caretBox(input.current, mentionStart) }, size, view));
  }, [input, mentionStart, people]);

  return createPortal(
    <div
      ref={ref}
      id="mention-list"
      className="popover menu mention-list"
      role="listbox"
      aria-label="People to mention"
      style={at ? { left: at.left, top: at.top } : { visibility: "hidden" }}
      // Keeps the focus, and the caret, in the comment field.
      onMouseDown={(e) => e.preventDefault()}
    >
      {people.map((person, index) => (
        <div
          key={person.accountId}
          id={`mention-${person.accountId}`}
          role="option"
          aria-selected={index === active}
          className="menu__item"
          data-active={index === active || undefined}
          onMouseEnter={() => onHover(index)}
          onClick={() => onPick(person)}
        >
          <span className="menu__icon" aria-hidden="true">
            <Avatar person={person} size={18} />
          </span>
          <span className="menu__text">{person.name}</span>
        </div>
      ))}
    </div>,
    document.body,
  );
}

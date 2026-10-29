import type { Person } from "../../api";
import { useUserAvatar } from "../../queries/avatars";

/** A person's Jira profile picture (a `data:` URL from Rust), else their initials on their color. */
export function Avatar({ person, size = 26 }: { person: Person; size?: number }) {
  const { data: picture } = useUserAvatar(person.accountId);
  return (
    <span
      className={`avatar avatar--${person.color}`}
      style={{ width: size, height: size }}
      title={person.name}
      aria-label={person.name}
      role="img"
    >
      {picture ? <img className="avatar__picture" src={picture} alt="" draggable={false} /> : person.initials}
    </span>
  );
}

export function AvatarStack({ people }: { people: Person[] }) {
  return (
    <span className="avatar-stack">
      {people.map((person) => (
        <Avatar key={person.accountId} person={person} />
      ))}
    </span>
  );
}

import type { Person } from "../../api";

/** A person's initials on their color. */
export function Avatar({ person, size = 26 }: { person: Person; size?: number }) {
  return (
    <span
      className={`avatar avatar--${person.color}`}
      style={{ width: size, height: size }}
      title={person.name}
      aria-label={person.name}
      role="img"
    >
      {person.initials}
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

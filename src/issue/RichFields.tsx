import type { CustomField } from "../api";
import { hasContent } from "../lib/jiraHtml";
import { JiraHtml } from "./JiraHtml";

/** The rich text custom fields with content (acceptance criteria, steps to reproduce), each under its name. */
export function RichFields({ fields, issueId, variant }: { fields: CustomField[]; issueId: string; variant: "issue" | "peek" }) {
  return fields
    .filter((field) => field.html && hasContent(field.html))
    .map((field) => (
      <section key={field.id} className={`rich-field rich-field--${variant}`} aria-label={field.name}>
        <h2>{field.name}</h2>
        <JiraHtml html={field.html ?? ""} issueId={issueId} />
      </section>
    ));
}

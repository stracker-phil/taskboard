import { ChevronDown } from "lucide-react";

/** The one chevron of every disclosure (Spaces, groups, subtrees): points down when open, right when closed, and
 *  turns between them. Its button decides the color and when it shows. */
export function DisclosureChevron({ open }: { open: boolean }) {
  return <ChevronDown className={open ? "disclosure-chevron" : "disclosure-chevron disclosure-chevron--closed"} size={12} strokeWidth={2.4} aria-hidden="true" />;
}

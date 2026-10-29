import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** Outside the issue's key: a sync doesn't change an attachment's bytes, so it never refetches them. */
export const attachmentImageKey = (issueId: string, attachmentId: string) => ["attachmentImage", issueId, attachmentId] as const;

/** An image attached to the issue as a `data:` URL, or null to keep its chip (ADR 0039); asked once `enabled`. */
export const useAttachmentImage = (issueId: string, attachmentId: string, enabled: boolean) =>
  useQuery({
    queryKey: attachmentImageKey(issueId, attachmentId),
    queryFn: () => api.attachmentImage(issueId, attachmentId),
    enabled,
  });

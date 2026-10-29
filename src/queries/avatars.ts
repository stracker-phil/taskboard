import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** Rust refreshes a picture at most weekly; an hour keeps a long session from holding an old one forever. */
const AVATAR_STALE_MS = 60 * 60 * 1000;

export const avatarKey = (accountId: string) => ["avatar", accountId] as const;

/** The person's picture as a `data:` URL, or null for initials (ADR 0032). */
export const useUserAvatar = (accountId: string) =>
  useQuery({ queryKey: avatarKey(accountId), queryFn: () => api.userAvatar(accountId), staleTime: AVATAR_STALE_MS });

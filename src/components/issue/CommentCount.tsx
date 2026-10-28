import { MessageCircle } from "lucide-react";

export function CommentCount({ count }: { count: number }) {
  if (count <= 0) return null;
  return (
    <span className="comment-count" aria-label={count === 1 ? "1 comment" : `${count} comments`}>
      <MessageCircle size={12} strokeWidth={2} aria-hidden="true" />
      {count}
    </span>
  );
}

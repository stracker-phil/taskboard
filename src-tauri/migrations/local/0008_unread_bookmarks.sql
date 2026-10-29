UPDATE filter_list SET name = 'Unread' WHERE kind = 'recent' AND name = 'Recently changed';
UPDATE filter_list SET name = 'Bookmarks' WHERE kind = 'pinned' AND name = 'Pinned';

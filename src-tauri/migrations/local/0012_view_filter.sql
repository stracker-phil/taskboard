-- A view's own conditions (ADR 0033): the same JSON filter as filter_list.filter, ANDed with its list's chain.
-- NULL holds no conditions. A view now decides how issues show and may narrow which.
ALTER TABLE view ADD COLUMN filter TEXT;

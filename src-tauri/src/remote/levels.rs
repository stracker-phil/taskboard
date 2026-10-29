//! The sidebar as the command line names it: a level is `everything`, `recent`, `pinned`, a Space's key or
//! name, a filter list's name (`NW/Open bugs` inside a Space) or any level's id; a view is its id or name.
//! A unique start of a name is enough.

use crate::cli::rows::{LevelRow, SpaceRow, ViewRow};
use crate::lists::{Kind, ListNode, SidebarTree};
use crate::views::View;

/// One level and the words it answers to, the first being how messages show it.
struct Named {
    id: i64,
    words: Vec<String>,
}

fn named(tree: &SidebarTree) -> Vec<Named> {
    let home = std::iter::once(&tree.everything)
        .chain(&tree.home)
        .map(|node| Named {
            id: node.id,
            words: home_words(node),
        });
    let spaces = tree.spaces.iter().flat_map(|space| {
        let own = Named {
            id: space.list_id,
            words: vec![space.key.clone(), space.name.clone()],
        };
        let lists = space.lists.iter().map(|list| Named {
            id: list.id,
            words: vec![format!("{}/{}", space.key, list.name), list.name.clone()],
        });
        std::iter::once(own).chain(lists)
    });
    home.chain(spaces).collect()
}

fn home_words(node: &ListNode) -> Vec<String> {
    let keyword = match node.kind {
        Kind::Everything => Some("everything"),
        Kind::Recent => Some("recent"),
        Kind::Pinned => Some("pinned"),
        Kind::Space | Kind::List => None,
    };
    std::iter::once(node.name.clone())
        .chain(keyword.map(String::from))
        .collect()
}

/// The level's id, or why the word names none or several.
pub fn find_level(tree: &SidebarTree, word: &str) -> Result<i64, String> {
    let levels = named(tree);
    let word = word.trim();
    if let Ok(id) = word.parse::<i64>() {
        return levels
            .iter()
            .find(|level| level.id == id)
            .map(|level| level.id)
            .ok_or_else(|| format!("No sidebar level has the id {id}; see `taskboard filters`."));
    }
    pick(&levels, word, "Space or filter list", "`taskboard spaces` and `taskboard filters`")
}

/// The view's id, or why the word names none or several of `views`.
pub fn find_view(views: &[View], word: &str) -> Result<i64, String> {
    let word = word.trim();
    if let Ok(id) = word.parse::<i64>()
        && views.iter().any(|view| view.id == id)
    {
        return Ok(id);
    }
    let named: Vec<Named> = views
        .iter()
        .map(|view| Named {
            id: view.id,
            words: vec![view.name.clone()],
        })
        .collect();
    pick(&named, word, "view", "`taskboard views`")
}

/// An exact name (any case) first, else a unique start of one.
fn pick(candidates: &[Named], word: &str, what: &str, listing: &str) -> Result<i64, String> {
    let wanted = word.to_lowercase();
    let matching = |accept: &dyn Fn(&str) -> bool| -> Vec<&Named> {
        candidates
            .iter()
            .filter(|c| c.words.iter().any(|w| accept(&w.to_lowercase())))
            .collect()
    };
    let exact = matching(&|w| w == wanted);
    let found = if exact.is_empty() && !wanted.is_empty() {
        matching(&|w| w.starts_with(&wanted))
    } else {
        exact
    };
    match found.as_slice() {
        [one] => Ok(one.id),
        [] => Err(format!("No {what} is called “{word}”; see {listing}.")),
        several => {
            let names: Vec<String> = several
                .iter()
                .map(|c| format!("{} ({})", c.words[0], c.id))
                .collect();
            Err(format!("“{word}” could be {}; name one by its id.", names.join(", ")))
        }
    }
}

/// How messages name a level: `Everything`, `NW Website relaunch`, `NW/Open bugs`.
pub fn level_label(tree: &SidebarTree, id: i64) -> Option<String> {
    if let Some(node) = std::iter::once(&tree.everything)
        .chain(&tree.home)
        .find(|n| n.id == id)
    {
        return Some(node.name.clone());
    }
    tree.spaces.iter().find_map(|space| {
        if space.list_id == id {
            return Some(format!("{} {}", space.key, space.name));
        }
        space
            .lists
            .iter()
            .find(|list| list.id == id)
            .map(|list| format!("{}/{}", space.key, list.name))
    })
}

pub fn space_rows(tree: &SidebarTree, current: Option<i64>) -> Vec<SpaceRow> {
    tree.spaces
        .iter()
        .map(|space| SpaceRow {
            id: space.list_id,
            key: space.key.clone(),
            name: space.name.clone(),
            client: space.client.clone(),
            open: space.open_count,
            current: current == Some(space.list_id),
        })
        .collect()
}

/// Everything, Home's lists and every Space's filter lists; `space` (a Space's level id) keeps only that
/// Space's.
pub fn level_rows(tree: &SidebarTree, space: Option<i64>, current: Option<i64>) -> Vec<LevelRow> {
    let row = |node: &ListNode, key: Option<&str>| LevelRow {
        id: node.id,
        kind: node.kind,
        name: node.name.clone(),
        space: key.map(String::from),
        open: node.count,
        current: current == Some(node.id),
    };
    let home = std::iter::once(&tree.everything)
        .chain(&tree.home)
        .filter(|_| space.is_none())
        .map(|node| row(node, None));
    let lists = tree
        .spaces
        .iter()
        .filter(|s| space.is_none_or(|id| id == s.list_id))
        .flat_map(|s| s.lists.iter().map(move |list| (list, s.key.as_str())))
        .map(|(list, key)| row(list, Some(key)));
    home.chain(lists).collect()
}

pub fn view_rows(views: &[View], current: Option<i64>) -> Vec<ViewRow> {
    views
        .iter()
        .map(|view| ViewRow {
            id: view.id,
            name: view.name.clone(),
            layout: view.layout,
            is_default: view.is_default,
            current: current == Some(view.id),
        })
        .collect()
}

#[cfg(test)]
mod tests;

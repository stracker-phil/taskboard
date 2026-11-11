//! Bare `http(s)://` addresses in typed text, which Markdown leaves as plain text.

pub(super) enum Piece<'a> {
    Text(&'a str),
    Link(&'a str),
}

/// The text as plain runs and bare `http(s)://` links; a scheme with nothing after it stays text.
pub(super) fn split(text: &str) -> Vec<Piece<'_>> {
    let mut pieces = Vec::new();
    let mut rest = text;
    while let Some(start) = next_address(rest) {
        let (before, from_address) = rest.split_at(start);
        let (address, after) = from_address.split_at(address_end(from_address));
        if !before.is_empty() {
            pieces.push(Piece::Text(before));
        }
        let only_scheme = address == "https://" || address == "http://";
        pieces.push(if only_scheme {
            Piece::Text(address)
        } else {
            Piece::Link(address)
        });
        rest = after;
    }
    if !rest.is_empty() {
        pieces.push(Piece::Text(rest));
    }
    pieces
}

fn next_address(text: &str) -> Option<usize> {
    [text.find("https://"), text.find("http://")]
        .into_iter()
        .flatten()
        .min()
}

/// The address runs to the next space; closing punctuation after it belongs to the sentence.
fn address_end(from_address: &str) -> usize {
    let word = from_address
        .find(char::is_whitespace)
        .unwrap_or(from_address.len());
    from_address[..word]
        .trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '"', '\''])
        .len()
}

//! The look of a sidebar or tab tile: an icon (a letter, digit or emoji, or the name of a line icon) on one of
//! the palette's tile colors (`--space-tile-N`). Spaces, filter lists and views share it.

use crate::error::{AppError, AppResult};

/// The tile colors the palette has.
pub const TILE_COLORS: u8 = 12;

/// The longest line icon name the app's icon set uses, with room to spare.
const ICON_NAME_MAX: usize = 40;

/// An emoji with its modifiers and joiners stays well under this many characters.
const GLYPH_MAX: usize = 8;

/// A line icon's name: lowercase letters, digits and dashes, two characters or more.
fn is_icon_name(icon: &str) -> bool {
    icon.len() >= 2
        && icon.len() <= ICON_NAME_MAX
        && icon
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// One letter or digit, or one emoji (a symbol with its modifiers, variation selectors and joiners).
fn is_glyph(icon: &str) -> bool {
    let mut chars = icon.chars();
    match chars.next() {
        Some(first) if chars.clone().next().is_none() => {
            !first.is_whitespace() && !first.is_control()
        }
        Some(first) => {
            !first.is_alphanumeric()
                && icon.chars().count() <= GLYPH_MAX
                && icon
                    .chars()
                    .all(|c| !c.is_whitespace() && !c.is_control() && !c.is_ascii())
        }
        None => false,
    }
}

/// Trimmed; blank is none.
pub fn clean_icon(icon: Option<String>) -> AppResult<Option<String>> {
    let Some(icon) = icon.map(|i| i.trim().to_string()).filter(|i| !i.is_empty()) else {
        return Ok(None);
    };
    if is_icon_name(&icon) || is_glyph(&icon) {
        return Ok(Some(icon));
    }
    Err(AppError::InvalidField {
        field: "icon".into(),
        message: "Use one letter, digit or emoji, or an icon from the list".into(),
    })
}

/// A tile color number (`TILE_COLORS`); blank is none.
pub fn clean_color(color: Option<String>) -> AppResult<Option<String>> {
    let Some(color) = color
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
    else {
        return Ok(None);
    };
    if color
        .parse::<u8>()
        .is_ok_and(|n| (1..=TILE_COLORS).contains(&n))
    {
        return Ok(Some(color));
    }
    Err(AppError::InvalidField {
        field: "color".into(),
        message: format!("Use a color from 1 to {TILE_COLORS}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(value: &str) -> AppResult<Option<String>> {
        clean_icon(Some(value.into()))
    }

    fn color(value: &str) -> AppResult<Option<String>> {
        clean_color(Some(value.into()))
    }

    fn is_invalid_field(result: AppResult<Option<String>>, expected: &str) -> bool {
        matches!(result, Err(AppError::InvalidField { field, .. }) if field == expected)
    }

    #[test]
    fn a_missing_or_blank_icon_is_none() {
        assert_eq!(clean_icon(None).unwrap(), None);
        for blank in ["", " ", "\t\n"] {
            assert_eq!(icon(blank).unwrap(), None, "{blank:?}");
        }
    }

    #[test]
    fn an_icon_is_trimmed() {
        for (input, expected) in [(" A ", "A"), ("\t🚀\n", "🚀"), ("  rocket ", "rocket")] {
            assert_eq!(icon(input).unwrap().as_deref(), Some(expected), "{input:?}");
        }
    }

    #[test]
    fn letters_digits_emoji_and_line_icon_names_are_accepted() {
        for input in [
            "A",
            "z",
            "7",
            "É",
            "🚀",
            "👩‍💻",
            "👍🏽",
            "rocket",
            "check-circle",
            "a1",
        ] {
            assert_eq!(icon(input).unwrap().as_deref(), Some(input), "{input}");
        }
    }

    #[test]
    fn an_icon_name_may_be_forty_characters_long() {
        let name = "a".repeat(ICON_NAME_MAX);

        assert_eq!(icon(&name).unwrap(), Some(name));
    }

    #[test]
    fn malformed_icons_are_invalid_fields_named_icon() {
        let too_long = "a".repeat(ICON_NAME_MAX + 1);
        for input in [
            "ab c",
            "AB",
            "Ab",
            "ÄÖ",
            "a\u{7}",
            "\u{7}",
            "🚀\u{7}",
            "🚀 🚀",
            "rocket!",
            too_long.as_str(),
        ] {
            assert!(is_invalid_field(icon(input), "icon"), "{input:?}");
        }
    }

    #[test]
    fn a_missing_or_blank_color_is_none() {
        assert_eq!(clean_color(None).unwrap(), None);
        for blank in ["", "  "] {
            assert_eq!(color(blank).unwrap(), None, "{blank:?}");
        }
    }

    #[test]
    fn colors_one_to_twelve_are_accepted_and_trimmed() {
        for number in 1..=TILE_COLORS {
            let text = number.to_string();
            assert_eq!(color(&text).unwrap(), Some(text.clone()), "{text}");
            assert_eq!(
                color(&format!(" {text} ")).unwrap(),
                Some(text.clone()),
                "{text}"
            );
        }
    }

    #[test]
    fn colors_outside_the_palette_are_invalid_fields_named_color() {
        for input in ["0", "13", "x", "-1", "1.5", "256", "red"] {
            assert!(is_invalid_field(color(input), "color"), "{input}");
        }
    }
}

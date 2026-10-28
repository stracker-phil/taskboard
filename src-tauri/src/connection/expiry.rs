//! When to warn that the API token runs out: Atlassian tokens expire after 1 to 365 days.

use chrono::NaiveDate;
use serde::Serialize;
use ts_rs::TS;

pub const WARN_DAYS: i64 = 14;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum TokenExpiry {
    /// No date known, or more than `WARN_DAYS` left.
    Fine,
    /// `days` left; 0 = expires today.
    Soon {
        days: i64,
    },
    Expired,
}

pub fn token_expiry(expires: Option<NaiveDate>, today: NaiveDate) -> TokenExpiry {
    let Some(expires) = expires else {
        return TokenExpiry::Fine;
    };
    match (expires - today).num_days() {
        days if days < 0 => TokenExpiry::Expired,
        days if days <= WARN_DAYS => TokenExpiry::Soon { days },
        _ => TokenExpiry::Fine,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()
    }

    fn expiry_in(days: i64) -> TokenExpiry {
        token_expiry(Some(today() + chrono::Duration::days(days)), today())
    }

    #[test]
    fn a_token_without_a_known_date_is_fine() {
        assert_eq!(token_expiry(None, today()), TokenExpiry::Fine);
    }

    #[test]
    fn the_warning_starts_fourteen_days_before_the_date() {
        for (days, expected) in [
            (365, TokenExpiry::Fine),
            (15, TokenExpiry::Fine),
            (14, TokenExpiry::Soon { days: 14 }),
            (1, TokenExpiry::Soon { days: 1 }),
            (0, TokenExpiry::Soon { days: 0 }),
            (-1, TokenExpiry::Expired),
            (-90, TokenExpiry::Expired),
        ] {
            assert_eq!(expiry_in(days), expected, "{days} days left");
        }
    }

    #[test]
    fn expiry_is_tagged_by_state_for_the_page() {
        for (expiry, expected) in [
            (TokenExpiry::Fine, json!({ "state": "fine" })),
            (
                TokenExpiry::Soon { days: 3 },
                json!({ "state": "soon", "days": 3 }),
            ),
            (TokenExpiry::Expired, json!({ "state": "expired" })),
        ] {
            assert_eq!(
                serde_json::to_value(expiry).unwrap(),
                expected,
                "{expiry:?}"
            );
        }
    }
}

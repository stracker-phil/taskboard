//! Settings → Status's record of the changes sent to Jira (`writes::sent`) and its CSV export. Pure: no I/O.

use super::groups::{DiagnosticGroup, DiagnosticRow, DiagnosticTone, group, row};
use crate::writes::sent::{KEEP_DAYS, SentChange};

/// How many of the newest changes the page lists; Export writes them all.
pub const SHOWN: u32 = 50;

/// One row per change in the order given (newest first), each labelled with its time (`local`) and issue; a
/// refused one in red with Jira's answer.
pub fn sent_group(changes: &[SentChange], local: impl Fn(&str) -> String) -> DiagnosticGroup {
    let kept = format!("Kept for {KEEP_DAYS} days");
    let export = row(&kept, shown_label(changes.len()), DiagnosticTone::Info)
        .offering("app:exportChanges", "Export…");
    let rows = changes.iter().map(|change| {
        let label = format!(
            "{} · {}",
            local(&change.at),
            change.issue_key.as_deref().unwrap_or("Removed issue")
        );
        match &change.error {
            None => row(&label, change.change.clone(), DiagnosticTone::Ok),
            Some(error) => row(
                &label,
                format!("{} · refused: {error}", change.change),
                DiagnosticTone::Error,
            ),
        }
    });
    group(
        "Changes Sent to Jira",
        std::iter::once(export)
            .chain(rows)
            .collect::<Vec<DiagnosticRow>>(),
    )
}

fn shown_label(shown: usize) -> String {
    match shown {
        0 => "None yet".into(),
        n => format!("The newest {n} below"),
    }
}

/// The records as CSV with a header line, oldest first, for a spreadsheet.
pub fn to_csv(changes: &[SentChange]) -> String {
    let mut csv = String::from("Sent at (UTC),Issue,Change,Result\n");
    for change in changes.iter().rev() {
        let result = change
            .error
            .as_deref()
            .map_or("Done".to_string(), |e| format!("Refused: {e}"));
        let fields = [
            change.at.as_str(),
            change.issue_key.as_deref().unwrap_or(""),
            change.change.as_str(),
            result.as_str(),
        ];
        csv.push_str(&fields.map(csv_field).join(","));
        csv.push('\n');
    }
    csv
}

/// Quoted when it holds a comma, a quote or a line break; a leading `=`, `+`, `-` or `@` is defused, so a
/// spreadsheet doesn't run it as a formula.
fn csv_field(text: &str) -> String {
    let text = match text.chars().next() {
        Some('=' | '+' | '-' | '@') => format!("'{text}"),
        _ => text.to_string(),
    };
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(at: &str, key: Option<&str>, change: &str, error: Option<&str>) -> SentChange {
        SentChange {
            at: at.into(),
            issue_key: key.map(Into::into),
            change: change.into(),
            error: error.map(Into::into),
        }
    }

    fn local(at: &str) -> String {
        format!("local {at}")
    }

    fn many(count: usize) -> Vec<SentChange> {
        (0..count)
            .map(|n| sent("t", Some("NW-1"), &format!("Status: Step {n}"), None))
            .collect()
    }

    fn export_row(value: &str) -> DiagnosticRow {
        DiagnosticRow {
            label: "Kept for 180 days".into(),
            value: value.into(),
            tone: DiagnosticTone::Info,
            command: Some("app:exportChanges".into()),
            command_label: Some("Export…".into()),
            copy_text: None,
        }
    }

    #[test]
    fn without_changes_the_group_only_says_none_yet_and_offers_the_export() {
        let group = sent_group(&[], local);

        assert_eq!(group.title, "Changes Sent to Jira");
        assert_eq!(group.rows, [export_row("None yet")]);
    }

    #[test]
    fn the_first_row_names_how_many_of_the_newest_changes_are_below() {
        for (count, value) in [
            (1, "The newest 1 below"),
            (2, "The newest 2 below"),
            (50, "The newest 50 below"),
            (51, "The newest 51 below"),
        ] {
            let group = sent_group(&many(count), local);

            assert_eq!(group.rows[0], export_row(value), "{count}");
            assert_eq!(group.rows.len(), count + 1, "{count}");
        }
    }

    #[test]
    fn a_taken_change_is_an_ok_row_labelled_with_its_local_time_and_issue() {
        let group = sent_group(
            &[sent(
                "2026-10-28T11:00:00.000Z",
                Some("NW-1042"),
                "Assignee: Jamie Doe",
                None,
            )],
            local,
        );

        assert_eq!(
            group.rows[1],
            DiagnosticRow {
                label: "local 2026-10-28T11:00:00.000Z · NW-1042".into(),
                value: "Assignee: Jamie Doe".into(),
                tone: DiagnosticTone::Ok,
                command: None,
                command_label: None,
                copy_text: None,
            }
        );
    }

    #[test]
    fn a_refused_change_is_an_error_row_with_jiras_answer() {
        let group = sent_group(
            &[sent(
                "t",
                Some("NW-1042"),
                "Priority: Low",
                Some("Not allowed"),
            )],
            local,
        );

        assert_eq!(
            group.rows[1],
            DiagnosticRow {
                label: "local t · NW-1042".into(),
                value: "Priority: Low · refused: Not allowed".into(),
                tone: DiagnosticTone::Error,
                command: None,
                command_label: None,
                copy_text: None,
            }
        );
    }

    #[test]
    fn a_change_of_an_issue_that_left_the_cache_is_labelled_removed_issue() {
        let group = sent_group(&[sent("t", None, "Comment (12 characters)", None)], local);

        assert_eq!(group.rows[1].label, "local t · Removed issue");
    }

    #[test]
    fn the_rows_keep_the_order_of_the_changes_given() {
        let changes = [
            sent("b", Some("NW-2"), "Status: Done", None),
            sent("a", Some("NW-1"), "Status: To Do", None),
        ];

        let group = sent_group(&changes, local);

        let labels: Vec<&str> = group.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(
            labels,
            ["Kept for 180 days", "local b · NW-2", "local a · NW-1"]
        );
    }

    #[test]
    fn no_changes_give_only_the_csv_header() {
        assert_eq!(to_csv(&[]), "Sent at (UTC),Issue,Change,Result\n");
    }

    #[test]
    fn the_csv_lists_the_oldest_first_with_done_or_the_refusal_as_result() {
        let newest_first = [
            sent(
                "2026-10-28T12:00:00.000Z",
                None,
                "Priority: Low",
                Some("Not allowed"),
            ),
            sent(
                "2026-10-28T11:00:00.000Z",
                Some("NW-1"),
                "Status: Done",
                None,
            ),
        ];

        assert_eq!(
            to_csv(&newest_first),
            "Sent at (UTC),Issue,Change,Result\n\
             2026-10-28T11:00:00.000Z,NW-1,Status: Done,Done\n\
             2026-10-28T12:00:00.000Z,,Priority: Low,Refused: Not allowed\n"
        );
    }

    #[test]
    fn a_csv_field_with_a_comma_a_quote_or_a_line_break_is_quoted_with_quotes_doubled() {
        for (text, field) in [
            (
                "Web link: https://example.com/a,b",
                "\"Web link: https://example.com/a,b\"",
            ),
            (
                "Assignee: Jamie \"JD\" Doe",
                "\"Assignee: Jamie \"\"JD\"\" Doe\"",
            ),
            ("two\nlines", "\"two\nlines\""),
            ("one\rline", "\"one\rline\""),
            ("plain", "plain"),
        ] {
            let csv = to_csv(&[sent("t", Some("NW-1"), text, None)]);

            assert_eq!(
                csv,
                format!("Sent at (UTC),Issue,Change,Result\nt,NW-1,{field},Done\n"),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_csv_field_that_starts_like_a_formula_gets_a_leading_apostrophe() {
        for (text, field) in [
            ("=SUM(A1)", "'=SUM(A1)"),
            ("+1", "'+1"),
            ("-1", "'-1"),
            ("@jamie", "'@jamie"),
            ("a=b", "a=b"),
            ("=1,2", "\"'=1,2\""),
        ] {
            let csv = to_csv(&[sent("t", Some("NW-1"), text, None)]);

            assert_eq!(
                csv,
                format!("Sent at (UTC),Issue,Change,Result\nt,NW-1,{field},Done\n"),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_refusal_that_starts_like_a_formula_needs_no_apostrophe_because_it_starts_with_refused() {
        let csv = to_csv(&[sent("t", Some("NW-1"), "Status: Done", Some("=bad"))]);

        assert_eq!(
            csv,
            "Sent at (UTC),Issue,Change,Result\nt,NW-1,Status: Done,Refused: =bad\n"
        );
    }
}

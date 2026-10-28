//! Just enough JQL for the queries Taskboard writes: `AND`/`OR`/parentheses over `project`, `id`,
//! `statusCategory` and `updated` clauses, and a trailing `ORDER BY`. A query it can't read matches every
//! issue.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::Value;

use crate::jira::dto::parse_jira_time;

#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    All,
    Or(Box<Query>, Box<Query>),
    And(Box<Query>, Box<Query>),
    Clause {
        field: String,
        op: String,
        values: Vec<String>,
    },
}

/// What a clause compares against: the fake's clock and the user's time zone for date literals.
pub struct Context {
    pub now: DateTime<Utc>,
    pub zone: Tz,
}

pub fn parse(jql: &str) -> Option<Query> {
    let lower = jql.to_ascii_lowercase();
    let body = match lower.find(" order by ") {
        Some(at) => &jql[..at],
        None if lower.trim_start().starts_with("order by") => "",
        None => jql,
    };
    let tokens = tokenize(body)?;
    if tokens.is_empty() {
        return Some(Query::All);
    }
    let mut parser = Parser { tokens, at: 0 };
    let query = parser.or()?;
    (parser.at == parser.tokens.len()).then_some(query)
}

/// `parse`, or every issue when the query isn't understood.
pub fn parse_or_all(jql: &str) -> Query {
    parse(jql).unwrap_or(Query::All)
}

pub fn matches(query: &Query, issue: &Value, context: &Context) -> bool {
    match query {
        Query::All => true,
        Query::Or(a, b) => matches(a, issue, context) || matches(b, issue, context),
        Query::And(a, b) => matches(a, issue, context) && matches(b, issue, context),
        Query::Clause { field, op, values } => clause(field, op, values, issue, context),
    }
}

fn clause(field: &str, op: &str, values: &[String], issue: &Value, context: &Context) -> bool {
    let fields = &issue["fields"];
    let actual: Vec<String> = match field {
        "project" => vec![
            str_of(&fields["project"]["key"]),
            str_of(&fields["project"]["id"]),
        ],
        "id" | "issue" | "key" => vec![str_of(&issue["id"]), str_of(&issue["key"])],
        "statuscategory" => {
            let category = &fields["status"]["statusCategory"];
            vec![str_of(&category["name"]), str_of(&category["key"])]
        }
        "updated" => return updated_clause(op, values, fields, context),
        _ => return true,
    };
    let hit = values
        .iter()
        .any(|v| actual.iter().any(|a| a.eq_ignore_ascii_case(v)));
    match op {
        "=" | "in" => hit,
        "!=" | "not in" => !hit,
        _ => true,
    }
}

fn updated_clause(op: &str, values: &[String], fields: &Value, context: &Context) -> bool {
    let Some(updated) = fields["updated"].as_str().and_then(parse_jira_time) else {
        return false;
    };
    let Some(bound) = values.first().and_then(|v| instant(v, context)) else {
        return true;
    };
    let updated = updated.with_timezone(&Utc);
    match op {
        ">=" => updated >= bound,
        ">" => updated > bound,
        "<=" => updated <= bound,
        "<" => updated < bound,
        _ => true,
    }
}

/// `-90d`, `-15m` or `yyyy-MM-dd HH:mm` in the user's zone.
fn instant(value: &str, context: &Context) -> Option<DateTime<Utc>> {
    if let Some(relative) = value.strip_prefix('-') {
        let (number, unit) = relative.split_at(relative.len().checked_sub(1)?);
        let n: i64 = number.parse().ok()?;
        let span = match unit {
            "d" => chrono::Duration::days(n),
            "h" => chrono::Duration::hours(n),
            "m" => chrono::Duration::minutes(n),
            "w" => chrono::Duration::weeks(n),
            _ => return None,
        };
        return Some(context.now - span);
    }
    let local = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").ok()?;
    let at = context.zone.from_local_datetime(&local).earliest()?;
    Some(at.with_timezone(&Utc))
}

fn str_of(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Open,
    Close,
    Comma,
    Word(String),
    Quoted(String),
}

fn tokenize(text: &str) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            '(' | ')' | ',' => {
                chars.next();
                tokens.push(match c {
                    '(' => Token::Open,
                    ')' => Token::Close,
                    _ => Token::Comma,
                });
            }
            '"' | '\'' => {
                chars.next();
                let mut quoted = String::new();
                loop {
                    match chars.next()? {
                        '\\' => quoted.push(chars.next()?),
                        q if q == c => break,
                        other => quoted.push(other),
                    }
                }
                tokens.push(Token::Quoted(quoted));
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || "(),\"'".contains(c) {
                        break;
                    }
                    word.push(c);
                    chars.next();
                }
                tokens.push(Token::Word(word));
            }
        }
    }
    Some(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

impl Parser {
    fn peek_word(&self) -> Option<String> {
        match self.tokens.get(self.at) {
            Some(Token::Word(w)) => Some(w.to_ascii_lowercase()),
            _ => None,
        }
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        self.at += 1;
        token
    }

    fn or(&mut self) -> Option<Query> {
        let mut query = self.and()?;
        while self.peek_word().as_deref() == Some("or") {
            self.at += 1;
            query = Query::Or(Box::new(query), Box::new(self.and()?));
        }
        Some(query)
    }

    fn and(&mut self) -> Option<Query> {
        let mut query = self.factor()?;
        while self.peek_word().as_deref() == Some("and") {
            self.at += 1;
            query = Query::And(Box::new(query), Box::new(self.factor()?));
        }
        Some(query)
    }

    fn factor(&mut self) -> Option<Query> {
        if self.tokens.get(self.at) == Some(&Token::Open) {
            self.at += 1;
            let inner = self.or()?;
            return (self.next()? == Token::Close).then_some(inner);
        }
        let Token::Word(field) = self.next()? else {
            return None;
        };
        let mut op = match self.next()? {
            Token::Word(op) => op.to_ascii_lowercase(),
            _ => return None,
        };
        if op == "not" && self.peek_word().as_deref() == Some("in") {
            self.at += 1;
            op = "not in".into();
        }
        let values = if op.ends_with("in") {
            self.list()?
        } else {
            vec![self.value()?]
        };
        Some(Query::Clause {
            field: field.to_ascii_lowercase(),
            op,
            values,
        })
    }

    fn list(&mut self) -> Option<Vec<String>> {
        if self.next()? != Token::Open {
            return None;
        }
        let mut values = Vec::new();
        loop {
            values.push(self.value()?);
            match self.next()? {
                Token::Comma => continue,
                Token::Close => return Some(values),
                _ => return None,
            }
        }
    }

    fn value(&mut self) -> Option<String> {
        match self.next()? {
            Token::Word(w) | Token::Quoted(w) => Some(w),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono_tz::Europe::Vienna;
    use serde_json::json;

    use super::*;
    use crate::jira::jql::{and_project, and_updated_since, default_scope, order_by_updated};

    fn c(field: &str, op: &str, values: &[&str]) -> Query {
        Query::Clause {
            field: field.into(),
            op: op.into(),
            values: values.iter().map(|v| v.to_string()).collect(),
        }
    }

    fn and(a: Query, b: Query) -> Query {
        Query::And(Box::new(a), Box::new(b))
    }

    fn or(a: Query, b: Query) -> Query {
        Query::Or(Box::new(a), Box::new(b))
    }

    fn scope_query() -> Query {
        and(
            c("project", "in", &["NW", "LUM"]),
            or(
                c("statuscategory", "!=", &["Done"]),
                c("updated", ">=", &["-90d"]),
            ),
        )
    }

    fn context_at(now: &str) -> Context {
        Context {
            now: now.parse().unwrap(),
            zone: Vienna,
        }
    }

    fn demo_context() -> Context {
        context_at("2026-10-28T11:00:00Z")
    }

    fn issue(
        key: &str,
        id: &str,
        project: (&str, &str),
        category: (&str, &str),
        updated: &str,
    ) -> Value {
        json!({
            "id": id,
            "key": key,
            "fields": {
                "project": { "id": project.1, "key": project.0 },
                "status": { "statusCategory": { "name": category.0, "key": category.1 } },
                "updated": updated,
            }
        })
    }

    fn open_nw() -> Value {
        issue(
            "NW-1",
            "101",
            ("NW", "10001"),
            ("In Progress", "indeterminate"),
            "2026-10-28T10:00:00.000+0000",
        )
    }

    fn selects(jql: &str, issue: &Value, context: &Context) -> bool {
        let query = parse(jql).unwrap_or_else(|| panic!("{jql} should parse"));
        matches(&query, issue, context)
    }

    #[test]
    fn the_default_scope_parses_into_projects_and_the_open_or_recent_rule() {
        let scope = default_scope(&["NW".into(), "LUM".into()], 90).unwrap();
        assert_eq!(parse(&scope), Some(scope_query()));
    }

    #[test]
    fn clauses_added_to_the_scope_parse_as_further_and_terms() {
        let scope = default_scope(&["NW".into(), "LUM".into()], 90).unwrap();
        assert_eq!(
            parse(&and_project(&scope, "NW")),
            Some(and(scope_query(), c("project", "=", &["NW"])))
        );
        assert_eq!(
            parse(&and_updated_since(&scope, "2026-10-28 10:05")),
            Some(and(
                scope_query(),
                c("updated", ">=", &["2026-10-28 10:05"])
            ))
        );
    }

    #[test]
    fn a_trailing_order_by_is_ignored() {
        let scope = default_scope(&["NW".into()], 90).unwrap();
        assert_eq!(parse(&order_by_updated(&scope)), parse(&scope));
        assert_eq!(
            parse("project = NW order by updated desc"),
            Some(c("project", "=", &["NW"]))
        );
        assert_eq!(parse("ORDER BY updated ASC"), Some(Query::All));
    }

    #[test]
    fn an_empty_query_selects_everything() {
        assert_eq!(parse(""), Some(Query::All));
        assert_eq!(parse("   "), Some(Query::All));
    }

    #[test]
    fn id_lists_and_negated_lists_parse_with_all_their_values() {
        for (jql, expected) in [
            ("id in (1, 2)", c("id", "in", &["1", "2"])),
            ("ID IN (1,2,3)", c("id", "in", &["1", "2", "3"])),
            (
                r#"project not in ("ARC", "INT")"#,
                c("project", "not in", &["ARC", "INT"]),
            ),
            (
                "statusCategory != Done",
                c("statuscategory", "!=", &["Done"]),
            ),
            (r#"project = 'NW'"#, c("project", "=", &["NW"])),
        ] {
            assert_eq!(parse(jql), Some(expected), "{jql}");
        }
    }

    #[test]
    fn and_binds_tighter_than_or_and_parentheses_override_it() {
        let a = || c("project", "=", &["A"]);
        let b = || c("project", "=", &["B"]);
        let d = || c("project", "=", &["C"]);
        assert_eq!(
            parse("project = A OR project = B AND project = C"),
            Some(or(a(), and(b(), d())))
        );
        assert_eq!(
            parse("(project = A OR project = B) and project = C"),
            Some(and(or(a(), b()), d()))
        );
        assert_eq!(
            parse("project = A AND project = B AND project = C"),
            Some(and(and(a(), b()), d()))
        );
    }

    #[test]
    fn syntax_the_reader_does_not_know_is_not_parsed_and_selects_everything() {
        for jql in [
            "project =",
            "project = (",
            "(project = A",
            "project = A)",
            "project = A AND",
            "project in (A",
            "project in A",
            "project = A project = B",
            "project = \"A",
            "foo",
        ] {
            assert_eq!(parse(jql), None, "{jql}");
            assert_eq!(parse_or_all(jql), Query::All, "{jql}");
        }
    }

    #[test]
    fn a_query_that_parses_is_returned_by_parse_or_all() {
        assert_eq!(parse_or_all("project = NW"), c("project", "=", &["NW"]));
    }

    #[test]
    fn project_clauses_match_by_key_or_id_ignoring_case() {
        let context = demo_context();
        let issue = open_nw();
        for (jql, expected) in [
            (r#"project = "NW""#, true),
            ("project = nw", true),
            ("project = 10001", true),
            (r#"project = "LUM""#, false),
            (r#"project in ("LUM", "NW")"#, true),
            (r#"project in ("LUM", "FBA")"#, false),
            (r#"project not in ("NW")"#, false),
            (r#"project not in ("ARC")"#, true),
            (r#"project != "NW""#, false),
        ] {
            assert_eq!(selects(jql, &issue, &context), expected, "{jql}");
        }
    }

    #[test]
    fn id_and_key_clauses_match_the_issue_id_or_key() {
        let context = demo_context();
        let issue = open_nw();
        for (jql, expected) in [
            ("id in (101, 102)", true),
            ("id = 103", false),
            (r#"key = "NW-1""#, true),
            (r#"issue in ("NW-2", "NW-1")"#, true),
            (r#"id not in (101)"#, false),
        ] {
            assert_eq!(selects(jql, &issue, &context), expected, "{jql}");
        }
    }

    #[test]
    fn status_category_clauses_match_by_name_or_key() {
        let context = demo_context();
        let todo = issue(
            "NW-2",
            "102",
            ("NW", "10001"),
            ("To Do", "new"),
            "2026-10-28T10:00:00.000+0000",
        );
        let done = issue(
            "NW-3",
            "103",
            ("NW", "10001"),
            ("Done", "done"),
            "2026-10-28T10:00:00.000+0000",
        );
        for (jql, on_todo, on_done) in [
            ("statusCategory != Done", true, false),
            ("statusCategory = Done", false, true),
            (r#"statusCategory = "To Do""#, true, false),
            ("statusCategory = new", true, false),
            ("statusCategory in (done, new)", true, true),
        ] {
            assert_eq!(
                selects(jql, &todo, &context),
                on_todo,
                "{jql} on a to-do issue"
            );
            assert_eq!(
                selects(jql, &done, &context),
                on_done,
                "{jql} on a done issue"
            );
        }
    }

    #[test]
    fn a_relative_updated_window_is_measured_from_the_contexts_now() {
        let issue = issue(
            "NW-4",
            "104",
            ("NW", "10001"),
            ("Done", "done"),
            "2026-07-30T13:00:00.000+0200",
        );
        for (now, expected) in [
            ("2026-10-28T11:00:00Z", true),
            ("2026-10-28T11:00:01Z", false),
            ("2026-10-27T11:00:00Z", true),
        ] {
            assert_eq!(
                selects("updated >= -90d", &issue, &context_at(now)),
                expected,
                "now {now}"
            );
        }
    }

    #[test]
    fn relative_units_cover_weeks_days_hours_and_minutes() {
        let context = demo_context();
        let issue = issue(
            "NW-5",
            "105",
            ("NW", "10001"),
            ("Done", "done"),
            "2026-10-28T09:30:00.000+0000",
        );
        for (jql, expected) in [
            ("updated >= -2h", true),
            ("updated >= -90m", true),
            ("updated >= -89m", false),
            ("updated >= -1w", true),
            ("updated < -1h", true),
            ("updated < -2h", false),
        ] {
            assert_eq!(selects(jql, &issue, &context), expected, "{jql}");
        }
    }

    #[test]
    fn a_date_literal_is_read_in_the_users_time_zone() {
        let context = demo_context();
        for (updated, jql, expected) in [
            (
                "2026-10-28T11:00:00.000+0000",
                r#"updated >= "2026-10-28 12:00""#,
                true,
            ),
            (
                "2026-10-28T10:59:59.000+0000",
                r#"updated >= "2026-10-28 12:00""#,
                false,
            ),
            (
                "2026-07-15T10:00:00.000+0000",
                r#"updated >= "2026-07-15 12:00""#,
                true,
            ),
            (
                "2026-07-15T09:59:00.000+0000",
                r#"updated >= "2026-07-15 12:00""#,
                false,
            ),
            (
                "2026-10-28T12:00:00.000+0100",
                r#"updated >= "2026-10-28 12:00""#,
                true,
            ),
            (
                "2026-10-28T11:00:00.000+0000",
                r#"updated > "2026-10-28 12:00""#,
                false,
            ),
            (
                "2026-10-28T11:00:00.000+0000",
                r#"updated <= "2026-10-28 12:00""#,
                true,
            ),
            (
                "2026-10-28T11:00:00.000+0000",
                r#"updated < "2026-10-28 12:00""#,
                false,
            ),
        ] {
            let issue = issue("NW-6", "106", ("NW", "10001"), ("Done", "done"), updated);
            assert_eq!(selects(jql, &issue, &context), expected, "{updated} {jql}");
        }
    }

    #[test]
    fn an_issue_without_a_readable_updated_time_fails_updated_clauses() {
        let context = demo_context();
        let mut broken = open_nw();
        broken["fields"]["updated"] = json!("yesterday");
        assert!(!selects("updated >= -90d", &broken, &context));
        assert!(selects("project = NW", &broken, &context));
    }

    #[test]
    fn the_default_scope_selects_open_work_and_recently_finished_work() {
        let context = demo_context();
        let scope = default_scope(&["NW".into()], 90).unwrap();
        let open_old = issue(
            "NW-7",
            "107",
            ("NW", "10001"),
            ("To Do", "new"),
            "2025-01-10T09:00:00.000+0100",
        );
        let done_recent = issue(
            "NW-8",
            "108",
            ("NW", "10001"),
            ("Done", "done"),
            "2026-10-20T09:00:00.000+0200",
        );
        let done_old = issue(
            "NW-9",
            "109",
            ("NW", "10001"),
            ("Done", "done"),
            "2026-01-10T09:00:00.000+0100",
        );
        let other_project = issue(
            "LUM-1",
            "301",
            ("LUM", "10003"),
            ("To Do", "new"),
            "2026-10-20T09:00:00.000+0200",
        );
        for (issue, expected) in [
            (&open_old, true),
            (&done_recent, true),
            (&done_old, false),
            (&other_project, false),
        ] {
            assert_eq!(
                selects(&scope, issue, &context),
                expected,
                "{}",
                issue["key"]
            );
        }
    }

    #[test]
    fn a_clause_on_an_unknown_field_selects_every_issue() {
        let context = demo_context();
        assert!(selects("foo = bar", &open_nw(), &context));
        assert!(selects("project = LUM OR foo = bar", &open_nw(), &context));
        assert!(!selects(
            "project = LUM AND project = FBA",
            &open_nw(),
            &context
        ));
    }

    #[test]
    fn the_all_query_selects_every_issue() {
        assert!(matches(&Query::All, &open_nw(), &demo_context()));
    }
}

//! Me mode: which issues are mine. Assigned to me always; the rules switched on add more (ADR 0045).

pub mod commands;

#[cfg(test)]
mod tests;

use rusqlite::Connection;
use rusqlite::types::Value as SqlValue;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;
use crate::filter::{Condition, Ctx, Field, ME, compile};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MeMatch {
    /// A user field holds me: Reporter or a custom user field, by its filter name.
    Field { field: String },
    /// A comment @-mentions me.
    Mentioned,
    /// A subtask of an issue that is mine by the other rules.
    Subtasks,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct MeRule {
    pub rule: MeMatch,
    pub on: bool,
}

/// What the Me popup offers before the owner adds fields: Reporter, mentions and subtasks, all off.
pub fn default_rules() -> Vec<MeRule> {
    [
        MeMatch::Field {
            field: Field::Reporter.name().into(),
        },
        MeMatch::Mentioned,
        MeMatch::Subtasks,
    ]
    .into_iter()
    .map(|rule| MeRule { rule, on: false })
    .collect()
}

/// The issues Me mode keeps, as SQL over `cache.issue i`; without a connected account, none. A field rule for a
/// field the cache doesn't know matches nothing; one for the assignee adds nothing to the base clause.
pub fn condition(rules: &[MeRule], ctx: &Ctx) -> AppResult<(String, Vec<SqlValue>)> {
    let Some(me) = &ctx.me else {
        return Ok(("0".into(), Vec::new()));
    };
    let mut parts = vec!["i.assignee_id = ?".to_string()];
    let mut params: Vec<SqlValue> = vec![me.clone().into()];
    let mut subtasks = false;
    for rule in rules.iter().filter(|r| r.on) {
        match &rule.rule {
            MeMatch::Field { field } => {
                let field = Field::from(field.clone());
                let unknown = matches!(&field, Field::Custom(id) if !ctx.custom.contains_key(id));
                if field == Field::Assignee || unknown {
                    continue;
                }
                let compiled = compile(&[Condition::is(field, ME)], ctx)?;
                parts.push(compiled.condition);
                params.extend(compiled.params);
            }
            MeMatch::Mentioned => {
                parts.push(MENTIONED.into());
                params.extend([me.clone().into(), me.clone().into()]);
            }
            MeMatch::Subtasks => subtasks = true,
        }
    }
    let mine = parts
        .iter()
        .map(|part| format!("({part})"))
        .collect::<Vec<_>>()
        .join(" OR ");
    Ok(match subtasks {
        true => with_subtasks(&mine, params),
        false => (format!("({mine})"), params),
    })
}

/// Also the subtasks whose parent is mine. The parent query aliases its table `i` too, because the rule clauses
/// are written for that alias.
fn with_subtasks(mine: &str, params: Vec<SqlValue>) -> (String, Vec<SqlValue>) {
    (
        format!(
            "(({mine}) OR (i.type_is_subtask = 1 AND i.parent_id IN (SELECT i.id FROM cache.issue i WHERE {mine})))"
        ),
        [params.clone(), params].concat(),
    )
}

/// One edit of the Me rules, from the Me popover or Settings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MeRuleEdit {
    Switch {
        rule: MeMatch,
        on: bool,
    },
    /// Offers a user field, switched on; one already offered stays as it is.
    AddField {
        field: String,
    },
    RemoveField {
        field: String,
    },
}

/// The rules after the edit; the built-in rules can't be removed.
pub fn edited(mut rules: Vec<MeRule>, edit: MeRuleEdit) -> Vec<MeRule> {
    match edit {
        MeRuleEdit::Switch { rule, on } => {
            for r in rules.iter_mut().filter(|r| r.rule == rule) {
                r.on = on;
            }
        }
        MeRuleEdit::AddField { field } => {
            let rule = MeMatch::Field { field };
            if !rules.iter().any(|r| r.rule == rule) {
                rules.push(MeRule { rule, on: true });
            }
        }
        MeRuleEdit::RemoveField { field } => rules.retain(|r| {
            r.rule
                != MeMatch::Field {
                    field: field.clone(),
                }
        }),
    }
    rules
}

/// A cached comment's body mentions the account; `instr` skips most comments before the ADF is walked.
const MENTIONED: &str = "i.id IN (SELECT c.issue_id FROM cache.comment c
    WHERE instr(c.body_adf, ?) > 0 AND EXISTS (
        SELECT 1 FROM json_tree(c.body_adf) t
        WHERE t.type = 'object' AND json_extract(t.value, '$.type') = 'mention'
          AND json_extract(t.value, '$.attrs.id') = ?))";

/// A user field the Me popup may offer, for Settings' "Add Field…".
#[derive(Serialize, Clone, Debug, PartialEq, Eq, TS)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct UserField {
    /// The filter name: `reporter` or a custom field's id.
    pub field: String,
    pub name: String,
}

/// Reporter, then the custom user fields the cache knows, by name.
pub fn user_fields(conn: &Connection) -> AppResult<Vec<UserField>> {
    let mut fields = vec![UserField {
        field: Field::Reporter.name().into(),
        name: "Reporter".into(),
    }];
    let mut query = conn.prepare(
        "SELECT id, name FROM cache.field
         WHERE id LIKE 'customfield\\_%' ESCAPE '\\' AND type = 'user'
         ORDER BY name COLLATE NOCASE, id",
    )?;
    let custom = query.query_map([], |row| {
        Ok(UserField {
            field: row.get(0)?,
            name: row.get(1)?,
        })
    })?;
    fields.extend(custom.collect::<Result<Vec<_>, _>>()?);
    Ok(fields)
}

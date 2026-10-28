//! The app's error type and its JSON shape across the Tauri boundary (`{ kind, message, field? }`).

use serde::{Deserialize, Serialize, Serializer};
use ts_rs::TS;

pub type AppResult<T> = Result<T, AppError>;

/// Every error that crosses the Tauri boundary. Serialized as `{ kind, message, field? }` so the UI can
/// branch on `kind`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AppError {
    #[error("File error: {0}")]
    Io(String),
    #[error("{0}")]
    InvalidInput(String),
    /// Invalid input in one form field; `field` is its path (`connection.site`).
    #[error("`{field}`: {message}")]
    InvalidField { field: String, message: String },
    #[error("{0}")]
    NotFound(String),
    /// Needs a Jira connection that isn't set up yet.
    #[error("{0}")]
    NotConnected(String),
    /// A bug or a broken invariant; the message is for the log and a bug report.
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            AppError::Io(_) => ErrorKind::Io,
            AppError::InvalidInput(_) | AppError::InvalidField { .. } => ErrorKind::InvalidInput,
            AppError::NotFound(_) => ErrorKind::NotFound,
            AppError::NotConnected(_) => ErrorKind::NotConnected,
            AppError::Internal(_) => ErrorKind::Internal,
        }
    }

    pub fn field(&self) -> Option<FieldJson<'_>> {
        match self {
            AppError::InvalidField { field, message } => Some(FieldJson {
                path: field,
                message,
            }),
            _ => None,
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ErrorJson {
            kind: self.kind(),
            message: self.to_string(),
            field: self.field(),
        }
        .serialize(serializer)
    }
}

/// The JSON shape of an `AppError`.
#[derive(Serialize, TS)]
#[ts(export, export_to = "src/api/types.gen.ts", rename = "AppError")]
struct ErrorJson<'a> {
    kind: ErrorKind,
    message: String,
    /// The form field the error is about (`InvalidField` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    field: Option<FieldJson<'a>>,
}

/// `message` is the error's message without the `` `path`: `` prefix.
#[derive(Serialize, TS, Debug, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts", rename = "AppErrorField")]
pub struct FieldJson<'a> {
    pub path: &'a str,
    pub message: &'a str,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Io,
    InvalidInput,
    NotFound,
    NotConnected,
    Internal,
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::InvalidInput(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_kind_and_message() {
        assert_eq!(
            serde_json::to_value(AppError::NotFound("Issue NW-3".into())).unwrap(),
            json!({ "kind": "not_found", "message": "Issue NW-3" })
        );
    }

    #[test]
    fn invalid_field_serializes_its_path_and_message_without_prefix() {
        let err = AppError::InvalidField {
            field: "connection.site".into(),
            message: "must be a host name".into(),
        };
        assert_eq!(
            serde_json::to_value(&err).unwrap(),
            json!({
                "kind": "invalid_input",
                "message": "`connection.site`: must be a host name",
                "field": { "path": "connection.site", "message": "must be a host name" }
            })
        );
    }

    #[test]
    fn field_is_absent_for_other_errors() {
        assert_eq!(
            serde_json::to_value(AppError::Internal("broken".into())).unwrap(),
            json!({ "kind": "internal", "message": "broken" })
        );
    }

    #[test]
    fn every_variant_has_its_kind() {
        let cases = [
            (AppError::Io(String::new()), ErrorKind::Io),
            (
                AppError::InvalidInput(String::new()),
                ErrorKind::InvalidInput,
            ),
            (
                AppError::InvalidField {
                    field: "a".into(),
                    message: "b".into(),
                },
                ErrorKind::InvalidInput,
            ),
            (AppError::NotFound(String::new()), ErrorKind::NotFound),
            (
                AppError::NotConnected(String::new()),
                ErrorKind::NotConnected,
            ),
            (AppError::Internal(String::new()), ErrorKind::Internal),
        ];
        for (err, kind) in cases {
            assert_eq!(err.kind(), kind, "{err:?}");
        }
    }

    #[test]
    fn not_connected_errors_serialize_with_the_snake_case_kind() {
        assert_eq!(
            serde_json::to_value(AppError::NotConnected("Connect to Jira first".into())).unwrap(),
            json!({ "kind": "not_connected", "message": "Connect to Jira first" })
        );
    }

    #[test]
    fn io_errors_convert_to_io_with_a_file_error_message() {
        let err = AppError::from(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        assert_eq!(err, AppError::Io("denied".into()));
        assert_eq!(err.to_string(), "File error: denied");
    }

    #[test]
    fn json_errors_convert_to_invalid_input() {
        let parse_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let message = parse_error.to_string();
        let err = AppError::from(parse_error);
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
        assert_eq!(err.to_string(), message);
    }
}

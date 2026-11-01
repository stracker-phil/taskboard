use std::io::{self, BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::Path;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;
use crate::error::{AppError, ErrorKind};
use crate::test_support::temp_config_dir;
use crate::views::Layout;
use tokio::sync::mpsc;

fn serve_into_channel(socket: &Path) -> mpsc::UnboundedReceiver<IpcMessage> {
    let (tx, rx) = mpsc::unbounded_channel();
    let listener = bind(socket).unwrap();
    tokio::spawn(serve(listener, move |m| {
        let _ = tx.send(m);
        async { Ok(serde_json::Value::Null) }
    }));
    rx
}

fn serve_answering(socket: &Path) {
    let listener = bind(socket).unwrap();
    tokio::spawn(serve(listener, |m| async move {
        match m {
            IpcMessage::Error { message } => Err(IpcError::new(
                ErrorKind::InvalidInput,
                format!("failed: {message}"),
            )),
            other => Ok(serde_json::json!({ "handled": other.kind() })),
        }
    }));
}

async fn ask(socket: &Path, message: IpcMessage) -> io::Result<Option<IpcReply>> {
    let socket = socket.to_path_buf();
    tokio::task::spawn_blocking(move || request(&socket, message))
        .await
        .unwrap()
}

async fn forward(socket: &Path, message: IpcMessage) -> Forward {
    let socket = socket.to_path_buf();
    tokio::task::spawn_blocking(move || try_forward(&socket, &message))
        .await
        .unwrap()
        .unwrap()
}

/// Sends raw bytes and returns whatever the server answers before closing.
async fn raw_exchange(socket: &Path, bytes: &[u8]) -> String {
    let mut stream = tokio::net::UnixStream::connect(socket).await.unwrap();
    let _ = stream.write_all(bytes).await;
    let mut reply = String::new();
    let _ = stream.read_to_string(&mut reply).await;
    reply
}

fn activate() -> IpcMessage {
    IpcMessage::Activate { hidden: false }
}

#[tokio::test]
async fn successful_request_is_answered_with_the_handler_result_under_its_id() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    serve_answering(&socket);

    let reply = ask(&socket, activate()).await.unwrap().unwrap();

    assert!(reply.ok);
    assert_eq!(reply.result, serde_json::json!({ "handled": "Activate" }));
    assert_eq!(reply.error, None);
    assert!(!reply.id.is_empty());
}

#[tokio::test]
async fn open_request_reaches_the_handler_with_its_key() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    serve_answering(&socket);

    let reply = ask(
        &socket,
        IpcMessage::Open {
            key: "NW-12".into(),
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(
        reply.into_result(),
        Ok(serde_json::json!({ "handled": "Open" }))
    );
}

#[tokio::test]
async fn failing_request_is_answered_with_the_handler_error() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    serve_answering(&socket);

    let reply = ask(
        &socket,
        IpcMessage::Error {
            message: "x".into(),
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert!(!reply.ok);
    assert_eq!(reply.result, serde_json::Value::Null);
    let expected = IpcError::new(ErrorKind::InvalidInput, "failed: x");
    assert_eq!(reply.error, Some(expected.clone()));
    assert_eq!(reply.into_result(), Err(expected));
}

#[test]
fn messages_have_their_wire_shape() {
    let mut cases = vec![
        (
            IpcMessage::Activate { hidden: true },
            serde_json::json!({ "type": "activate", "hidden": true }),
        ),
        (
            IpcMessage::Error {
                message: "boom".into(),
            },
            serde_json::json!({ "type": "error", "message": "boom" }),
        ),
        (
            IpcMessage::Show { focus: true },
            serde_json::json!({ "type": "show", "focus": true }),
        ),
        (IpcMessage::Hide, serde_json::json!({ "type": "hide" })),
        (IpcMessage::Quit, serde_json::json!({ "type": "quit" })),
        (IpcMessage::Status, serde_json::json!({ "type": "status" })),
        (
            IpcMessage::Open {
                key: "NW-12".into(),
            },
            serde_json::json!({ "type": "open", "key": "NW-12" }),
        ),
        (
            IpcMessage::Sync { full: true },
            serde_json::json!({ "type": "sync", "full": true }),
        ),
        (
            IpcMessage::Sync { full: false },
            serde_json::json!({ "type": "sync", "full": false }),
        ),
        (IpcMessage::Spaces, serde_json::json!({ "type": "spaces" })),
        (
            IpcMessage::Filters { space: None },
            serde_json::json!({ "type": "filters", "space": null }),
        ),
        (
            IpcMessage::Filters {
                space: Some("NW".into()),
            },
            serde_json::json!({ "type": "filters", "space": "NW" }),
        ),
        (
            IpcMessage::Views { level: None },
            serde_json::json!({ "type": "views", "level": null }),
        ),
        (
            IpcMessage::Go {
                level: "NW".into(),
                view: None,
            },
            serde_json::json!({ "type": "go", "level": "NW", "view": null }),
        ),
        (
            IpcMessage::Go {
                level: "NW/Open bugs".into(),
                view: Some("Board".into()),
            },
            serde_json::json!({ "type": "go", "level": "NW/Open bugs", "view": "Board" }),
        ),
        (
            IpcMessage::Me { on: None },
            serde_json::json!({ "type": "me", "on": null }),
        ),
        (
            IpcMessage::Me { on: Some(true) },
            serde_json::json!({ "type": "me", "on": true }),
        ),
        (
            IpcMessage::Layout {
                layout: Layout::Board,
            },
            serde_json::json!({ "type": "layout", "layout": "board" }),
        ),
        (
            IpcMessage::Search {
                text: "login page".into(),
            },
            serde_json::json!({ "type": "search", "text": "login page" }),
        ),
        (
            IpcMessage::ResetCache,
            serde_json::json!({ "type": "resetCache" }),
        ),
    ];
    #[cfg(debug_assertions)]
    cases.push((
        IpcMessage::Eval { js: "1 + 1".into() },
        serde_json::json!({ "type": "eval", "js": "1 + 1" }),
    ));
    #[cfg(debug_assertions)]
    cases.push((
        IpcMessage::DevDrift,
        serde_json::json!({ "type": "devDrift" }),
    ));
    #[cfg(debug_assertions)]
    cases.push((
        IpcMessage::DevSeedSynthetic { count: 5000 },
        serde_json::json!({ "type": "devSeedSynthetic", "count": 5000 }),
    ));
    for (message, wire) in cases {
        assert_eq!(serde_json::to_value(&message).unwrap(), wire, "{wire}");
        assert_eq!(
            serde_json::from_value::<IpcMessage>(wire.clone()).unwrap(),
            message,
            "{wire}"
        );
    }
}

#[test]
fn replies_carry_the_error_kind_and_message_on_the_wire() {
    let reply = IpcReply {
        id: "7".into(),
        ok: false,
        result: serde_json::Value::Null,
        error: Some(IpcError::new(ErrorKind::NotFound, "No such issue")),
    };
    assert_eq!(
        serde_json::to_value(&reply).unwrap(),
        serde_json::json!({
            "id": "7",
            "ok": false,
            "result": null,
            "error": { "kind": "not_found", "message": "No such issue" }
        })
    );
}

#[test]
fn ipc_error_from_an_app_error_keeps_its_kind_and_display_text() {
    let cases = [
        (
            AppError::InvalidField {
                field: "title".into(),
                message: "must not be empty".into(),
            },
            IpcError::new(ErrorKind::InvalidInput, "`title`: must not be empty"),
        ),
        (
            AppError::Io("denied".into()),
            IpcError::new(ErrorKind::Io, "File error: denied"),
        ),
        (
            AppError::NotConnected("Connect to Jira first".into()),
            IpcError::new(ErrorKind::NotConnected, "Connect to Jira first"),
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(IpcError::from(error.clone()), expected, "{error:?}");
    }
}

#[test]
fn a_failed_reply_without_an_error_gets_a_generic_message() {
    let reply = IpcReply {
        id: "1".into(),
        ok: false,
        result: serde_json::Value::Null,
        error: None,
    };
    assert_eq!(
        reply.into_result(),
        Err(IpcError::new(ErrorKind::Io, "The request failed"))
    );
}

#[test]
fn a_successful_reply_turns_into_its_result() {
    let reply = IpcReply {
        id: "1".into(),
        ok: true,
        result: serde_json::json!({ "ids": [1, 2] }),
        error: None,
    };
    assert_eq!(
        reply.into_result(),
        Ok(serde_json::json!({ "ids": [1, 2] }))
    );
}

#[tokio::test]
async fn request_over_64_kib_gets_no_reply_and_returns_an_error() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let mut rx = serve_into_channel(&socket);

    let huge = IpcMessage::Error {
        message: "a".repeat(MAX_MESSAGE_BYTES as usize + 1),
    };
    assert!(ask(&socket, huge).await.is_err());

    assert_eq!(forward(&socket, activate()).await, Forward::Delivered);
    assert_eq!(rx.recv().await.unwrap(), activate());
}

#[tokio::test]
async fn one_way_message_is_acknowledged_with_ok_and_reaches_the_handler() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let mut rx = serve_into_channel(&socket);

    let line = format!("{}\n", serde_json::to_string(&activate()).unwrap());
    assert_eq!(raw_exchange(&socket, line.as_bytes()).await, "ok\n");
    assert_eq!(rx.recv().await.unwrap(), activate());

    assert_eq!(forward(&socket, activate()).await, Forward::Delivered);
    assert_eq!(rx.recv().await.unwrap(), activate());
}

#[tokio::test]
async fn messages_round_trip_through_the_socket() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let mut rx = serve_into_channel(&socket);

    for message in [
        IpcMessage::Error {
            message: "boom".into(),
        },
        IpcMessage::Open {
            key: "NW-12".into(),
        },
        IpcMessage::Activate { hidden: true },
        activate(),
    ] {
        assert_eq!(forward(&socket, message.clone()).await, Forward::Delivered);
        assert_eq!(rx.recv().await.unwrap(), message);
    }
}

#[tokio::test]
async fn malformed_line_gets_no_ack_and_reaches_no_handler() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let mut rx = serve_into_channel(&socket);

    assert_eq!(raw_exchange(&socket, b"{\"type\": nope\n").await, "");

    assert_eq!(forward(&socket, activate()).await, Forward::Delivered);
    assert_eq!(rx.recv().await.unwrap(), activate());
}

#[tokio::test]
async fn line_over_64_kib_is_refused() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let mut rx = serve_into_channel(&socket);

    let mut huge = br#"{"type":"error","message":""#.to_vec();
    huge.extend(std::iter::repeat_n(b'a', 70 * 1024));
    huge.extend_from_slice(b"\"}\n");
    assert_eq!(raw_exchange(&socket, &huge).await, "");

    assert_eq!(forward(&socket, activate()).await, Forward::Delivered);
    assert_eq!(rx.recv().await.unwrap(), activate());
}

#[tokio::test]
async fn two_clients_at_once_are_both_served() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let mut rx = serve_into_channel(&socket);

    let first = IpcMessage::Error {
        message: "first".into(),
    };
    let second = IpcMessage::Error {
        message: "second".into(),
    };
    let (a, b) = tokio::join!(
        forward(&socket, first.clone()),
        forward(&socket, second.clone())
    );
    assert_eq!((a, b), (Forward::Delivered, Forward::Delivered));

    let mut received = vec![rx.recv().await.unwrap(), rx.recv().await.unwrap()];
    received.sort_by_key(|m| format!("{m:?}"));
    assert_eq!(received, vec![first, second]);
}

#[test]
fn activate_without_hidden_deserializes_as_not_hidden() {
    let message: IpcMessage = serde_json::from_str(r#"{"type":"activate"}"#).unwrap();
    assert_eq!(message, IpcMessage::Activate { hidden: false });
}

#[test]
fn sync_without_full_deserializes_as_not_full() {
    let message: IpcMessage = serde_json::from_str(r#"{"type":"sync"}"#).unwrap();
    assert_eq!(message, IpcMessage::Sync { full: false });
}

#[tokio::test]
async fn a_sync_request_reaches_the_handler_with_its_flag() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let mut rx = serve_into_channel(&socket);

    let message = IpcMessage::Sync { full: true };
    assert_eq!(forward(&socket, message.clone()).await, Forward::Delivered);
    assert_eq!(rx.recv().await.unwrap(), message);
}

#[test]
fn forwarding_without_running_instance_reports_no_instance() {
    let dir = temp_config_dir();
    let forwarded = try_forward(&dir.path().join("none.sock"), &activate()).unwrap();
    assert_eq!(forwarded, Forward::NoInstance);
}

#[test]
fn request_without_a_socket_returns_none() {
    let dir = temp_config_dir();
    let reply = request(&dir.path().join("none.sock"), activate()).unwrap();
    assert_eq!(reply, None);
}

#[test]
fn stale_socket_file_means_no_instance_and_is_replaced_by_bind() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    drop(UnixListener::bind(&socket).unwrap());
    assert!(socket.exists());

    assert_eq!(
        try_forward(&socket, &activate()).unwrap(),
        Forward::NoInstance
    );
    assert!(bind(&socket).is_ok());
}

#[test]
fn bind_refuses_while_a_listener_is_alive() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let _alive = bind(&socket).unwrap();
    let err = bind(&socket).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::AddrInUse);
}

#[test]
fn instance_that_closes_without_ack_rejects_the_message() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).unwrap();
    });

    let forwarded = try_forward(&socket, &activate()).unwrap();
    server.join().unwrap();
    assert_eq!(forwarded, Forward::Rejected);
}

#[test]
fn socket_is_private_to_the_user() {
    let dir = temp_config_dir();
    let socket = dir.path().join("tb.sock");
    let _listener = bind(&socket).unwrap();
    let mode = std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

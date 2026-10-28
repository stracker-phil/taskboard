//! Minimal HTTP/1.1: one request per connection, `Content-Length` bodies, JSON answers.

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// What the fake Jira was asked.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    pub method: String,
    /// Without the query string.
    pub path: String,
    pub query: Vec<(String, String)>,
    pub authorization: Option<String>,
    pub body: Value,
}

impl Recorded {
    pub fn query_value(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

impl Response {
    pub fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body,
        }
    }
}

pub async fn read_request(socket: &mut TcpStream) -> Option<Recorded> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end = loop {
        if let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break at;
        }
        let read = socket.read(&mut chunk).await.ok()?;
        if read == 0 {
            return None;
        }
        data.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&data[..header_end]).to_string();
    let header = |name: &str| {
        head.lines().skip(1).find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    };
    let length: usize = header("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    while data.len() < header_end + 4 + length {
        let read = socket.read(&mut chunk).await.ok()?;
        if read == 0 {
            break;
        }
        data.extend_from_slice(&chunk[..read]);
    }
    let mut request_line = head.lines().next()?.split_whitespace();
    let method = request_line.next()?.to_string();
    let target = url::Url::parse(&format!("http://fake{}", request_line.next()?)).ok()?;
    let body = &data[header_end + 4..];
    Some(Recorded {
        method,
        path: target.path().to_string(),
        query: target.query_pairs().into_owned().collect(),
        authorization: header("authorization"),
        body: serde_json::from_slice(body).unwrap_or(Value::Null),
    })
}

pub async fn write_response(socket: &mut TcpStream, response: Response) {
    let body = response.body.to_string();
    let mut head = format!(
        "HTTP/1.1 {} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status,
        body.len()
    );
    for (name, value) in &response.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let _ = socket.write_all(head.as_bytes()).await;
    let _ = socket.write_all(body.as_bytes()).await;
    let _ = socket.shutdown().await;
}

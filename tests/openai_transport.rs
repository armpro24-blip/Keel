//! Transport tests for `OpenAiChatModel::complete` against a local one-shot
//! HTTP server on 127.0.0.1. No network, no real provider.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};

mod common;

use common::TempDir;
use keel::log::{read_events, SessionLog};
use keel::message::Message;
use keel::model::{Model, ModelError};
use keel::openai::OpenAiChatModel;
use serde_json::Value;

/// What the fake server saw in the single request it accepted.
struct Captured {
    request_line: String,
    /// Header names lowercased.
    headers: Vec<(String, String)>,
    body: Value,
}

impl Captured {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Serve exactly one request with the given status line and JSON body, then
/// close. Returns the base URL to point the adapter at and the handle that
/// yields what the server captured.
fn serve_once(status_line: &'static str, body: &'static str) -> (String, JoinHandle<Captured>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("local addr").port();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 4096];

        let header_end = loop {
            let read = stream.read(&mut chunk).expect("read request");
            assert!(read > 0, "client closed before sending headers");
            buffer.extend_from_slice(&chunk[..read]);
            if let Some(position) = find(&buffer, b"\r\n\r\n") {
                break position + 4;
            }
        };
        let head = String::from_utf8(buffer[..header_end].to_vec()).expect("ascii head");
        let mut lines = head.split("\r\n");
        let request_line = lines.next().expect("request line").to_string();
        let headers: Vec<(String, String)> = lines
            .filter(|line| !line.is_empty())
            .map(|line| {
                let (key, value) = line.split_once(':').expect("header line");
                (key.trim().to_ascii_lowercase(), value.trim().to_string())
            })
            .collect();
        let content_length: usize = headers
            .iter()
            .find(|(key, _)| key == "content-length")
            .expect("request carries content-length")
            .1
            .parse()
            .expect("numeric content-length");
        while buffer.len() < header_end + content_length {
            let read = stream.read(&mut chunk).expect("read body");
            assert!(read > 0, "client closed before sending the whole body");
            buffer.extend_from_slice(&chunk[..read]);
        }
        let body_bytes = &buffer[header_end..header_end + content_length];
        let request_body: Value = serde_json::from_slice(body_bytes).expect("JSON body");

        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");

        Captured {
            request_line,
            headers,
            body: request_body,
        }
    });
    (format!("http://127.0.0.1:{port}/v1"), handle)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[test]
fn complete_posts_to_chat_completions_with_bearer_auth_and_parses_reply() {
    let (base_url, server) = serve_once(
        "HTTP/1.1 200 OK",
        r#"{"choices":[{"message":{"role":"assistant","content":"pong"}}]}"#,
    );
    let mut model =
        OpenAiChatModel::new("test-key".to_string(), base_url, "test-model".to_string());

    let reply = model
        .complete("sys", &[Message::user_text("ping")], &[])
        .unwrap();
    let captured = server.join().expect("server thread");

    assert_eq!(reply, Message::assistant_text("pong"));
    assert_eq!(captured.request_line, "POST /v1/chat/completions HTTP/1.1");
    assert_eq!(captured.header("authorization"), Some("Bearer test-key"));
    assert_eq!(captured.header("content-type"), Some("application/json"));
    assert_eq!(captured.body["model"], "test-model");
    assert_eq!(captured.body["messages"][0]["role"], "system");
    assert_eq!(captured.body["messages"][1]["content"], "ping");
    assert!(captured.body.get("tools").is_none());
}

#[test]
fn non_2xx_with_error_object_reports_the_provider_message() {
    let (base_url, server) = serve_once(
        "HTTP/1.1 401 Unauthorized",
        r#"{"error":{"message":"bad key","type":"invalid_request_error"}}"#,
    );
    let mut model = OpenAiChatModel::new("k".to_string(), base_url, "m".to_string());

    let error = model
        .complete("sys", &[Message::user_text("x")], &[])
        .unwrap_err();
    server.join().expect("server thread");

    assert_eq!(error, ModelError::Provider("bad key".to_string()));
}

#[test]
fn non_2xx_without_error_object_reports_the_status() {
    let (base_url, server) = serve_once("HTTP/1.1 503 Service Unavailable", r#"{"status":"down"}"#);
    let mut model = OpenAiChatModel::new("k".to_string(), base_url, "m".to_string());

    let error = model
        .complete("sys", &[Message::user_text("x")], &[])
        .unwrap_err();
    server.join().expect("server thread");

    match error {
        ModelError::Provider(detail) => {
            assert!(detail.starts_with("HTTP 503"), "got {detail}");
            assert!(detail.contains("down"), "got {detail}");
        }
        other => panic!("expected Provider error, got {other:?}"),
    }
}

#[test]
fn unreachable_server_is_a_provider_error() {
    // Bind and drop immediately so the port is closed when the adapter connects.
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind loopback")
        .local_addr()
        .expect("local addr")
        .port();
    let mut model = OpenAiChatModel::new(
        "k".to_string(),
        format!("http://127.0.0.1:{port}/v1"),
        "m".to_string(),
    );

    let error = model
        .complete("sys", &[Message::user_text("x")], &[])
        .unwrap_err();

    match error {
        ModelError::Provider(detail) => {
            assert!(detail.starts_with("HTTP request failed"), "got {detail}")
        }
        other => panic!("expected Provider error, got {other:?}"),
    }
}

#[test]
fn the_wire_log_records_the_exact_request_and_response_bodies() {
    let dir = TempDir::new("wire");
    let (base_url, server) = serve_once(
        "HTTP/1.1 200 OK",
        r#"{"choices":[{"message":{"role":"assistant","content":"pong"}}]}"#,
    );
    let mut model = OpenAiChatModel::new("k".to_string(), base_url, "m".to_string());
    let wire = SessionLog::open(&dir.path, "s.wire").unwrap();
    let wire_path = wire.path().to_path_buf();
    model.record_wire_to(wire);

    model
        .complete("sys", &[Message::user_text("ping")], &[])
        .unwrap();
    server.join().expect("server thread");

    let events = read_events(&wire_path).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event"], "request");
    assert_eq!(events[0]["body"]["messages"][0]["content"], "sys");
    assert_eq!(events[0]["body"]["messages"][1]["content"], "ping");
    assert_eq!(events[1]["event"], "response");
    assert_eq!(
        events[1]["body"]["choices"][0]["message"]["content"],
        "pong"
    );
    assert!(model.take_wire_log_failure().is_none());
}

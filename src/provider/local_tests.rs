use super::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

#[test]
fn only_loopback_endpoints_are_accepted() {
    assert_eq!(validate_endpoint("http://127.0.0.1:11434/v1/"), Ok("http://127.0.0.1:11434/v1".into()));
    assert!(validate_endpoint("http://localhost:1234/v1").is_ok());
    assert!(validate_endpoint("http://LOCALHOST/v1").is_ok());
    assert!(validate_endpoint("http://[::1]:8080/v1").is_ok());
    for bad in [
        "http://192.168.1.5:11434/v1",
        "http://example.com/v1",
        "http://127.0.0.1@example.com/v1",
        "http://localhost.example.com/v1",
        "http://127.0.0.1.nip.io/v1",
        "https://127.0.0.1:11434/v1",
        "127.0.0.1:11434/v1",
        "http://127.0.0.1:99999/v1",
        "http://[::2]/v1",
    ] {
        assert!(validate_endpoint(bad).is_err(), "{bad} must be refused");
    }
}

/// A one-shot loopback server: answers one request with `status` and `body`, and returns the
/// request it got.
fn stub(status: &str, body: &str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (tx, rx) = mpsc::channel();
    let response =
        format!("HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}", body.len());
    thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read");
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap_or(0);
            }
            head.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).expect("body");
        reader.get_mut().write_all(response.as_bytes()).expect("write");
        tx.send(head + &String::from_utf8_lossy(&body)).ok();
    });
    (format!("http://127.0.0.1:{port}/v1"), rx)
}

#[test]
fn a_rewrite_posts_the_exact_model_and_the_delimited_text() {
    let (endpoint, request) =
        stub("200 OK", r#"{"choices":[{"message":{"content":"```\nThe meeting moved to Tuesday.\n```"}}]}"#);
    let answer = rewrite(&endpoint, "qwen2.5:0.5b", &Style::Clear, "the meeting are moved");
    assert_eq!(answer, Ok("The meeting moved to Tuesday.".into()));
    let request = request.recv().expect("request");
    assert!(request.starts_with("POST /v1/chat/completions"), "{request}");
    let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).expect("body")).expect("json");
    assert_eq!(body["model"], "qwen2.5:0.5b");
    assert_eq!(body["stream"], false);
    assert_eq!(body["messages"][1]["content"], "<text>\nthe meeting are moved\n</text>");
}

#[test]
fn server_errors_and_runaway_answers_are_failures() {
    let (endpoint, _) = stub("404 Not Found", r#"{"error":{"message":"model 'nope' not found"}}"#);
    assert_eq!(
        rewrite(&endpoint, "nope", &Style::Clear, "text"),
        Err("The server answered 404 Not Found: model 'nope' not found".into())
    );
    let long = "word ".repeat(200);
    let (endpoint, _) = stub("200 OK", &format!(r#"{{"choices":[{{"message":{{"content":"{long}"}}}}]}}"#));
    assert!(rewrite(&endpoint, "m", &Style::Clear, "hi").is_err());
}

#[test]
fn models_are_listed_by_exact_id() {
    let (endpoint, request) =
        stub("200 OK", r#"{"object":"list","data":[{"id":"qwen2.5:0.5b"},{"id":"inkling-2-mini"}]}"#);
    assert_eq!(list_models(&endpoint), Ok(vec!["qwen2.5:0.5b".into(), "inkling-2-mini".into()]));
    assert!(request.recv().expect("request").starts_with("GET /v1/models"));
}

#[test]
fn a_closed_port_says_the_server_is_not_running() {
    let port = TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port();
    let error = rewrite(&format!("http://127.0.0.1:{port}/v1"), "m", &Style::Clear, "text").unwrap_err();
    assert!(error.contains("Cannot reach"), "{error}");
}

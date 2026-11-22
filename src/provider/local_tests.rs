use super::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

#[test]
fn without_the_opt_in_only_this_mac_is_accepted() {
    assert_eq!(validate_endpoint("http://127.0.0.1:11434/v1/", false), Ok("http://127.0.0.1:11434/v1".into()));
    for good in ["http://localhost:1234/v1", "http://LOCALHOST/v1", "http://[::1]:8080/v1", "https://127.0.0.1:8443/v1"]
    {
        assert!(validate_endpoint(good, false).is_ok(), "{good} must be accepted");
    }
    for bad in [
        "http://192.168.1.5:11434/v1",
        "http://100.101.1.2:11434/v1",
        "http://box.tailnet.ts.net:11434/v1",
        "http://127.0.0.1@example.com/v1",
        "http://localhost.example.com/v1",
        "127.0.0.1:11434/v1",
        "ftp://127.0.0.1/v1",
        "http://127.0.0.1:99999/v1",
    ] {
        assert!(validate_endpoint(bad, false).is_err(), "{bad} must be refused");
    }
}

#[test]
fn with_the_opt_in_private_addresses_and_names_are_accepted_but_public_ones_never() {
    for good in [
        "http://10.0.0.2:11434/v1",
        "http://172.31.255.255/v1",
        "http://192.168.1.5:11434/v1",
        "http://100.64.0.1/v1",
        "http://100.127.255.254/v1",
        "http://169.254.10.10/v1",
        "http://[fd7a:115c:a1e0::1]:11434/v1",
        "http://[fe80::1]/v1",
        "https://box.tailnet.ts.net/v1",
        "http://studio.local:1234/v1",
    ] {
        assert!(validate_endpoint(good, true).is_ok(), "{good} must be accepted");
    }
    // Each just outside a private range, and the user-name trick that still points elsewhere.
    for bad in [
        "http://8.8.8.8/v1",
        "http://172.32.0.1/v1",
        "http://100.63.255.255/v1",
        "http://100.128.0.1/v1",
        "http://[2001:4860::8888]/v1",
        "http://[::ffff:8.8.8.8]/v1",
        "http://10.0.0.2@example.com/v1",
        "http://-bad.local/v1",
    ] {
        assert!(validate_endpoint(bad, true).is_err(), "{bad} must be refused");
    }
}

#[test]
fn the_resolver_policy_follows_the_endpoint_host() {
    let ip = |s: &str| s.parse::<IpAddr>().expect("ip");
    // A loopback endpoint (`localhost`) may only reach loopback, even with the opt-in on.
    assert!(allowed(ip("127.0.0.1"), true));
    assert!(allowed(ip("::ffff:127.0.0.1"), true));
    assert!(!allowed(ip("192.168.1.5"), true));
    // A network endpoint (a name) may reach private addresses, never public ones.
    assert!(allowed(ip("100.101.1.2"), false));
    assert!(allowed(ip("fd7a:115c:a1e0::1"), false));
    assert!(!allowed(ip("93.184.216.34"), false));
    assert!(!allowed(ip("::ffff:93.184.216.34"), false));
}

#[test]
fn network_host_names_servers_outside_this_mac_only() {
    assert_eq!(network_host(OLLAMA), None);
    assert_eq!(network_host("http://localhost:1234/v1"), None);
    assert_eq!(network_host("http://[::1]:8080/v1"), None);
    assert_eq!(network_host("http://100.101.1.2:11434/v1"), Some("100.101.1.2:11434"));
    assert_eq!(network_host("https://box.tailnet.ts.net/v1"), Some("box.tailnet.ts.net"));
}

/// A one-shot loopback server: answers one request with `status` and `body`, and returns the
/// request it got.
fn stub(status: &str, body: &str) -> (String, mpsc::Receiver<String>) {
    stub_typed(status, "application/json", body)
}

fn stub_typed(status: &str, content_type: &str, body: &str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (tx, rx) = mpsc::channel();
    let response =
        format!("HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\n\r\n{body}", body.len());
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
    let answer =
        rewrite(&endpoint, "qwen2.5:0.5b", None, &Style::Clear, "the meeting are moved", &Cancel::default(), &|_| {});
    assert_eq!(answer, Ok("The meeting moved to Tuesday.".into()));
    let request = request.recv().expect("request");
    assert!(request.starts_with("POST /v1/chat/completions"), "{request}");
    let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).expect("body")).expect("json");
    assert_eq!(body["model"], "qwen2.5:0.5b");
    // Asked to stream; this server answered with one JSON body, which still works.
    assert_eq!(body["stream"], true);
    let roles: Vec<&str> =
        body["messages"].as_array().expect("messages").iter().map(|m| m["role"].as_str().unwrap_or("")).collect();
    assert_eq!(roles, ["system", "user", "assistant", "user"], "the style example comes before the text");
    assert_eq!(body["messages"][3]["content"], "<text>\nthe meeting are moved\n</text>");
}

#[test]
fn server_errors_and_runaway_answers_are_failures() {
    let (endpoint, _) = stub("404 Not Found", r#"{"error":{"message":"model 'nope' not found"}}"#);
    assert_eq!(
        rewrite(&endpoint, "nope", None, &Style::Clear, "text", &Cancel::default(), &|_| {}),
        Err("The server answered 404 Not Found: model 'nope' not found".into())
    );
    let long = "word ".repeat(200);
    let (endpoint, _) = stub("200 OK", &format!(r#"{{"choices":[{{"message":{{"content":"{long}"}}}}]}}"#));
    assert!(rewrite(&endpoint, "m", None, &Style::Clear, "hi", &Cancel::default(), &|_| {}).is_err());
}

#[test]
fn models_are_listed_by_exact_id() {
    let (endpoint, request) =
        stub("200 OK", r#"{"object":"list","data":[{"id":"qwen2.5:0.5b"},{"id":"inkling-2-mini"}]}"#);
    assert_eq!(list_models(&endpoint, false, None), Ok(vec!["qwen2.5:0.5b".into(), "inkling-2-mini".into()]));
    let request = request.recv().expect("request");
    assert!(request.starts_with("GET /v1/models"));
    assert!(!request.to_ascii_lowercase().contains("authorization"), "no key, no header");
}

#[test]
fn llama_server_gets_the_key_as_a_bearer_token_and_a_401_names_the_key() {
    let key = ApiKey("s3cret".into());
    let (endpoint, request) = stub("200 OK", r#"{"choices":[{"message":{"content":"Done."}}]}"#);
    assert_eq!(
        rewrite(&endpoint, "m", Some(&key), &Style::Clear, "done", &Cancel::default(), &|_| {}),
        Ok("Done.".into())
    );
    assert!(request.recv().expect("request").contains("authorization: Bearer s3cret\r\n"));
    let (endpoint, _) = stub("401 Unauthorized", r#"{"error":{"message":"Invalid API Key","code":401}}"#);
    let error = list_models(&endpoint, false, None).unwrap_err();
    assert!(error.contains("needs an API key"), "{error}");
    assert_eq!(format!("{key:?}"), "ApiKey(***)", "Debug never shows the key");
}

#[test]
fn only_gguf_paths_are_shortened_for_display() {
    assert_eq!(short_model_name("/Users/me/models/qwen2.5-0.5b-q4_k_m.gguf"), "qwen2.5-0.5b-q4_k_m.gguf");
    assert_eq!(short_model_name("hf.co/bartowski/Qwen:Q4_K_M"), "hf.co/bartowski/Qwen:Q4_K_M");
    assert_eq!(short_model_name("qwen2.5:0.5b"), "qwen2.5:0.5b");
}

#[test]
fn a_closed_port_says_the_server_is_not_running() {
    let port = TcpListener::bind("127.0.0.1:0").expect("bind").local_addr().expect("addr").port();
    let error =
        rewrite(&format!("http://127.0.0.1:{port}/v1"), "m", None, &Style::Clear, "text", &Cancel::default(), &|_| {})
            .unwrap_err();
    assert!(error.contains("Cannot reach"), "{error}");
}

/// Server-sent events as Ollama, LM Studio and llama-server send them.
fn sse(deltas: &[&str]) -> String {
    let mut body = String::from(": keep-alive\n\n");
    for delta in deltas {
        body += &format!("data: {}\n\n", json!({ "choices": [{ "delta": { "content": delta } }] }));
    }
    body + "data: [DONE]\n\n"
}

#[test]
fn a_streamed_answer_reports_the_text_so_far_and_ends_clean() {
    let (endpoint, _) = stub_typed("200 OK", "text/event-stream", &sse(&["```\nHi", " team", ".\n```"]));
    let seen = std::sync::Mutex::new(Vec::new());
    let answer = rewrite(&endpoint, "m", None, &Style::Clear, "hi team", &Cancel::default(), &|so_far| {
        seen.lock().expect("lock").push(so_far.to_string());
    });
    // Partials are raw; only the final answer loses the fence.
    assert_eq!(*seen.lock().expect("lock"), ["```\nHi", "```\nHi team", "```\nHi team.\n```"]);
    assert_eq!(answer, Ok("Hi team.".into()));
}

#[test]
fn a_streamed_runaway_stops_at_the_cap_and_a_cancel_stops_reading() {
    let chunk = "word ".repeat(50);
    let (endpoint, _) = stub_typed("200 OK", "text/event-stream", &sse(&[chunk.as_str(); 20]));
    let partials = std::sync::atomic::AtomicUsize::new(0);
    let answer = rewrite(&endpoint, "m", None, &Style::Clear, "hi", &Cancel::default(), &|_| {
        partials.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });
    assert!(answer.is_err_and(|e| e.contains("far longer")));
    // The 400-character cap is passed in the second chunk (250 characters each).
    assert_eq!(partials.load(std::sync::atomic::Ordering::Relaxed), 1);

    let (endpoint, _) = stub_typed("200 OK", "text/event-stream", &sse(&["Hi"]));
    let cancel = Cancel::default();
    cancel.cancel();
    assert_eq!(rewrite(&endpoint, "m", None, &Style::Clear, "hi", &cancel, &|_| {}), Err("Cancelled.".into()));
}

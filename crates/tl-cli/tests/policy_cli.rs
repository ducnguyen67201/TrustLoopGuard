use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

use tempfile::tempdir;

const POLICY_YAML: &str = r#"
id: refund-guarantee
description: Prevent guaranteed refund promises.
match:
  literal: guaranteed refund
action: deny
severity: high
"#;

fn tl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tl"))
}

fn write_policy_file() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("refund-guarantee.yaml");
    std::fs::write(&path, POLICY_YAML).expect("write policy");
    (dir, path)
}

#[test]
fn policy_validate_reports_valid_yaml() {
    let (_dir, path) = write_policy_file();

    let output = tl()
        .args(["policy", "validate"])
        .arg(&path)
        .output()
        .expect("run tl");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("ok: policy `refund-guarantee` valid"));
}

const FAMILY_POLICY_YAML: &str = r#"
family: approval
id: payments-need-admin
when:
  tools: [payment.transfer]
approver_roles: [admin]
action: require_approval
"#;

fn write_family_policy_file() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("payments-need-admin.yaml");
    std::fs::write(&path, FAMILY_POLICY_YAML).expect("write policy");
    (dir, path)
}

#[test]
fn policy_validate_reports_valid_family_yaml() {
    let (_dir, path) = write_family_policy_file();

    let output = tl()
        .args(["policy", "validate"])
        .arg(&path)
        .output()
        .expect("run tl");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("ok: family policy `payments-need-admin` valid"),
        "stdout: {out}"
    );
}

#[test]
fn policy_push_posts_family_yaml_to_server() {
    let (_dir, path) = write_family_policy_file();
    let (url, seen) = spawn_server(
        "HTTP/1.1 201 Created\r\ncontent-type: application/json\r\n\r\n\
         {\"id\":\"payments-need-admin\",\"description\":\"\",\"severity\":\"medium\",\"enabled\":true,\"source_yaml\":\"family: approval\"}",
    );

    let output = tl()
        .args(["policy", "push"])
        .arg(&path)
        .args(["--url", &url, "--api-key", "secret"])
        .output()
        .expect("run tl");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("ok: pushed policy `payments-need-admin`"));

    let request = seen.recv().expect("request");
    assert!(request.starts_with("POST /v1/policies HTTP/1.1"));
    assert!(request.contains("content-type: application/yaml"));
    assert!(request.contains("family: approval"));
}

#[test]
fn policy_push_posts_yaml_to_server() {
    let (_dir, path) = write_policy_file();
    let (url, seen) = spawn_server(
        "HTTP/1.1 201 Created\r\ncontent-type: application/json\r\n\r\n\
         {\"id\":\"refund-guarantee\",\"description\":\"Prevent guaranteed refund promises.\",\"severity\":\"high\",\"enabled\":true,\"source_yaml\":\"id: refund-guarantee\"}",
    );

    let output = tl()
        .args(["policy", "push"])
        .arg(&path)
        .args(["--url", &url, "--api-key", "secret"])
        .output()
        .expect("run tl");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("ok: pushed policy `refund-guarantee`"));

    let request = seen.recv().expect("request");
    assert!(request.starts_with("POST /v1/policies HTTP/1.1"));
    assert!(request.contains("content-type: application/yaml"));
    assert!(request.contains("authorization: Bearer secret"));
    assert!(request.contains("id: refund-guarantee"));
}

#[test]
fn policy_pull_writes_source_yaml_to_file() {
    let dir = tempdir().expect("tempdir");
    let output_path = dir.path().join("pulled.yaml");
    let (url, seen) = spawn_server(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n\
         {\"id\":\"refund-guarantee\",\"description\":\"Prevent guaranteed refund promises.\",\"severity\":\"high\",\"enabled\":true,\"source_yaml\":\"id: refund-guarantee\\nseverity: high\\n\"}",
    );

    let output = tl()
        .args(["policy", "pull", "refund-guarantee"])
        .args(["--output"])
        .arg(&output_path)
        .args(["--url", &url])
        .output()
        .expect("run tl");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("ok: pulled policy `refund-guarantee`"));
    assert_eq!(
        std::fs::read_to_string(&output_path).expect("pulled yaml"),
        "id: refund-guarantee\nseverity: high\n"
    );

    let request = seen.recv().expect("request");
    assert!(request.starts_with("GET /v1/policies/refund-guarantee HTTP/1.1"));
}

fn spawn_server(response: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("local addr"));
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request = read_http_request(&mut stream);
        tx.send(request).expect("send request");
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush");
    });
    (url, rx)
}

fn read_http_request(stream: &mut std::net::TcpStream) -> String {
    let mut data = Vec::new();
    let mut buf = [0_u8; 1024];
    loop {
        let n = stream.read(&mut buf).expect("read");
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        if let Some(header_end) = find_header_end(&data) {
            let headers = String::from_utf8_lossy(&data[..header_end]).to_ascii_lowercase();
            let content_length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length: "))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if data.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&data).to_string()
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|window| window == b"\r\n\r\n")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

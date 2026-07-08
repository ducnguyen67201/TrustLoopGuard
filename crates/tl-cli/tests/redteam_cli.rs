use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::sync::mpsc;
use std::thread;

fn tl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tl"))
}

#[test]
fn regression_check_passes_when_counts_are_within_thresholds() {
    let (url, seen) = spawn_server(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n\
         {\"job\":{\"id\":\"job-1\",\"workspace_id\":\"workspace-1\",\"environment_id\":\"env-1\",\"status\":\"complete\",\"target\":\"http://agent\",\"profile\":\"fast\",\"agent_id\":null,\"attacks\":2,\"landed\":0,\"blocked\":2,\"error\":null,\"created_at\":\"2026-01-01T00:00:00Z\",\"updated_at\":\"2026-01-01T00:00:00Z\"},\"source_job_id\":\"source-1\",\"total\":2,\"passed\":2,\"failed\":0,\"missing\":0,\"inconclusive\":0,\"results\":[]}",
    );

    let output = tl()
        .args([
            "redteam",
            "regressions",
            "check",
            "job-1",
            "--source-job-id",
            "source-1",
            "--case-key",
            "case/a",
            "--limit",
            "10",
            "--url",
            &url,
            "--api-key",
            "secret",
        ])
        .output()
        .expect("run tl");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("regression job `job-1`: 2/2 passed"),
        "stdout: {out}"
    );
    assert!(out.contains("ok: regression check passed"), "stdout: {out}");

    let request = seen.recv().expect("request");
    assert!(request.starts_with(
        "GET /v1/redteam/regressions/results/job-1?source_job_id=source-1&case_key=case%2Fa&limit=10 HTTP/1.1"
    ));
    assert!(request.contains("authorization: Bearer secret"));
}

#[test]
fn regression_check_fails_when_counts_exceed_thresholds() {
    let (url, _seen) = spawn_server(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n\
         {\"job\":{\"id\":\"job-2\",\"workspace_id\":\"workspace-1\",\"environment_id\":\"env-1\",\"status\":\"complete\",\"target\":\"http://agent\",\"profile\":\"fast\",\"agent_id\":null,\"attacks\":2,\"landed\":1,\"blocked\":1,\"error\":null,\"created_at\":\"2026-01-01T00:00:00Z\",\"updated_at\":\"2026-01-01T00:00:00Z\"},\"source_job_id\":\"source-1\",\"total\":2,\"passed\":1,\"failed\":1,\"missing\":0,\"inconclusive\":0,\"results\":[{\"case_key\":\"case-failed\",\"expected_outcome\":\"block\",\"status\":\"failed\",\"session_id\":\"sess-1\",\"actual_outcome\":\"landed\",\"landed\":true,\"reason\":\"expected Block, got outcome `landed` landed=true\"}]}",
    );

    let output = tl()
        .args([
            "redteam",
            "regressions",
            "check",
            "job-2",
            "--source-job-id",
            "source-1",
            "--url",
            &url,
        ])
        .output()
        .expect("run tl");

    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("case-failed [failed]"), "stdout: {out}");
    let err = stderr(&output);
    assert!(err.contains("regression check failed"), "stderr: {err}");
}

#[test]
fn regression_history_lists_result_snapshots() {
    let (url, seen) = spawn_server(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n\
         {\"snapshots\":[{\"id\":\"snapshot-1\",\"job_id\":\"job-1\",\"source_job_id\":\"source-1\",\"environment_id\":\"env-1\",\"agent_id\":\"agent-1\",\"case_keys\":[\"case-a\"],\"total\":1,\"passed\":1,\"failed\":0,\"missing\":0,\"inconclusive\":0,\"created_at\":\"2026-01-01T00:00:00Z\",\"updated_at\":\"2026-01-01T00:00:00Z\"}]}",
    );

    let output = tl()
        .args([
            "redteam",
            "regressions",
            "history",
            "--source-job-id",
            "source-1",
            "--agent-id",
            "agent-1",
            "--limit",
            "5",
            "--url",
            &url,
        ])
        .output()
        .expect("run tl");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("regression result snapshots"), "stdout: {out}");
    assert!(out.contains("job=job-1 source=source-1"), "stdout: {out}");

    let request = seen.recv().expect("request");
    assert!(request.starts_with(
        "GET /v1/redteam/regressions/results?source_job_id=source-1&agent_id=agent-1&limit=5 HTTP/1.1"
    ));
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

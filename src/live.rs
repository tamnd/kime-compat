//! The response contract against a running server, over plain HTTP.
//!
//! `kime-compat live <base url>` sends every committed request under `fixtures/` to the server
//! and checks what comes back with the same rules as the committed responses, then does the same
//! for each text in an optional corpus with each fixture's questions. The client is a few lines
//! of HTTP/1.1 on a `TcpStream`, since the harness should not need a TLS stack or an async
//! runtime to talk to a server on localhost.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use serde_json::Value;

/// A server at `http://host:port`, with no path prefix.
#[derive(Debug, Clone)]
pub struct Server {
    host: String,
}

impl Server {
    /// Parses `http://host:port` or `host:port`. `https` is refused rather than sent in the clear.
    pub fn new(url: &str) -> Result<Self, String> {
        if url.starts_with("https://") {
            return Err(format!("{url}: live only speaks plain http, to a server on this machine"));
        }
        let host = url.trim_start_matches("http://").trim_end_matches('/');
        if host.is_empty() || host.contains('/') {
            return Err(format!("{url}: expected http://host:port"));
        }
        Ok(Self { host: host.to_string() })
    }

    /// One request on a fresh connection. Returns the status and the parsed JSON body, or the
    /// body as a string when it is not JSON.
    pub fn call(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<(u16, Value), String> {
        let mut stream =
            TcpStream::connect(&self.host).map_err(|e| format!("{}: {e}", self.host))?;
        stream.set_read_timeout(Some(Duration::from_secs(120))).map_err(|e| e.to_string())?;
        let body = body.unwrap_or("");
        let head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.host,
            body.len()
        );
        stream
            .write_all(head.as_bytes())
            .and_then(|()| stream.write_all(body.as_bytes()))
            .map_err(|e| e.to_string())?;
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).map_err(|e| format!("{method} {path}: {e}"))?;
        parse(&raw).map_err(|e| format!("{method} {path}: {e}"))
    }
}

fn parse(raw: &[u8]) -> Result<(u16, Value), String> {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("no end of headers")?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status: u16 = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("no status in {head:?}"))?;
    let chunked = head.lines().any(|l| {
        let l = l.to_ascii_lowercase();
        l.starts_with("transfer-encoding:") && l.contains("chunked")
    });
    let mut body = raw[split + 4..].to_vec();
    if chunked {
        body = unchunk(&body)?;
    }
    let text = String::from_utf8(body).map_err(|e| e.to_string())?;
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::String(text))))
}

fn unchunk(mut rest: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    loop {
        let end =
            rest.windows(2).position(|w| w == b"\r\n").ok_or("a chunk without a size line")?;
        let size = String::from_utf8_lossy(&rest[..end]);
        let size = usize::from_str_radix(size.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|e| format!("chunk size {size:?}: {e}"))?;
        rest = &rest[end + 2..];
        if size == 0 {
            return Ok(out);
        }
        out.extend_from_slice(rest.get(..size).ok_or("a chunk shorter than its size")?);
        rest = rest.get(size + 2..).ok_or("a chunk without its line end")?;
    }
}

/// What one live run found.
#[derive(Debug, Default)]
pub struct Report {
    /// Requests sent to `/v1/systemone`.
    pub calls: usize,
    /// Every broken rule, prefixed with the case it came from.
    pub problems: Vec<String>,
    /// The time each `/v1/systemone` call took, in milliseconds.
    pub took_ms: Vec<f64>,
}

/// Checks `/v1/models` and every `(name, request)` against the server.
pub fn run(server: &Server, cases: &[(String, Value)]) -> Report {
    let mut report = Report::default();
    match server.call("GET", "/v1/models", None) {
        Ok((200, body)) => report.problems.extend(check_models(&body)),
        Ok((status, body)) => report.problems.push(format!("GET /v1/models: {status} {body}")),
        Err(e) => {
            report.problems.push(e);
            return report;
        }
    }
    for (name, request) in cases {
        let started = Instant::now();
        let result = server.call("POST", "/v1/systemone", Some(&request.to_string()));
        report.took_ms.push(started.elapsed().as_secs_f64() * 1e3);
        report.calls += 1;
        match result {
            Ok((200, response)) => report.problems.extend(
                crate::contract::check(request, &response)
                    .into_iter()
                    .map(|p| format!("{name}: {p}")),
            ),
            Ok((status, body)) => report.problems.push(format!("{name}: {status} {body}")),
            Err(e) => report.problems.push(format!("{name}: {e}")),
        }
    }
    report
}

/// The shape the TypeSafe SDKs parse: `{"models": [{name, description, release_date}]}`, all
/// strings. `typesafe_sdk` 0.7.1 refuses a null `release_date`.
pub fn check_models(body: &Value) -> Vec<String> {
    let Some(models) = body.get("models").and_then(Value::as_array) else {
        return vec!["GET /v1/models: no models array".into()];
    };
    if models.is_empty() {
        return vec!["GET /v1/models: the models array is empty".into()];
    }
    let mut problems = Vec::new();
    for (i, m) in models.iter().enumerate() {
        for field in ["name", "description", "release_date"] {
            if !m.get(field).is_some_and(Value::is_string) {
                problems.push(format!("GET /v1/models: models[{i}].{field} is not a string"));
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_a_plain_and_a_chunked_response() {
        let plain = b"HTTP/1.1 200 OK\r\ncontent-length: 8\r\n\r\n{\"a\": 1}";
        assert_eq!(parse(plain).unwrap(), (200, json!({"a": 1})));
        let chunked = b"HTTP/1.1 422 Unprocessable\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n4\r\n: 2}\r\n0\r\n\r\n";
        assert_eq!(parse(chunked).unwrap(), (422, json!({"a": 2})));
        assert_eq!(parse(b"HTTP/1.1 500 x\r\n\r\noops").unwrap(), (500, json!("oops")));
    }

    #[test]
    fn refuses_https_and_paths() {
        assert!(Server::new("https://api.typesafe.ai").is_err());
        assert!(Server::new("http://127.0.0.1:8000/v1").is_err());
        assert!(Server::new("http://127.0.0.1:8000/").is_ok());
    }

    #[test]
    fn a_null_release_date_fails() {
        let body = json!({"models": [{"name": "laya", "description": "d", "release_date": null}]});
        assert_eq!(
            check_models(&body),
            vec!["GET /v1/models: models[0].release_date is not a string"]
        );
    }
}

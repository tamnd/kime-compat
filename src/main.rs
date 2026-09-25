use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "kime-compat: the compatibility harness for kime

usage: kime-compat <command>

commands:
  surfaces   print the compatibility surfaces and the milestone each starts at
  fixtures   check every committed fixture against the response rules in spec/03-api.md
  live <url> [texts.json]
             send every committed request to a running server as jev-latest and check its
             responses the same way, then each text in texts.json (a list of strings) with each
             fixture's questions
  help       print this
";

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let result = match std::env::args().nth(1).as_deref() {
        Some("surfaces") => {
            std::fs::read_to_string(root.join("surfaces.tsv")).map_err(|e| e.to_string()).map(|t| {
                t.lines()
                    .filter(|l| !l.starts_with('#') && !l.is_empty())
                    .map(|l| l.replace('\t', " | ") + "\n")
                    .collect()
            })
        }
        Some("fixtures") => fixtures(&root.join("fixtures")),
        Some("live") => {
            live(&root.join("fixtures"), std::env::args().nth(2), std::env::args().nth(3))
        }
        Some("--version" | "-V") => Ok(format!("kime-compat {}\n", env!("CARGO_PKG_VERSION"))),
        None | Some("help" | "--help" | "-h") => Ok(HELP.to_string()),
        Some(other) => Err(format!("unknown command {other}\n\n{HELP}")),
    };
    match result {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// Every directory under `fixtures/<surface>/` with a `request.json` and a `response.json`.
fn fixture_dirs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for surface in std::fs::read_dir(root).into_iter().flatten().flatten() {
        for case in std::fs::read_dir(surface.path()).into_iter().flatten().flatten() {
            if case.path().join("request.json").is_file() {
                out.push(case.path());
            }
        }
    }
    out.sort();
    out
}

fn fixtures(root: &Path) -> Result<String, String> {
    let dirs = fixture_dirs(root);
    if dirs.is_empty() {
        return Err("no fixtures found, which means this is checking nothing".into());
    }
    let mut failures = Vec::new();
    for dir in &dirs {
        let read = |name: &str| -> Result<serde_json::Value, String> {
            let p = dir.join(name);
            let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))
        };
        let shown = dir.strip_prefix(root).unwrap_or(dir).display().to_string();
        match (read("request.json"), read("response.json")) {
            (Ok(req), Ok(resp)) => failures.extend(
                kime_compat::contract::check(&req, &resp)
                    .into_iter()
                    .map(|p| format!("{shown}: {p}")),
            ),
            (Err(e), _) | (_, Err(e)) => failures.push(e),
        }
    }
    if failures.is_empty() {
        Ok(format!("{} fixtures obey the response contract\n", dirs.len()))
    } else {
        Err(failures.join("\n"))
    }
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn live(root: &Path, url: Option<String>, texts: Option<String>) -> Result<String, String> {
    let server = kime_compat::live::Server::new(
        &url.ok_or("live needs a base url, such as http://127.0.0.1:8000")?,
    )?;
    let mut cases = Vec::new();
    for dir in fixture_dirs(root) {
        let shown = dir.strip_prefix(root).unwrap_or(&dir).display().to_string();
        let mut request = read_json(&dir.join("request.json"))?;
        // The contract is Jev's, and kime-serve takes a request with no model as laya-serve's
        // dialect, so every request names one. A fixture may name a model from the spec that this
        // server does not serve, and that becomes the alias too.
        if !request
            .get("model")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|m| m.ends_with("-latest"))
        {
            request["model"] = "jev-latest".into();
        }
        cases.push((shown, request));
    }
    if cases.is_empty() {
        return Err("no fixtures found, which means this is checking nothing".into());
    }
    let fixtures = cases.len();
    if let Some(path) = texts {
        let texts = read_json(Path::new(&path))?;
        let texts = texts.as_array().ok_or(format!("{path}: expected a list of strings"))?;
        let templates = cases.clone();
        for (i, text) in texts.iter().enumerate() {
            for (name, request) in &templates {
                let mut request = request.clone();
                request["state"] = text.clone();
                cases.push((format!("{path}[{i}] with {name}"), request));
            }
        }
    }
    let report = kime_compat::live::run(&server, &cases);
    let mut ms = report.took_ms.clone();
    ms.sort_by(f64::total_cmp);
    let at = |q: f64| {
        ms.get(((ms.len() as f64 * q) as usize).min(ms.len().saturating_sub(1)))
            .copied()
            .unwrap_or(0.0)
    };
    let summary = format!(
        "{} requests ({} fixtures, {} from texts), {} broke a rule, p50 {:.1} ms, p99 {:.1} ms\n",
        report.calls,
        fixtures,
        report.calls.saturating_sub(fixtures),
        report.problems.len(),
        at(0.5),
        at(0.99)
    );
    if report.problems.is_empty() {
        Ok(summary)
    } else {
        Err(format!("{}\n{summary}", report.problems.join("\n")))
    }
}

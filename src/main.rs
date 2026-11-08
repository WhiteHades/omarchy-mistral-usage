use serde_json::{Value, json};
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn normalize(input: &Value) -> Result<Value, &'static str> {
    if input["status"] != "ready" {
        return Ok(
            json!({"status": input["status"].as_str().unwrap_or("unavailable"), "budgets": []}),
        );
    }
    let mut budgets = Vec::new();
    for (key, label) in [("vibe_budget", "Vibe Code"), ("api_budget", "API")] {
        let raw = &input[key];
        if raw.is_null() {
            continue;
        }
        let allowance = raw["initial_budget"].as_f64().ok_or("Invalid allowance")?;
        let percent = raw["usage_percentage"].as_f64().ok_or("Invalid usage")?;
        let currency = raw["currency"].as_str().ok_or("Missing currency")?;
        if !allowance.is_finite()
            || allowance < 0.0
            || !percent.is_finite()
            || percent < 0.0
            || currency.len() != 3
            || !currency.bytes().all(|c| c.is_ascii_uppercase())
        {
            return Err("Invalid budget data");
        }
        if allowance == 0.0 {
            continue;
        }
        let used = allowance * (percent / 100.0);
        if !used.is_finite() {
            return Err("Invalid spending amount");
        }
        let reset = raw["reset_at"].as_str().unwrap_or("");
        if reset.len() > 40 || !reset.is_ascii() {
            return Err("Invalid reset time");
        }
        budgets.push(json!({
            "id": key, "label": label, "allowance": allowance,
            "used": used, "remaining": (allowance - used).max(0.0),
            "percent": percent, "currency": currency, "resetAt": reset,
            "paygEnabled": raw["payg_enabled"].as_bool()
        }));
    }
    let plan = match input["plan"].as_str().unwrap_or("") {
        "EDU" => "Student",
        "INDIVIDUAL" => "Pro",
        "TEAM" => "Team",
        "ENTERPRISE" => "Enterprise",
        _ => "Mistral",
    };
    Ok(
        json!({"status": if budgets.is_empty() { "hidden" } else { "ready" }, "plan": plan, "budgets": budgets}),
    )
}

fn fetch() -> Result<Value, &'static str> {
    let install = Command::new("mise")
        .args(["where", "pypi:mistral-vibe"])
        .stderr(Stdio::null())
        .output()
        .map_err(|_| "Vibe is not installed through mise")?;
    if !install.status.success() {
        return Err("Vibe is not installed through mise");
    }
    let install = String::from_utf8(install.stdout).map_err(|_| "Invalid Vibe installation")?;
    let python = PathBuf::from(install.trim()).join("mistral-vibe/bin/python");
    let mut child = Command::new("setpriv")
        .args(["--pdeathsig", "TERM", "--"])
        .arg(python)
        .args(["-c", include_str!("fetch.py")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not start Vibe's authentication client")?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| "Could not read Vibe's response")?
        {
            break status;
        }
        if started.elapsed() > Duration::from_secs(15) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Mistral did not respond in time");
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    if !status.success() {
        return Err("Vibe's authentication client is unavailable");
    }
    let mut output = String::new();
    child
        .stdout
        .take()
        .ok_or("Missing response")?
        .take(65537)
        .read_to_string(&mut output)
        .map_err(|_| "Invalid response")?;
    if output.len() > 65536 {
        return Err("Mistral's response was too large");
    }
    let input: Value = serde_json::from_str(&output).map_err(|_| "Invalid response")?;
    normalize(&input)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("omarchy-mistral-usage {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let result = if args == ["--validate"] {
        let mut input = String::new();
        io::stdin().take(65537).read_to_string(&mut input).ok();
        if input.len() > 65536 {
            Err("Response too large")
        } else {
            serde_json::from_str(&input)
                .map_err(|_| "Invalid JSON")
                .and_then(|v| normalize(&v))
        }
    } else if args.is_empty() {
        fetch()
    } else {
        Err("Usage: omarchy-mistral-usage [--version|--validate]")
    };
    let mut record = result.unwrap_or_else(
        |message| json!({"status": "unavailable", "message": message, "budgets": []}),
    );
    record["checkedAt"] = json!(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    println!("{record}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response() -> Value {
        json!({"status":"ready", "identity":"a".repeat(64), "plan":"EDU",
            "vibe_budget":{"usage_percentage":1.25,"initial_budget":150,"currency":"USD","reset_at":"2026-12-01T00:00:00Z","payg_enabled":false},
            "api_budget":{"usage_percentage":0,"initial_budget":15,"currency":"USD","reset_at":"2026-12-01T00:00:00Z","payg_enabled":false}})
    }

    #[test]
    fn official_percentage_is_not_a_fraction() {
        let result = normalize(&response()).unwrap();
        assert_eq!(result["plan"], "Student");
        assert_eq!(result["budgets"][1]["allowance"], 15.0);
        assert!((result["budgets"][0]["used"].as_f64().unwrap() - 1.875).abs() < 0.000001);
        assert_eq!(result["budgets"][0]["resetAt"], "2026-12-01T00:00:00Z");
    }

    #[test]
    fn hidden_or_failed_usage_never_looks_unused() {
        let hidden = json!({"identity":"a".repeat(64),"status":"ready"});
        assert_eq!(normalize(&hidden).unwrap()["status"], "hidden");
        let denied = json!({"identity":"a".repeat(64),"status":"forbidden"});
        assert!(
            normalize(&denied).unwrap()["budgets"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn malformed_money_is_rejected_and_exhaustion_is_retained() {
        let mut input = response();
        input["vibe_budget"]["usage_percentage"] = json!(-1);
        assert!(normalize(&input).is_err());
        input["vibe_budget"]["usage_percentage"] = json!(125);
        let result = normalize(&input).unwrap();
        assert_eq!(result["budgets"][0]["used"], 187.5);
        assert_eq!(result["budgets"][0]["remaining"], 0.0);
        input["vibe_budget"]["initial_budget"] = json!("150");
        assert!(normalize(&input).is_err());
    }

    #[test]
    fn native_qml_formats_money_and_utc_month_boundaries() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!(".tmp/qml-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Format.js"), include_str!("../Format.js")).unwrap();
        std::fs::write(dir.join("shell.qml"), r#"
import QtQuick
import Quickshell
import "Format.js" as Format
ShellRoot {
  Timer {
    interval: 10
    running: true
    onTriggered: {
      function check(value) { if (!value) throw new Error("Mistral QML check failed") }
      check(Format.money(150, "USD").indexOf("150") >= 0)
      check(Format.money(1.875, "USD").indexOf("1.88") >= 0)
      check(Format.resetTime({resetAt:"2026-12-01T00:00:00Z"}, Date.UTC(2026,10,8)) === Date.UTC(2026,11,1))
      check(Format.resetTime({}, Date.UTC(2026,11,31,23,59)) === Date.UTC(2027,0,1))
      check(Format.resetTime({}, Date.UTC(2028,1,29)) === Date.UTC(2028,2,1))
      check(Format.resetTime({resetAt:"2026-11-01T00:00:00Z"}, Date.UTC(2026,10,8)) === Date.UTC(2026,10,1))
      console.log("MISTRAL_QML_CHECK_OK")
    }
  }
}
"#).unwrap();
        let output = Command::new("timeout")
            .args(["3s", "quickshell", "--no-color", "-p"])
            .arg(&dir)
            .env("QT_QPA_PLATFORM", "offscreen")
            .env("LC_ALL", "C.UTF-8")
            .output()
            .unwrap();
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(
            output.status.code() == Some(124)
                && log.contains("MISTRAL_QML_CHECK_OK")
                && !log.contains("Error:"),
            "{log}"
        );
    }
}

use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

/// Send a plain text message via the existing Telegram telemetry spool.
/// Falls back to synchronous tg_send if spool is unavailable.
pub fn send(text: &str) {
    let spool = PathBuf::from("/tmp/telemetry-spool/queue.jsonl");
    let _ = std::fs::create_dir_all(spool.parent().unwrap());

    let payload = format!("{{\"text\":\"{}\"}}", escape_json(text));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&spool) {
        let _ = f.write_all(payload.as_bytes());
        let _ = f.write_all(b"\n");
    }

    let status = process::Command::new("bash")
        .arg("-c")
        .arg("source /root/dowiz/tools/telemetry/lib.sh >/dev/null 2>&1 && tg_spool_ensure >/dev/null 2>&1 || true")
        .status();

    if status.map(|s| !s.success()).unwrap_or(true) {
        let _ = process::Command::new("bash")
            .arg("-c")
            .arg(format!(
                "source /root/dowiz/tools/telemetry/lib.sh >/dev/null 2>&1 && tg_deliver '{}' >/dev/null 2>&1 || true",
                escape_sh(text)
            ))
            .status();
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_sh(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\'', "'\\''")
}

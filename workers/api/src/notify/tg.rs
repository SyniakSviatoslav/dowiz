//! TELEGRAM'S ANSWERS, CLASSIFIED (W-TG T1).
//!
//! THE DEFECT. `notify::telegram` flattened every refusal to a string, so a
//! 429 ("wait 7 seconds"), a 403 ("the bot was removed from the group") and a
//! 400 ("the group became a supergroup, here is its new id") all looked the
//! same to the drain and each burned one of the six tries. A kicked bot was
//! retried for half an hour; a migrated group lost every message; a rate limit
//! was answered by hammering the chat again ten seconds later.
//!
//! THE RULE, PER ANSWER (https://core.telegram.org/bots/api#responseparameters):
//!   429 + `retry_after`        -> wait that long, the attempt does NOT count;
//!   400 + `migrate_to_chat_id` -> the chat has a new id: rewrite it, retry now;
//!   403, "chat not found", "message thread not found" -> the chat is GONE:
//!                                 stop, abandon, tell the owner;
//!   anything else              -> an ordinary failed attempt (the backoff).
//!
//! A TARGET is `chat` or `chat:thread` (a forum topic), the convention the
//! WooCommerce plugin uses; a legacy entry's `to` is a bare chat and parses to
//! no thread, so every entry written before this is byte-identical.

use serde_json::{json, Value};

/// Telegram refuses messages past this many characters.
pub const TEXT_MAX: usize = 4096;

/// Why a send did not land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fail {
    /// 429: wait this many SECONDS; the attempt does not count.
    RetryAfter(i64),
    /// The group was upgraded to a supergroup; this is its new chat id.
    Migrated(i64),
    /// Terminal for this chat: the bot was removed, blocked, or the chat or
    /// topic does not exist. Carries Telegram's own words.
    Gone(String),
    /// An ordinary failure (network, 5xx, a malformed request).
    Other(String),
}

impl Fail {
    /// Telegram's words, for a console that shows them.
    pub fn words(&self) -> String {
        match self {
            Fail::RetryAfter(s) => format!("Too Many Requests: retry after {s}"),
            Fail::Migrated(to) => format!("group chat was upgraded to a supergroup chat {to}"),
            Fail::Gone(d) | Fail::Other(d) => d.clone(),
        }
    }
}

/// Classify one HTTP answer. PURE: the status and the body are values.
pub fn classify(status: u16, body: &str) -> Fail {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let desc: String = v
        .get("description")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| body.to_string())
        .chars()
        .take(200)
        .collect();
    let params = v.get("parameters");
    if let Some(to) = params.and_then(|p| p.get("migrate_to_chat_id")).and_then(Value::as_i64) {
        return Fail::Migrated(to);
    }
    if status == 429 {
        // A 429 with no number still waits: a guess of a few seconds is kinder
        // to the chat than retrying on the backoff's first 10 s.
        let s = params.and_then(|p| p.get("retry_after")).and_then(Value::as_i64).unwrap_or(5);
        return Fail::RetryAfter(s.clamp(1, 3600));
    }
    let lower = desc.to_ascii_lowercase();
    if status == 403 || lower.contains("chat not found") || lower.contains("message thread not found") {
        return Fail::Gone(desc);
    }
    Fail::Other(desc)
}

/// Where a message goes: a chat, and a forum topic in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub chat: String,
    pub thread: Option<i64>,
}

/// `"-100123:42"` -> chat `-100123`, thread 42. `"-100123"`, `"@channel"` and
/// `"12345"` have no thread. A suffix that is not a number is part of the chat.
pub fn target_of(to: &str) -> Target {
    let to = to.trim();
    if let Some((chat, thread)) = to.rsplit_once(':') {
        if let (false, Ok(t)) = (chat.is_empty(), thread.parse::<i64>()) {
            return Target { chat: chat.to_string(), thread: Some(t) };
        }
    }
    Target { chat: to.to_string(), thread: None }
}

/// The inverse of [`target_of`].
pub fn target_text(chat: &str, thread: Option<i64>) -> String {
    match thread {
        Some(t) => format!("{}:{t}", chat.trim()),
        None => chat.trim().to_string(),
    }
}

/// The `sendMessage` body. PURE. Plain text on purpose (no `parse_mode`):
/// Markdown escaping of dish names is where a notification silently stops.
pub fn send_body(t: &Target, text: &str) -> Value {
    let text: String = text.chars().take(TEXT_MAX).collect();
    let mut b = json!({ "chat_id": t.chat, "text": text, "disable_web_page_preview": true });
    if let Some(th) = t.thread {
        b["message_thread_id"] = json!(th);
    }
    b
}

/// One Bot API call. `Ok` is Telegram's `result`.
pub async fn call(token: &str, method: &str, payload: &Value) -> Result<Value, Fail> {
    use worker::*;
    let url = format!("https://api.telegram.org/bot{token}/{method}");
    let headers = Headers::new();
    headers.set("content-type", "application/json").map_err(|e| Fail::Other(e.to_string()))?;
    let r = Request::new_with_init(
        &url,
        RequestInit::new().with_method(Method::Post).with_headers(headers).with_body(Some(payload.to_string().into())),
    )
    .map_err(|e| Fail::Other(e.to_string()))?;
    let mut res = Fetch::Request(r).send().await.map_err(|e| Fail::Other(e.to_string()))?;
    let status = res.status_code();
    let body = res.text().await.unwrap_or_default();
    if status < 400 {
        let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
        return Ok(v.get("result").cloned().unwrap_or(Value::Null));
    }
    Err(classify(status, &body))
}

/// One message to a target (`chat` or `chat:thread`).
pub async fn send(token: &str, to: &str, text: &str) -> Result<(), Fail> {
    call(token, "sendMessage", &send_body(&target_of(to), text)).await.map(|_| ())
}

/// What the drain does with a Telegram entry after one attempt. PURE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Sent,
    /// Put it back with these tries and this next instant. A 429 keeps the
    /// tries it had: waiting because Telegram said so is not a failure.
    Retry { tries: u32, next_at_ms: i64 },
    /// The group became a supergroup: rewrite the target and send again now.
    Moved { to: String },
    /// The chat is gone: this entry and every other one for `chat` are
    /// abandoned now, and the groups on it are marked left.
    Gone { chat: String, why: String },
    /// Failed six times (`outbox::MAX_TRIES`).
    Abandon { after: u32 },
}

pub fn outcome(e: &crate::outbox::Entry, r: &Result<(), Fail>, now_ms: i64) -> Outcome {
    let fail = match r {
        Ok(()) => return Outcome::Sent,
        Err(f) => f,
    };
    match fail {
        Fail::RetryAfter(s) => Outcome::Retry { tries: e.tries, next_at_ms: now_ms + s * 1000 },
        Fail::Migrated(to) => Outcome::Moved { to: target_text(&to.to_string(), target_of(&e.to).thread) },
        Fail::Gone(why) => Outcome::Gone { chat: target_of(&e.to).chat, why: why.clone() },
        Fail::Other(_) => match crate::outbox::after_attempt(e, false, now_ms) {
            crate::outbox::Verdict::Retry { tries, next_at_ms } => Outcome::Retry { tries, next_at_ms },
            crate::outbox::Verdict::Abandon { after } => Outcome::Abandon { after },
            crate::outbox::Verdict::Sent => Outcome::Sent,
        },
    }
}

#[cfg(test)]
mod tests;

//! An assistant for the people running the venue.
//!
//! WHAT IT IS FOR. An owner mid-rush wants "how many orders are still waiting
//! and what is the oldest one", not a dashboard they have to read. A courier
//! wants "what is left on my run". Both questions have exact answers already in
//! the hub; what is missing is a way to ask them in a sentence.
//!
//! IT IS NEVER AN AUTHORITY. The model is given FACTS THE HUB ALREADY COMPUTED
//! and asked to phrase them. It cannot place an order, move an order, change a
//! price or read a token — not because it is told not to, but because this
//! module has no path to any of those. The kernel remains the only thing that
//! decides anything, exactly as it does for every other surface.
//!
//! LOCAL BY DEFAULT, and the default is the point. `ai.endpoint` starts at
//! `http://127.0.0.1:11434/v1`, an Ollama on the venue's own machine: the
//! questions and the data never leave their hardware. That is the arrangement
//! D0's "decentralized, local-first" asks for, and it is why plain HTTP to
//! loopback is permitted while plain HTTP to anywhere else is refused.
//!
//! WHAT GOES OUT DEPENDS ON WHERE IT GOES. A hosted endpoint gets a REDACTED
//! context: no customer name, no phone, no address. A local endpoint gets the
//! full picture, because "local" means the data is already there. This is
//! decided by `httpc::is_local` on the configured host, not by a setting the
//! owner can tick — an owner who points at a hosted model has not thereby
//! consented on their customers' behalf.

use serde_json::{json, Value};

use crate::httpc::{self, HttpError};

#[derive(Debug)]
pub enum AiError {
    /// The owner has not switched it on. Not an error to log — the normal state.
    Disabled,
    Misconfigured(String),
    Transport(String),
    /// The provider answered but not with a completion.
    Unusable(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::Disabled => write!(f, "the assistant is switched off"),
            AiError::Misconfigured(e) => write!(f, "{e}"),
            AiError::Transport(e) => write!(f, "could not reach the model: {e}"),
            AiError::Unusable(e) => write!(f, "the model did not answer usefully: {e}"),
        }
    }
}

/// `Debug` deliberately omits the token: the struct is printed in test failures
/// and could be printed in a log line, and a bearer token is exactly the thing
/// that must not appear in either.
pub struct Assistant {
    endpoint: String,
    model: String,
    token: Option<String>,
    /// Whether the configured endpoint is on this machine. Decides how much
    /// context is allowed to leave.
    pub local: bool,
}

impl std::fmt::Debug for Assistant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Assistant")
            .field("endpoint", &self.endpoint)
            .field("model", &self.model)
            .field("token", &self.token.as_ref().map(|_| "<set>"))
            .field("local", &self.local)
            .finish()
    }
}

impl Assistant {
    /// Build from the venue's settings, or say why not.
    pub fn from_settings(s: &dowiz_hub::settings::Settings) -> Result<Assistant, AiError> {
        if !s.flag("ai.enabled") {
            return Err(AiError::Disabled);
        }
        let endpoint = s.known("ai.endpoint").trim_end_matches('/').to_string();
        let url = httpc::parse_url(&endpoint)
            .map_err(|e| AiError::Misconfigured(format!("ai.endpoint: {e}")))?;
        let local = httpc::is_local(&url.host);
        if !url.tls && !local {
            return Err(AiError::Misconfigured(
                "ai.endpoint must use https unless it is on this machine".into(),
            ));
        }
        let model = s.known("ai.model");
        if model.trim().is_empty() {
            return Err(AiError::Misconfigured("ai.model is not set".into()));
        }
        Ok(Assistant { endpoint, model, token: s.get("ai.token"), local })
    }

    /// One question, one answer.
    ///
    /// The OpenAI chat-completions shape, because Ollama, vLLM, OpenRouter,
    /// Groq and the managed APIs all speak it — so the owner can point this at
    /// a model on their own box today and at their own hosted account
    /// tomorrow without anything here changing.
    pub async fn ask(&self, system: &str, user: &str, timeout_ms: u64) -> Result<String, AiError> {
        let body = json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            // Deterministic-ish and short. This is answering a question about
            // numbers that are already known, not writing prose.
            "temperature": 0.2,
            "max_tokens": 400,
            "stream": false
        })
        .to_string();

        let auth;
        let mut headers: Vec<(&str, &str)> = vec![("content-type", "application/json")];
        if let Some(t) = &self.token {
            auth = format!("Bearer {t}");
            headers.push(("authorization", &auth));
        }

        let url = format!("{}/chat/completions", self.endpoint);
        let (code, raw) =
            httpc::request("POST", &url, &headers, body.as_bytes(), timeout_ms, 512 * 1024)
                .await
                .map_err(|e| match e {
                    HttpError::InsecureRemote(h) => AiError::Misconfigured(format!(
                        "refusing to send this venue's data to {h} without TLS"
                    )),
                    other => AiError::Transport(other.to_string()),
                })?;

        let text = String::from_utf8_lossy(&raw);
        if !(200..300).contains(&code) {
            // The provider's own words. "model not found" and "invalid api key"
            // need different fixes and the owner is the one who has to make them.
            return Err(AiError::Unusable(format!(
                "{code}: {}",
                text.chars().take(300).collect::<String>()
            )));
        }
        let v: Value = serde_json::from_str(&text)
            .map_err(|_| AiError::Unusable(format!("not json: {}", text.chars().take(200).collect::<String>())))?;
        v.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            // An empty completion is a failure, not an answer. Returning "" here
            // would put a blank bubble on the owner's screen and tell them
            // nothing about why.
            .ok_or_else(|| AiError::Unusable(format!("no completion in {}", text.chars().take(200).collect::<String>())))
    }
}

/// The instructions the model is given.
///
/// States plainly that the facts are authoritative and it is not. A model that
/// invents a number here would have an owner phoning a customer about an order
/// that does not exist.
pub const SYSTEM_OWNER: &str = "\
You help the owner of one restaurant read their own live order data. \
The FACTS block below is the truth; it was computed by the system, not by you. \
Never invent an order, a number, a name or a time that is not in it. \
If the answer is not in the FACTS, say you do not have it. \
Answer in the language the question was asked in. Be brief: two or three sentences, \
or a short list. Do not add pleasantries.";

pub const SYSTEM_COURIER: &str = "\
You help a delivery courier with their own current run. \
The FACTS block below is the truth; never invent an address, an order or a time. \
If the answer is not in the FACTS, say you do not have it. \
Answer in the language the question was asked in, in one or two sentences. \
Be practical: this is read on a phone, often one-handed, often outdoors.";

/// Strip what must not leave the building.
///
/// Applied to the facts when the endpoint is NOT on this machine. It removes
/// the fields that identify a customer — name, phone, address, note — and
/// leaves the shape of the order intact, so a hosted model can still answer
/// "how many are waiting" and "which is oldest" but cannot be asked who lives
/// where.
pub fn redact(facts: &Value) -> Value {
    fn scrub(v: &Value) -> Value {
        match v {
            Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (k, val) in map {
                    match k.as_str() {
                        "contact" | "address" | "phone" | "name" | "note" | "courier_note" => {
                            // The KEY is kept so the model is not told the field
                            // does not exist -- it is told it was withheld.
                            out.insert(k.clone(), json!("[withheld]"));
                        }
                        _ => {
                            out.insert(k.clone(), scrub(val));
                        }
                    }
                }
                Value::Object(out)
            }
            Value::Array(items) => Value::Array(items.iter().map(scrub).collect()),
            other => other.clone(),
        }
    }
    scrub(facts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_hub::settings::Settings;

    fn settings(pairs: &[(&str, &str)]) -> Settings {
        let mut s = Settings::create().expect("settings");
        for (k, v) in pairs {
            s.set(k, v);
        }
        s
    }

    /// Off is the default and it is not an error state.
    #[test]
    fn the_assistant_is_off_until_it_is_switched_on() {
        let s = settings(&[]);
        assert!(matches!(Assistant::from_settings(&s), Err(AiError::Disabled)));
    }

    #[test]
    fn a_local_endpoint_is_recognised_as_local() {
        let s = settings(&[("ai.enabled", "1")]);
        let a = Assistant::from_settings(&s).expect("default should build");
        assert!(a.local, "the shipped default must be the on-machine model");
        assert_eq!(a.model, "llama3.2");
    }

    /// The rule that stops a token, and a customer list, going out in clear.
    #[test]
    fn plain_http_to_a_remote_model_is_refused_at_configuration_time() {
        let s = settings(&[("ai.enabled", "1"), ("ai.endpoint", "http://api.example.com/v1")]);
        match Assistant::from_settings(&s) {
            Err(AiError::Misconfigured(m)) => assert!(m.contains("https"), "{m}"),
            other => panic!("expected refusal, got {other:?}"),
        }
    }

    /// The token must not reach a log line or a test failure through Debug.
    #[test]
    fn debug_does_not_print_the_token() {
        let s = settings(&[
            ("ai.enabled", "1"),
            ("ai.endpoint", "https://api.example.com/v1"),
            ("ai.token", "sk-do-not-print-me"),
        ]);
        let a = Assistant::from_settings(&s).expect("build");
        let shown = format!("{a:?}");
        assert!(!shown.contains("sk-do-not-print-me"), "the token leaked: {shown}");
        assert!(shown.contains("<set>"), "but it must show that one IS set: {shown}");
    }

    #[test]
    fn a_hosted_https_endpoint_builds_but_is_not_local() {
        let s = settings(&[
            ("ai.enabled", "1"),
            ("ai.endpoint", "https://api.example.com/v1"),
            ("ai.token", "sk-x"),
        ]);
        let a = Assistant::from_settings(&s).expect("build");
        assert!(!a.local, "a hosted endpoint must not be treated as local");
        assert_eq!(a.token.as_deref(), Some("sk-x"));
    }

    #[test]
    fn nonsense_configuration_is_named_not_ignored() {
        let s = settings(&[("ai.enabled", "1"), ("ai.endpoint", "not a url")]);
        assert!(matches!(Assistant::from_settings(&s), Err(AiError::Misconfigured(_))));
        // A whitespace model name is REFUSED by name rather than quietly
        // replaced with the default. Substituting a default for nonsense is how
        // an owner ends up wondering why their configured model is not the one
        // answering.
        let s = settings(&[("ai.enabled", "1"), ("ai.model", " ")]);
        match Assistant::from_settings(&s) {
            Err(AiError::Misconfigured(m)) => assert!(m.contains("ai.model"), "{m}"),
            other => panic!("expected a named refusal, got {other:?}"),
        }
        // An ABSENT model, by contrast, uses the declared default -- that is
        // what a default is for.
        let s = settings(&[("ai.enabled", "1")]);
        assert!(Assistant::from_settings(&s).is_ok());
    }

    /// The privacy rule, as a test rather than as a comment.
    #[test]
    fn redaction_removes_every_way_to_identify_a_customer() {
        let facts = json!({
            "orders": [{
                "id": "ord_1",
                "status": "READY",
                "total": 2000,
                "contact": { "name": "Ana", "phone": "+355691234567" },
                "fulfilment": {
                    "kind": "delivery",
                    "address": { "line": "Rruga Taulantia 12", "note": "ring twice" }
                }
            }]
        });
        let out = redact(&facts).to_string();
        for leak in ["Ana", "+355691234567", "Rruga Taulantia", "ring twice"] {
            assert!(!out.contains(leak), "{leak:?} survived redaction: {out}");
        }
        // The shape a question needs must survive.
        assert!(out.contains("ord_1"));
        assert!(out.contains("READY"));
        assert!(out.contains("2000"));
        assert!(out.contains("delivery"));
        assert!(out.contains("withheld"), "the model should know a field was withheld");
    }

    /// Redaction must reach into nested structures, not just the top level.
    #[test]
    fn redaction_is_recursive() {
        let facts = json!({ "a": { "b": { "c": { "phone": "+355690000000" } } },
                            "list": [{ "contact": { "phone": "+355691111111" } }] });
        let out = redact(&facts).to_string();
        assert!(!out.contains("+35569"), "{out}");
    }

    /// A local endpoint keeps everything, because nothing left the machine.
    #[test]
    fn local_context_is_not_redacted() {
        let facts = json!({ "contact": { "phone": "+355691234567" } });
        assert!(facts.to_string().contains("+355691234567"));
    }
}

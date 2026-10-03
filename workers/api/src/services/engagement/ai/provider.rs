//! PURE. WHICH MODEL ANSWERS, IN WHAT ORDER, AND WHY THE OTHERS DID NOT (W-AI row 1).
//!
//! Two places a venue's question can go, tried in this order:
//!
//! 1. THE OWNER'S OWN ENDPOINT -- any OpenAI-compatible `/v1` base (`ai.endpoint`,
//!    `ai.model`, the secret `ai.token`). OpenRouter's free models are the
//!    documented default the console fills in (`public/admin/ai-logic.js`
//!    `OPENROUTER`), with the owner's OWN key: the platform holds no OpenRouter
//!    key and never will (operator 2026-10-03: "це повинен налаштувати і
//!    добавляти кожен власник хабу сам").
//! 2. CLOUDFLARE WORKERS AI through the Worker's `[ai]` binding. Its free
//!    allowance (10,000 neurons a day) belongs to the WHOLE ACCOUNT, so every
//!    venue draws on a daily share (`budget.rs`) and a spent share refuses.
//!
//! Then "AI is off", with the reason in words an owner can act on. NOTHING IS
//! SENT ANYWHERE while `ai.enabled` is off -- the switch predates this file and
//! keeps its meaning. The deterministic answers (`query.rs`, `answer.rs`) need
//! no model and work with the switch off.
//!
//! THE KEY IS A [`Key`]: no `Display`, a `Debug` that prints dots, and
//! [`Key::scrub`] for every text that came back from a provider, because a
//! provider's error body is the one place a key could be echoed into an answer.

use dowiz_hub::settings::Settings;

/// The Worker's binding name (`wrangler.toml` `[ai] binding = "AI"`).
pub const BINDING: &str = "AI";

/// The Workers AI model: Qwen3 lists Albanian, Ukrainian and Russian
/// (research 2026-10-03 U6), and it is the cheapest per output token of the
/// catalogue's chat models that does (`budget.rs`).
pub const WORKERS_MODEL: &str = "@cf/qwen/qwen3-30b-a3b-fp8";

/// The setting that picks the order (`auto` | `own` | `workers`).
pub const MODE_KEY: &str = "ai.provider";

/// Which routes an owner allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Their endpoint when it is set, else Workers AI.
    Auto,
    /// Only their endpoint.
    Own,
    /// Only Workers AI.
    Workers,
}

impl Mode {
    pub fn of(word: &str) -> Option<Mode> {
        match word.trim() {
            "" | "auto" => Some(Mode::Auto),
            "own" => Some(Mode::Own),
            "workers" => Some(Mode::Workers),
            _ => None,
        }
    }
    pub fn word(self) -> &'static str {
        match self {
            Mode::Auto => "auto",
            Mode::Own => "own",
            Mode::Workers => "workers",
        }
    }
}

/// A settings write the console may make to `ai.provider`, checked where it is
/// written (`services::venue::settings::set_setting`).
pub fn validate(key: &str, value: &str) -> Result<(), String> {
    if key == MODE_KEY && Mode::of(value).is_none() {
        return Err(format!("{MODE_KEY} is auto, own or workers, not {value:?}"));
    }
    Ok(())
}

/// A secret. Never printed: read it with [`Key::expose`] at the one place
/// it is sent (the `authorization` header).
#[derive(Clone, PartialEq, Eq)]
pub struct Key(String);

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key(\u{2022}\u{2022}\u{2022}\u{2022})")
    }
}

impl Key {
    pub fn new(raw: &str) -> Option<Key> {
        let k = raw.trim();
        (!k.is_empty()).then(|| Key(k.to_string()))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
    /// `text` with every occurrence of the key replaced by dots.
    pub fn scrub(&self, text: &str) -> String {
        if self.0.len() < 4 {
            return text.to_string();
        }
        text.replace(&self.0, "\u{2022}\u{2022}\u{2022}\u{2022}")
    }
}

/// Where one attempt goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    Own { endpoint: String, model: String, key: Option<Key> },
    Workers { model: &'static str },
}

impl Route {
    pub fn name(&self) -> &'static str {
        match self {
            Route::Own { .. } => "own",
            Route::Workers { .. } => "workers-ai",
        }
    }
    pub fn model(&self) -> &str {
        match self {
            Route::Own { model, .. } => model,
            Route::Workers { model } => model,
        }
    }
}

/// Why a route was not tried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Off {
    /// `ai.enabled` is off: nothing is sent anywhere.
    Disabled,
    /// No endpoint (and so no key) of the owner's.
    NeedsKey,
    /// The endpoint is not https; a Worker cannot reach anything else.
    NotHttps,
    /// The Worker has no `[ai]` binding.
    NoBinding,
    /// This venue's share of today's account allowance is spent.
    Budget,
    /// The owner chose the other route only.
    NotChosen,
}

impl Off {
    pub fn code(self) -> &'static str {
        match self {
            Off::Disabled => "disabled",
            Off::NeedsKey => "needs-key",
            Off::NotHttps => "not-https",
            Off::NoBinding => "workers-ai-unavailable",
            Off::Budget => "budget-spent",
            Off::NotChosen => "not-chosen",
        }
    }
}

/// The venue's AI configuration, read once.
#[derive(Clone, Debug)]
pub struct Cfg {
    pub enabled: bool,
    pub mode: Mode,
    pub endpoint: String,
    pub model: String,
    pub key: Option<Key>,
}

impl Cfg {
    pub fn of(s: &Settings) -> Cfg {
        Cfg {
            enabled: s.flag("ai.enabled"),
            // An unreadable word is the safe default, and the write path refuses one.
            mode: Mode::of(&s.known(MODE_KEY)).unwrap_or(Mode::Auto),
            endpoint: s.known("ai.endpoint").trim().trim_end_matches('/').to_string(),
            model: s.known("ai.model").trim().to_string(),
            key: s.get("ai.token").as_deref().and_then(Key::new),
        }
    }
}

/// The routes to try, in order, and why each other one is not tried.
#[derive(Debug, Default)]
pub struct Plan {
    pub routes: Vec<Route>,
    pub skipped: Vec<(&'static str, Off)>,
}

/// `binding`: the Worker has `[ai]`. `budget_left`: this venue can still
/// afford one Workers AI call today.
pub fn plan(cfg: &Cfg, binding: bool, budget_left: bool) -> Plan {
    let mut p = Plan::default();
    if !cfg.enabled {
        p.skipped.push(("all", Off::Disabled));
        return p;
    }
    if cfg.mode == Mode::Workers {
        p.skipped.push(("own", Off::NotChosen));
    } else if cfg.endpoint.is_empty() {
        p.skipped.push(("own", Off::NeedsKey));
    } else if !cfg.endpoint.starts_with("https://") {
        p.skipped.push(("own", Off::NotHttps));
    } else {
        p.routes.push(Route::Own { endpoint: cfg.endpoint.clone(), model: cfg.model.clone(), key: cfg.key.clone() });
    }
    if cfg.mode == Mode::Own {
        p.skipped.push(("workers-ai", Off::NotChosen));
    } else if !binding {
        p.skipped.push(("workers-ai", Off::NoBinding));
    } else if !budget_left {
        p.skipped.push(("workers-ai", Off::Budget));
    } else {
        p.routes.push(Route::Workers { model: WORKERS_MODEL });
    }
    p
}

/// A model's text without a reasoning model's `<think>` block.
pub fn without_thinking(text: &str) -> String {
    let mut out = text.to_string();
    while let (Some(a), Some(b)) = (out.find("<think>"), out.find("</think>")) {
        if b < a {
            break;
        }
        out.replace_range(a..b + "</think>".len(), "");
    }
    // Cut off while still thinking: there is no answer in it.
    if let Some(a) = out.find("<think>") {
        out.truncate(a);
    }
    out.trim().to_string()
}

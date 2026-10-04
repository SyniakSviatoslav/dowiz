//! THE VENUE'S SMS SETTINGS: the keys, the masked view, the owner's edit. PURE.
//!
//! NOT DECLARED SETTINGS (`dowiz_hub::settings::KNOWN`): written only through
//! `POST /api/owner/sms`, which checks the whole configuration at once (a
//! provider with no key is refused while the owner can still fix it), as the
//! e-bills keys are. The secret's last segment is `secret`, so
//! `settings::is_secret` redacts it in every generic read as well.
//!
//! NEVER A PLATFORM KEY: every credential here is the venue's own -- its own
//! phone's gateway login, or its own provider account.

use serde::Deserialize;
use serde_json::{json, Value};

pub const KEY_ON: &str = "notify.sms.on";
pub const KEY_PROVIDER: &str = "notify.sms.provider";
pub const KEY_URL: &str = "notify.sms.url";
pub const KEY_USER: &str = "notify.sms.user";
pub const KEY_SECRET: &str = "notify.sms.secret";
pub const KEY_FROM: &str = "notify.sms.from";
pub const KEY_DAILY: &str = "notify.sms.daily";
pub const KEYS: [&str; 7] = [KEY_ON, KEY_PROVIDER, KEY_URL, KEY_USER, KEY_SECRET, KEY_FROM, KEY_DAILY];

/// The venue's own Android phone, through SMSGate's free cloud relay.
pub const SMSGATE_URL: &str = "https://api.sms-gate.app/3rdparty/v1/messages";
/// textbee's cloud (MIT app on the venue's phone; free plan 50/day, 300/month).
pub const TEXTBEE_URL: &str = "https://api.textbee.dev/api/v1/gateway/send-sms";
/// Twilio's REST base; the account SID completes it. Paid, the owner's own account.
pub const TWILIO_BASE: &str = "https://api.twilio.com/2010-04-01/Accounts";

/// A day's SMS when the owner set none: order-status volume, not marketing,
/// and far below Android's own 30-per-minute prompt (`SmsUsageMonitor`).
pub const DAILY_DEFAULT: u32 = 60;
/// The most an owner may allow. A consumer SIM sending more is a SIM the
/// carrier blocks for bulk traffic.
pub const DAILY_MAX: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    SmsGate,
    TextBee,
    Twilio,
}

impl Provider {
    pub fn of(s: &str) -> Option<Provider> {
        match s {
            "smsgate" => Some(Provider::SmsGate),
            "textbee" => Some(Provider::TextBee),
            "twilio" => Some(Provider::Twilio),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::SmsGate => "smsgate",
            Provider::TextBee => "textbee",
            Provider::Twilio => "twilio",
        }
    }
    /// The URL when the owner gave none (Twilio's is built from the SID).
    pub fn default_url(self) -> &'static str {
        match self {
            Provider::SmsGate => SMSGATE_URL,
            Provider::TextBee => TEXTBEE_URL,
            Provider::Twilio => TWILIO_BASE,
        }
    }
}

/// A configuration the drain can send with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfg {
    pub provider: Provider,
    pub url: String,
    pub user: String,
    pub secret: String,
    pub from: String,
    pub daily: u32,
}

/// What a settings read gives: the stored value or empty.
pub trait Read {
    fn val(&self, key: &str) -> String;
}

impl Read for dowiz_hub::settings::Settings {
    fn val(&self, key: &str) -> String {
        self.get(key).unwrap_or_default().trim().to_string()
    }
}

/// The venue's daily budget, as stored or the default.
pub fn daily_of(s: &impl Read) -> u32 {
    s.val(KEY_DAILY).parse::<u32>().ok().filter(|d| (1..=DAILY_MAX).contains(d)).unwrap_or(DAILY_DEFAULT)
}

/// Is SMS switched on at this venue? (The order turn asks only this.)
pub fn is_on(s: &impl Read) -> bool {
    matches!(s.val(KEY_ON).as_str(), "on" | "1" | "true" | "yes")
}

/// The configuration, when it is switched on AND complete; `Err` says what is missing.
pub fn of(s: &impl Read) -> Result<Cfg, &'static str> {
    if !is_on(s) {
        return Err("sms_off");
    }
    of_any(s)
}

/// The configuration whether or not it is switched on: the console's Test
/// button proves a gateway before the owner turns customer texts on.
pub fn of_any(s: &impl Read) -> Result<Cfg, &'static str> {
    let provider = Provider::of(&s.val(KEY_PROVIDER)).unwrap_or(Provider::SmsGate);
    let url = Some(s.val(KEY_URL)).filter(|u| !u.is_empty()).unwrap_or_else(|| provider.default_url().to_string());
    let cfg = Cfg { provider, url, user: s.val(KEY_USER), secret: s.val(KEY_SECRET), from: s.val(KEY_FROM), daily: daily_of(s) };
    complete(&cfg)?;
    Ok(cfg)
}

/// What each provider needs. The words are console keys (`sms_missing_*`).
pub fn complete(c: &Cfg) -> Result<(), &'static str> {
    if c.secret.is_empty() {
        return Err("sms_missing_secret");
    }
    match c.provider {
        Provider::SmsGate | Provider::Twilio if c.user.is_empty() => Err("sms_missing_user"),
        // The Account SID is a PATH segment of Twilio's URL: letters and digits only.
        Provider::Twilio if !c.user.bytes().all(|b| b.is_ascii_alphanumeric()) => Err("sms_missing_user"),
        Provider::Twilio if c.from.is_empty() => Err("sms_missing_from"),
        _ => Ok(()),
    }
}

/// The console's view: the secret is "set" or not, never shown.
pub fn view(s: &impl Read) -> Value {
    let provider = Provider::of(&s.val(KEY_PROVIDER)).unwrap_or(Provider::SmsGate);
    json!({
        "on": is_on(s),
        "provider": provider.as_str(),
        "url": s.val(KEY_URL),
        "default_url": provider.default_url(),
        "user": s.val(KEY_USER),
        "secret_set": !s.val(KEY_SECRET).is_empty(),
        "from": s.val(KEY_FROM),
        "daily": daily_of(s),
        "missing": of(s).err().filter(|w| *w != "sms_off"),
    })
}

/// The owner's edit. A CLOSED shape; an absent secret keeps the stored one.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct CfgIn {
    pub on: bool,
    pub provider: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    /// Typed only when changing it; `""` forgets it.
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub daily: Option<u32>,
}

/// The settings writes an edit makes (`""` clears a key), or why it is refused.
/// `stored_secret` says whether a secret is already kept, so switching on with
/// a kept secret is not refused for the secret the owner cannot see.
pub fn edit(body: &CfgIn, stored_secret: bool) -> Result<Vec<(&'static str, String)>, String> {
    let provider = Provider::of(body.provider.trim()).ok_or_else(|| format!("unknown SMS provider {:?}", body.provider))?;
    let url = body.url.as_deref().unwrap_or("").trim().to_string();
    // HTTPS ONLY, and a host: a Worker has no loopback, and plain http would
    // carry the venue's gateway password in the clear.
    if !url.is_empty() {
        let host = url.strip_prefix("https://").map(|r| r.split(['/', '?', '#']).next().unwrap_or("")).unwrap_or("");
        if host.is_empty() || host.contains('@') || url.len() > 300 {
            return Err("the gateway address must be https://<host>/...".into());
        }
        if provider == Provider::Twilio {
            return Err("Twilio's address is fixed; leave the address empty".into());
        }
    }
    let user = body.user.as_deref().unwrap_or("").trim().to_string();
    let from = body.from.as_deref().unwrap_or("").trim().to_string();
    if !from.is_empty() && super::phone::e164(&from, "").is_none() {
        return Err("the sender number must be international, +355...".into());
    }
    let daily = body.daily.unwrap_or(DAILY_DEFAULT);
    if !(1..=DAILY_MAX).contains(&daily) {
        return Err(format!("the daily limit must be 1..{DAILY_MAX}"));
    }
    let secret = body.secret.as_deref().map(str::trim);
    if body.on {
        let has_secret = match secret {
            Some(s) => !s.is_empty(),
            None => stored_secret,
        };
        let probe = Cfg { provider, url: url.clone(), user: user.clone(), secret: if has_secret { "x".into() } else { String::new() }, from: from.clone(), daily };
        complete(&probe).map_err(str::to_string)?;
    }
    let mut out = vec![
        (KEY_ON, if body.on { "on".to_string() } else { String::new() }),
        (KEY_PROVIDER, provider.as_str().to_string()),
        (KEY_URL, url),
        (KEY_USER, user),
        (KEY_FROM, from),
        (KEY_DAILY, daily.to_string()),
    ];
    if let Some(s) = secret {
        if s.len() > 512 {
            return Err("the key is too long".into());
        }
        out.push((KEY_SECRET, s.to_string()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests;

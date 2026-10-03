//! PURE. ONE VENUE'S SHARE OF THE ACCOUNT'S FREE WORKERS AI ALLOWANCE (W-AI row 1).
//!
//! Cloudflare gives 10,000 neurons a day to the ACCOUNT, reset at 00:00 UTC,
//! and every venue on the platform draws on the same pool
//! (https://developers.cloudflare.com/workers-ai/platform/pricing/, read
//! 2026-10-03). One busy venue must not spend everybody's day, so each venue
//! has a daily cap: `AI_VENUE_NEURONS` (a Worker var) or [`DEFAULT_VENUE_DAILY`].
//!
//! WHAT A CALL COSTS is the pricing page's per-model rate, here for
//! `@cf/qwen/qwen3-30b-a3b-fp8`: 4,625 neurons per million input tokens and
//! 30,475 per million output tokens. A call is admitted on its WORST case
//! (the prompt's estimated tokens plus every output token it may produce) and
//! charged what the model reports in `usage`, or the worst case when it reports
//! nothing: a meter that undercounts would let the account run dry first.
//!
//! WHAT IS STORED: one hub-written setting, `ai.spent` = `<utc day>:<neurons>`.
//! A different day reads as 0, so the share renews itself with no job. It is
//! not a declared key, so `POST /api/owner/settings` cannot reset it.

/// The account's free allowance per UTC day.
pub const ACCOUNT_DAILY: i64 = 10_000;
/// One venue's share when the Worker sets none: 10,000 over about 30 venues,
/// rounded down (research 2026-10-03 P9, a GUESS to revisit with real use).
pub const DEFAULT_VENUE_DAILY: i64 = 300;
/// Neurons per million tokens, `@cf/qwen/qwen3-30b-a3b-fp8`.
pub const IN_PER_M: i64 = 4_625;
pub const OUT_PER_M: i64 = 30_475;
/// The hub-written setting that holds today's spend.
pub const SPENT_KEY: &str = "ai.spent";
/// The Worker var that sets the share.
pub const CAP_VAR: &str = "AI_VENUE_NEURONS";

const DAY_MS: i64 = 86_400_000;

/// The UTC day number of an instant: Workers AI's allowance resets at 00:00 UTC.
pub fn utc_day(now_ms: i64) -> i64 {
    now_ms.div_euclid(DAY_MS)
}

/// Neurons for a call, rounded UP: a fraction of a neuron is still spent.
pub fn neurons(tokens_in: i64, tokens_out: i64) -> i64 {
    let micro = tokens_in.max(0) * IN_PER_M + tokens_out.max(0) * OUT_PER_M;
    (micro + 999_999) / 1_000_000
}

/// Tokens in a text, estimated HIGH: one per three bytes. Albanian and
/// Cyrillic take more bytes per letter than English, and more tokens too.
pub fn tokens_of(text: &str) -> i64 {
    (text.len() as i64 + 2) / 3
}

/// The worst case of one call: the whole prompt and every output token allowed.
pub fn worst_case(prompt: &str, max_tokens: i64) -> i64 {
    neurons(tokens_of(prompt), max_tokens)
}

/// Today's spend from the stored value. Another day, or a value that does not
/// read, is 0 -- except that a value from the FUTURE (a clock that went
/// back) keeps its count, so turning a clock back cannot refill the share.
pub fn spent_today(raw: Option<&str>, day: i64) -> i64 {
    let Some((d, n)) = raw.and_then(|r| r.split_once(':')) else { return 0 };
    match (d.trim().parse::<i64>(), n.trim().parse::<i64>()) {
        (Ok(d), Ok(n)) if d >= day => n.max(0),
        _ => 0,
    }
}

/// The value to store after spending `add` more today.
pub fn record(raw: Option<&str>, day: i64, add: i64) -> String {
    format!("{day}:{}", spent_today(raw, day).saturating_add(add.max(0)))
}

/// The share from the Worker var: a whole number from 1 to the account's
/// allowance, else the default.
pub fn cap_of(var: Option<&str>) -> i64 {
    var.and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| (1..=ACCOUNT_DAILY).contains(n))
        .unwrap_or(DEFAULT_VENUE_DAILY)
}

/// Today's meter, as the console draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meter {
    pub day: i64,
    pub used: i64,
    pub cap: i64,
}

impl Meter {
    pub fn of(raw: Option<&str>, now_ms: i64, cap: i64) -> Meter {
        let day = utc_day(now_ms);
        Meter { day, used: spent_today(raw, day), cap }
    }
    pub fn left(&self) -> i64 {
        (self.cap - self.used).max(0)
    }
    /// A call whose worst case still fits.
    pub fn admits(&self, worst: i64) -> bool {
        self.used.saturating_add(worst) <= self.cap
    }
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "used": self.used, "cap": self.cap, "left": self.left(),
            "account": ACCOUNT_DAILY, "resetsAtUtcDay": self.day + 1,
        })
    }
}

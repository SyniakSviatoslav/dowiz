//! Bebop brand — Warm Cosmo-Noir narration for the CLI, harmonized to the Cowboy Bebop spaceship.
//!
//! Ground truth: the ship's main hull/signal color is the brand token `--teal #46B0A4`
//! (docs/design/dowiz-brand/BRAND-BIBLE.md §3 "Bebop teal — success / alive / data-signal"), on the
//! warm-noir field `--void #12100E` / `--hull #1A1E1F` with `--bone #F2E9DB` text. We reuse those
//! EXACT hexes so the CLI and the product share one signal color. No new palette invented.

/// Main CLI color — the Cowboy Bebop spaceship's teal signal. `0x46B0A4`.
pub const SHIP_TEAL: &str = "#46B0A4";
/// Deeper teal for pressed/hover states (`--teal-deep #3EA094`).
pub const SHIP_TEAL_DEEP: &str = "#3EA094";
/// Warm near-black base canvas (`--void #12100E`).
pub const VOID: &str = "#12100E";
/// Teal-tinted charcoal raised surface (`--hull #1A1E1F`).
pub const HULL: &str = "#1A1E1F";
/// Primary text (`--bone #F2E9DB`).
pub const BONE: &str = "#F2E9DB";
/// Brand amber, used sparingly per the 90/10 law (`--amber #E8A544`).
pub const AMBER: &str = "#E8A544";
/// Danger — always paired with a label, never color-only (WCAG 1.4.1, `--blood #E0543E`).
pub const BLOOD: &str = "#E0543E";

/// The ship mark — a single saturated accent (the 90/10 law: one meaningful color per view).
pub const SHIP: &str = "◈"; // ◈ — cold teal diamond, the machine's eye

/// The tagline — north star, operator-coined 2026-07-07 (BRAND-BIBLE §0).
pub const TAGLINE: &str = "Hybrid is a feature, not a bug.";

/// Per-state microcopy, canonical EN (BRAND-BIBLE §10). Dry co-pilot voice.
/// Law 7 (THE HARD RULE): money/auth/security copy stays plain — no wit.
pub enum Tone {
    /// Brand moment — dry wit allowed.
    Brand,
    /// Money/auth/security path — plain, zero joke.
    Plain,
}

pub struct Line {
    pub text: String,
    pub tone: Tone,
}

pub fn boot_link() -> &'static str {
    "Link established. Let us get your kitchen off the leash."
}

pub fn ready() -> &'static str {
    "Bebop online. The ship is yours."
}

pub fn idle() -> &'static str {
    "Quiet night. Nothing on the pass yet."
}

pub fn say(key: &str) -> Line {
    match key {
        // brand moments — full dry wit
        "empty.orders" => Line { text: "Quiet night. Nothing on the pass yet.".into(), tone: Tone::Brand },
        "empty.menu" => Line { text: "The menu's empty. Even the void needs a starter.".into(), tone: Tone::Brand },
        "save.success" => Line { text: "Saved. Back to work.".into(), tone: Tone::Brand },
        "loading" => Line { text: "Working. The machine doesn't rush — neither should you.".into(), tone: Tone::Brand },
        "generic.error" => Line { text: "Something broke. Not your fault this time — probably. We are on it.".into(), tone: Tone::Brand },
        "offline" => Line { text: "Connection's gone. Orders are queued. They'll survive; we built them to.".into(), tone: Tone::Brand },
        "404" => Line { text: "This page doesn't exist. Neither did half the promises other platforms made you.".into(), tone: Tone::Brand },
        "sacred.footer" => Line { text: "Built with devotion. Held together by spite. Yours, not ours.".into(), tone: Tone::Brand },
        // money / auth / security — PLAIN, zero wit (law 7)
        "payment.failed" => Line { text: "Payment didn't go through. Your card wasn't charged. Try again or use another card.".into(), tone: Tone::Plain },
        "refund.issued" => Line { text: "Refund sent. It may take 3–5 business days to appear.".into(), tone: Tone::Plain },
        "auth.failed" => Line { text: "Wrong email or password. Try again.".into(), tone: Tone::Plain },
        "session.expired" => Line { text: "You've been signed out. Sign in to continue.".into(), tone: Tone::Plain },
        "destructive.confirm" => Line { text: "This deletes it for good. No undo. Confirm you know what you are doing.".into(), tone: Tone::Plain },
        // agent-specific
        "guard.redline" => Line { text: "This edit is behind a red-line. I will not make it without your explicit go-ahead. State it plainly.".into(), tone: Tone::Plain },
        "guard.scope" => Line { text: "That file is outside the agreed scope. Update the scope or pick a different target.".into(), tone: Tone::Plain },
        "guard.falsegreen" => Line { text: "A guardrail read green but could not fail. I will not call this done until it goes red on bad input.".into(), tone: Tone::Plain },
        "tool.denied" => Line { text: "Blocked by an invariant. The machine refuses to lie to you.".into(), tone: Tone::Plain },
        _ => Line { text: key.into(), tone: Tone::Brand },
    }
}

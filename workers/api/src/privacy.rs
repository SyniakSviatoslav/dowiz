//! Privacy under Albania's Law 124/2024 and the GDPR (Wave P of
//! `ROADMAP-2026-09-22`, `BLUEPRINT-GDPR-AND-MCP-2026-09-24`).
//!
//! * `registry` — every store, recipient and browser key that can hold
//!   something about a person, as a table the gate checks (P1).
//! * `notice` — the privacy notice each venue serves at `/privacy`, rendered
//!   from the registry and the venue's own settings (P8).
//! * `dpa` — the dowiz <-> venue data processing agreement: its text, its
//!   version, and the acceptance stored on the venue's `loc` record (P9).

pub mod dpa;
pub mod notice;
pub mod registry;

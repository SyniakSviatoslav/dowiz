//! Hub modules (STRUCTURE-UPGRADE Part A). Each subdirectory is an ATOMIC hub module with its own
//! `module.toml` manifest, enforced by `scripts/module-integrity.mjs`: a module depends on the core
//! + ports, NEVER on another module's internals (any undeclared `use crate::modules::<other>` reds
//! the gate). GRAND-PLAN 1.x hub features land here under the A5 placement rule.
pub mod channel_attribution;

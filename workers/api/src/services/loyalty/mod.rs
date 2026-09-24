//! THE STAMP CARD, and nothing else (§3.5 and §6 item 7 of
//! BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22; card C5, operator 2026-09-24).
//!
//! WHAT IT IS. A fold over the venue's orders: every DELIVERED or PICKED_UP
//! order of one person, and every sitting of theirs that was PAID (one stamp
//! per sitting, however many rounds it had), since their last redemption, mod
//! N. When the count reaches N the NEXT placement takes the venue's `Fixed`
//! reward off through `dowiz_hub::promo`, and the order that took it records
//! it (`loyalty` on the envelope), which is what restarts the fold.
//!
//! WHAT IT IS NOT. No stored counter anywhere (G2, `tools/gates/record.sh`);
//! no expiry, no levels, no points, no streaks, no "almost there", and NO
//! MESSAGE about it is sent to anyone: the only place it is shown is the
//! customer's own order page. A number a person holds and spends is a
//! balance; the day it decides how staff treat them it becomes a rating, and
//! `tools/gates/no-scoring.sh` refuses that.
//!
//! ONE PERSON, SPELLED TWO WAYS, IS ONE CARD: the customer key goes through
//! the C4 resolver (`customers::alias::Aliases::circle`).

pub mod handlers;
pub mod stamps;

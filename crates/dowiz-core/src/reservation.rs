//! Reservation domain — the table-booking `decide/fold` Law.
//!
//! dowiz's kernel knew orders and not tables: the Book a Table screens in the
//! product had no domain behind them at all. This module is that domain, built
//! to the same rules as [`crate::order_machine`]:
//!
//! * a state machine whose forbidden transitions are **errors, not no-ops**;
//! * `decide → Event`, then `state = fold(events)` — a reservation's state is
//!   never written, it is replayed;
//! * no clock, no RNG, no network, no float in the decision path (MANIFESTO C2),
//!   so every node replays a reservation identically offline. `now_min` is an
//!   argument, never a reading;
//! * the adjacency is built once at compile time from `allowed_next`, and a
//!   debug assertion cross-checks the two derivations on every transition — the
//!   same falsifier the order FSM carries.
//!
//! # What a reservation is NOT
//!
//! It is not a score and it does not rank anybody. A no-show is recorded against
//! the RESERVATION, never against the guest: `DECISIONS.md` D0 makes trust a
//! signed capability, and a "reliability rating" for guests would be the same
//! forbidden thing as a courier score wearing a different hat.
//!
//! It also holds no money. A deposit, if a venue ever wants one, is a ledger
//! posting in [`crate::ledger_account`] referencing this reservation's id — the
//! reservation itself stays a promise about a table.

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

// ─────────────────────────────────────────────────────────────────────────────
// States
// ─────────────────────────────────────────────────────────────────────────────

/// The lifecycle of one table booking.
///
/// Deliberately NOT `Ord`/`PartialOrd`: ordering these would make "a better
/// state" expressible, and the routing enums in `crate::domain` omit the same
/// traits for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReservationStatus {
    /// The guest asked. Nothing is held yet.
    Requested,
    /// The venue accepted; a table is held for the slot.
    Confirmed,
    /// The party arrived and was seated.
    Seated,
    /// They ate and left. Terminal.
    Completed,
    /// The venue refused the request. Terminal.
    Declined,
    /// The guest cancelled. Terminal.
    CancelledByGuest,
    /// The venue cancelled a booking -- one it had confirmed, or a request it
    /// can no longer honour. Terminal.
    CancelledByVenue,
    /// The slot passed with nobody seated. Terminal.
    NoShow,
}

/// Every state, in a fixed order. The index into this slice is the bit position
/// used by [`FSM_ADJ`], so **the order is load-bearing** — appending is safe,
/// reordering is not.
pub const LIFECYCLE_STATES: [ReservationStatus; 8] = [
    ReservationStatus::Requested,
    ReservationStatus::Confirmed,
    ReservationStatus::Seated,
    ReservationStatus::Completed,
    ReservationStatus::Declined,
    ReservationStatus::CancelledByGuest,
    ReservationStatus::CancelledByVenue,
    ReservationStatus::NoShow,
];

impl ReservationStatus {
    /// Parse the wire form. An unknown string is rejected, never mapped to a
    /// default — a booking in an unrecognised state must stop the replay rather
    /// than silently become `Requested`.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "REQUESTED" => Some(Self::Requested),
            "CONFIRMED" => Some(Self::Confirmed),
            "SEATED" => Some(Self::Seated),
            "COMPLETED" => Some(Self::Completed),
            "DECLINED" => Some(Self::Declined),
            "CANCELLED_BY_GUEST" => Some(Self::CancelledByGuest),
            "CANCELLED_BY_VENUE" => Some(Self::CancelledByVenue),
            "NO_SHOW" => Some(Self::NoShow),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Requested => "REQUESTED",
            Self::Confirmed => "CONFIRMED",
            Self::Seated => "SEATED",
            Self::Completed => "COMPLETED",
            Self::Declined => "DECLINED",
            Self::CancelledByGuest => "CANCELLED_BY_GUEST",
            Self::CancelledByVenue => "CANCELLED_BY_VENUE",
            Self::NoShow => "NO_SHOW",
        }
    }

    /// A terminal state has no outgoing transition. Checked against
    /// `allowed_next` by a test, so the two cannot drift.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::Declined
                | Self::CancelledByGuest
                | Self::CancelledByVenue
                | Self::NoShow
        )
    }

    /// True once the venue has committed a table. Past this line a cancellation
    /// costs the venue a seating, which is why `NoShow` only exists on this
    /// side of it. (`CancelledByVenue` is reachable from a request too: the
    /// venue withdrawing what it never promised costs nobody a seating.)
    pub fn is_committed(&self) -> bool {
        matches!(self, Self::Confirmed | Self::Seated)
    }
}

/// The single source of truth for legal transitions.
pub fn allowed_next(from: ReservationStatus) -> &'static [ReservationStatus] {
    use ReservationStatus::*;
    match from {
        // Before a table is held, either side may walk away. The venue has
        // TWO ways to: `Declined` answers the request ("we cannot take it"),
        // `CancelledByVenue` withdraws an unanswered one the venue can no
        // longer honour (a closure, a private event) -- the owner console's
        // one "cancel" button means the same thing on every live booking,
        // and a 409 on the booking the venue has not yet answered was a
        // refusal nobody could explain to the guest.
        Requested => &[Confirmed, Declined, CancelledByGuest, CancelledByVenue],
        // A held table can be taken up, released by either side, or missed.
        Confirmed => &[Seated, CancelledByGuest, CancelledByVenue, NoShow],
        // Once the party is at the table the only way out is through.
        // A venue that must end a seating early does it as an order-side
        // refund, not by un-seating people who are already sitting down.
        Seated => &[Completed],
        Completed | Declined | CancelledByGuest | CancelledByVenue | NoShow => &[],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Compile-time adjacency — the second, independent derivation
// ─────────────────────────────────────────────────────────────────────────────

const fn idx_of(s: ReservationStatus) -> usize {
    match s {
        ReservationStatus::Requested => 0,
        ReservationStatus::Confirmed => 1,
        ReservationStatus::Seated => 2,
        ReservationStatus::Completed => 3,
        ReservationStatus::Declined => 4,
        ReservationStatus::CancelledByGuest => 5,
        ReservationStatus::CancelledByVenue => 6,
        ReservationStatus::NoShow => 7,
    }
}

const fn build_adjacency() -> [u8; 8] {
    let mut adj = [0u8; 8];
    let mut i = 0;
    while i < LIFECYCLE_STATES.len() {
        let from = LIFECYCLE_STATES[i];
        let outs = allowed_next_const(from);
        let mut j = 0;
        while j < outs.len() {
            adj[i] |= 1u8 << idx_of(outs[j]);
            j += 1;
        }
        i += 1;
    }
    adj
}

/// `allowed_next` again, as a `const fn` so the adjacency can be built at
/// compile time. Kept byte-for-byte in step with the runtime version by
/// [`tests::adjacency_agrees_with_allowed_next`], which is the whole point of
/// having two.
const fn allowed_next_const(from: ReservationStatus) -> &'static [ReservationStatus] {
    use ReservationStatus::*;
    match from {
        Requested => &[Confirmed, Declined, CancelledByGuest, CancelledByVenue],
        Confirmed => &[Seated, CancelledByGuest, CancelledByVenue, NoShow],
        Seated => &[Completed],
        Completed | Declined | CancelledByGuest | CancelledByVenue | NoShow => &[],
    }
}

/// `FSM_ADJ[i]` is a bitmask over the 8 states: bit `j` is set iff
/// `LIFECYCLE_STATES[i] → LIFECYCLE_STATES[j]` is legal.
pub const FSM_ADJ: [u8; 8] = build_adjacency();

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationError {
    /// `from == to`. A repeat of the state a booking is already in is a bug in
    /// the caller, not a no-op.
    SameStatus(ReservationStatus),
    /// The transition is not in `allowed_next`.
    Illegal(ReservationStatus, ReservationStatus),
    /// The party size is outside what a table booking can mean.
    PartySize(u16),
    /// The slot is in the past relative to the `now_min` the caller supplied.
    SlotInPast { slot_min: i64, now_min: i64 },
    /// The slot is further out than the venue books.
    SlotTooFar { slot_min: i64, horizon_min: i64 },
    /// Arithmetic on the slot overflowed. Fail closed rather than wrap.
    SlotOverflow,
}

impl ReservationError {
    pub fn message(&self) -> String {
        match self {
            Self::SameStatus(s) => {
                let mut m = String::from("reservation already in status ");
                m.push_str(s.as_str());
                m
            }
            Self::Illegal(a, b) => {
                let mut m = String::from("illegal reservation transition ");
                m.push_str(a.as_str());
                m.push_str(" -> ");
                m.push_str(b.as_str());
                m
            }
            Self::PartySize(n) => {
                let mut m = String::from("party size out of range: ");
                m.push_str(&n.to_string());
                m
            }
            Self::SlotInPast { slot_min, now_min } => {
                let mut m = String::from("slot ");
                m.push_str(&slot_min.to_string());
                m.push_str(" is before now ");
                m.push_str(&now_min.to_string());
                m
            }
            Self::SlotTooFar {
                slot_min,
                horizon_min,
            } => {
                let mut m = String::from("slot ");
                m.push_str(&slot_min.to_string());
                m.push_str(" is beyond the booking horizon ");
                m.push_str(&horizon_min.to_string());
                m
            }
            Self::SlotOverflow => String::from("slot arithmetic overflowed"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The Law
// ─────────────────────────────────────────────────────────────────────────────

/// Validate one transition. Forbidden transitions are errors.
pub fn assert_transition(
    from: ReservationStatus,
    to: ReservationStatus,
) -> Result<(), ReservationError> {
    if from == to {
        return Err(ReservationError::SameStatus(from));
    }
    let ok_slice = allowed_next(from).contains(&to);
    // The same differential check the order FSM runs: the runtime slice scan
    // against the compile-time bitmask. They come from two separate functions,
    // so a bug in `idx_of` or `build_adjacency` shows up here rather than as a
    // booking that quietly accepts an impossible move. Compiled out of release.
    debug_assert_eq!(
        ok_slice,
        (FSM_ADJ[idx_of(from)] & (1u8 << idx_of(to))) != 0,
        "reservation FSM diverged: allowed_next vs FSM_ADJ disagree"
    );
    if !ok_slice {
        return Err(ReservationError::Illegal(from, to));
    }
    Ok(())
}

/// Replay a sequence of transitions. Stops at the first illegal one and returns
/// both the error and the state actually reached, so a caller can say where the
/// replay stopped instead of only that it did.
pub fn fold_transitions(
    start: ReservationStatus,
    steps: &[ReservationStatus],
) -> Result<ReservationStatus, (ReservationError, ReservationStatus)> {
    let mut cur = start;
    for &next in steps {
        assert_transition(cur, next).map_err(|e| (e, cur))?;
        cur = next;
    }
    Ok(cur)
}

// ─────────────────────────────────────────────────────────────────────────────
// The request
// ─────────────────────────────────────────────────────────────────────────────

/// What a venue is willing to take. Supplied by the venue, never assumed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookingPolicy {
    /// Smallest party the venue seats.
    pub min_party: u16,
    /// Largest party the venue seats at one table.
    pub max_party: u16,
    /// How far ahead bookings are taken, in minutes.
    pub horizon_min: i64,
    /// How long before the slot a guest may still cancel without it counting as
    /// a no-show, in minutes.
    pub free_cancel_min: i64,
}

impl BookingPolicy {
    /// A policy a venue can start from. Every number here is a default the venue
    /// overrides, not a rule of the system.
    pub const fn default_policy() -> Self {
        Self {
            min_party: 1,
            max_party: 20,
            horizon_min: 60 * 24 * 60, // sixty days
            free_cancel_min: 120,
        }
    }
}

/// A booking request, before the venue has answered.
///
/// `slot_min` is minutes since the Unix epoch — an integer, because a booking
/// time is a point a venue and a guest must agree on exactly and a float would
/// let those two agreements differ in the last bit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationRequest {
    pub id: u64,
    pub venue: String,
    pub party: u16,
    pub slot_min: i64,
    /// Free text from the guest ("Birthday"). Never interpreted, only carried.
    pub occasion: String,
}

/// Check a request against a venue's policy at a caller-supplied `now_min`.
///
/// This is the `decide` half: it answers whether the request MAY become a
/// reservation, and it reads no clock of its own so two nodes replaying the same
/// request with the same `now_min` reach the same answer.
pub fn validate_request(
    req: &ReservationRequest,
    policy: &BookingPolicy,
    now_min: i64,
) -> Result<(), ReservationError> {
    if req.party < policy.min_party || req.party > policy.max_party {
        return Err(ReservationError::PartySize(req.party));
    }
    if req.slot_min < now_min {
        return Err(ReservationError::SlotInPast {
            slot_min: req.slot_min,
            now_min,
        });
    }
    let horizon = now_min
        .checked_add(policy.horizon_min)
        .ok_or(ReservationError::SlotOverflow)?;
    if req.slot_min > horizon {
        return Err(ReservationError::SlotTooFar {
            slot_min: req.slot_min,
            horizon_min: horizon,
        });
    }
    Ok(())
}

/// Whether a guest cancelling at `now_min` is inside the venue's free window.
///
/// Returns `Ok(true)` when the cancellation is free. Outside the window it is
/// still a legal cancellation — the venue simply knows it was late, which is a
/// fact about the booking and not a mark against the guest.
pub fn cancellation_is_free(
    slot_min: i64,
    policy: &BookingPolicy,
    now_min: i64,
) -> Result<bool, ReservationError> {
    let cutoff = slot_min
        .checked_sub(policy.free_cancel_min)
        .ok_or(ReservationError::SlotOverflow)?;
    Ok(now_min <= cutoff)
}

/// Whether a confirmed booking has been missed: the slot plus the venue's grace
/// has passed and nobody was seated.
pub fn is_no_show(
    status: ReservationStatus,
    slot_min: i64,
    grace_min: i64,
    now_min: i64,
) -> Result<bool, ReservationError> {
    if status != ReservationStatus::Confirmed {
        return Ok(false);
    }
    let deadline = slot_min
        .checked_add(grace_min)
        .ok_or(ReservationError::SlotOverflow)?;
    Ok(now_min > deadline)
}

// ─────────────────────────────────────────────────────────────────────────────
// Graph lenses — the same self-checks the order FSM carries
// ─────────────────────────────────────────────────────────────────────────────

/// How many states are reachable from `from`, itself included.
pub fn reachable(from: ReservationStatus) -> u8 {
    let mut seen = 0u8;
    let mut frontier = 1u8 << idx_of(from);
    while frontier != 0 {
        seen |= frontier;
        let mut next = 0u8;
        let mut i = 0;
        while i < 8 {
            if frontier & (1u8 << i) != 0 {
                next |= FSM_ADJ[i];
            }
            i += 1;
        }
        frontier = next & !seen;
    }
    seen.count_ones() as u8
}

/// True if the lifecycle graph contains a cycle. A booking that can return to a
/// state it has left is a booking whose history cannot be replayed, so this must
/// stay false.
pub fn has_cycle() -> bool {
    // Kahn's algorithm: repeatedly remove a state with no remaining in-edges.
    // Anything left over is in a cycle.
    let mut removed = 0u8;
    loop {
        let mut progressed = false;
        let mut i = 0;
        while i < 8 {
            let bit = 1u8 << i;
            if removed & bit == 0 {
                let mut has_in = false;
                let mut j = 0;
                while j < 8 {
                    if removed & (1u8 << j) == 0 && FSM_ADJ[j] & bit != 0 {
                        has_in = true;
                    }
                    j += 1;
                }
                if !has_in {
                    removed |= bit;
                    progressed = true;
                }
            }
            i += 1;
        }
        if !progressed {
            break;
        }
    }
    removed.count_ones() != 8
}

/// A topological order of the lifecycle, or `None` if the graph has a cycle.
pub fn topological_order() -> Option<Vec<ReservationStatus>> {
    let mut out = Vec::with_capacity(8);
    let mut removed = 0u8;
    while out.len() < 8 {
        let mut progressed = false;
        let mut i = 0;
        while i < 8 {
            let bit = 1u8 << i;
            if removed & bit == 0 {
                let mut has_in = false;
                let mut j = 0;
                while j < 8 {
                    if removed & (1u8 << j) == 0 && FSM_ADJ[j] & bit != 0 {
                        has_in = true;
                    }
                    j += 1;
                }
                if !has_in {
                    removed |= bit;
                    out.push(LIFECYCLE_STATES[i]);
                    progressed = true;
                }
            }
            i += 1;
        }
        if !progressed {
            return None;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::ReservationStatus::*;
    use super::*;

    fn policy() -> BookingPolicy {
        BookingPolicy::default_policy()
    }

    // ── The Law ──

    #[test]
    fn happy_path_folds() {
        assert_eq!(
            fold_transitions(Requested, &[Confirmed, Seated, Completed]),
            Ok(Completed)
        );
    }

    #[test]
    fn a_forbidden_transition_is_an_error_not_a_noop() {
        // Seating a party the venue never confirmed is the transition this FSM
        // exists to refuse.
        let err = assert_transition(Requested, Seated).unwrap_err();
        assert_eq!(err, ReservationError::Illegal(Requested, Seated));
    }

    #[test]
    fn fold_reports_where_it_stopped() {
        let (err, reached) =
            fold_transitions(Requested, &[Confirmed, Completed]).unwrap_err();
        assert_eq!(reached, Confirmed);
        assert_eq!(err, ReservationError::Illegal(Confirmed, Completed));
    }

    #[test]
    fn repeating_a_state_is_refused() {
        assert_eq!(
            assert_transition(Confirmed, Confirmed),
            Err(ReservationError::SameStatus(Confirmed))
        );
    }

    /// THE VENUE MAY WITHDRAW A REQUEST IT HAS NOT ANSWERED. Removing
    /// `CancelledByVenue` from `Requested`'s row turns this red; the twin
    /// below keeps the rest of the row honest.
    #[test]
    fn the_venue_may_cancel_a_request() {
        assert_eq!(assert_transition(Requested, CancelledByVenue), Ok(()));
        assert_eq!(fold_transitions(Requested, &[CancelledByVenue]), Ok(CancelledByVenue));
    }

    /// ...but a cancelled request stays cancelled, and a guest's cancel is
    /// still the guest's word, not the venue's.
    #[test]
    fn a_venue_cancelled_request_is_terminal() {
        for to in [Confirmed, Seated, Declined, CancelledByGuest, NoShow] {
            assert_eq!(
                assert_transition(CancelledByVenue, to),
                Err(ReservationError::Illegal(CancelledByVenue, to))
            );
        }
        // A request still cannot be marked a no-show: nothing was promised.
        assert!(assert_transition(Requested, NoShow).is_err());
    }

    #[test]
    fn a_seated_party_cannot_be_cancelled() {
        for to in [CancelledByGuest, CancelledByVenue, NoShow, Declined] {
            assert!(
                assert_transition(Seated, to).is_err(),
                "Seated -> {to:?} must be refused"
            );
        }
    }

    #[test]
    fn terminals_have_no_exits() {
        for s in LIFECYCLE_STATES {
            assert_eq!(
                s.is_terminal(),
                allowed_next(s).is_empty(),
                "{s:?}: is_terminal disagrees with allowed_next"
            );
        }
    }

    // ── The two derivations must agree ──

    #[test]
    fn adjacency_agrees_with_allowed_next() {
        for (i, &from) in LIFECYCLE_STATES.iter().enumerate() {
            for (j, &to) in LIFECYCLE_STATES.iter().enumerate() {
                let by_slice = allowed_next(from).contains(&to);
                let by_mask = FSM_ADJ[i] & (1u8 << j) != 0;
                assert_eq!(by_slice, by_mask, "{from:?} -> {to:?}");
            }
        }
    }

    #[test]
    fn idx_of_matches_lifecycle_order() {
        for (i, &s) in LIFECYCLE_STATES.iter().enumerate() {
            assert_eq!(idx_of(s), i);
        }
    }

    // ── Graph lenses ──

    #[test]
    fn the_lifecycle_is_acyclic() {
        assert!(!has_cycle(), "a reservation must never revisit a state");
        assert!(topological_order().is_some());
    }

    #[test]
    fn every_state_is_reachable_from_the_start() {
        assert_eq!(
            reachable(Requested),
            8,
            "a state no request can reach is a state that cannot happen"
        );
    }

    #[test]
    fn a_terminal_reaches_only_itself() {
        assert_eq!(reachable(Completed), 1);
    }

    // ── Wire form ──

    #[test]
    fn wire_form_round_trips_and_rejects_nonsense() {
        for s in LIFECYCLE_STATES {
            assert_eq!(ReservationStatus::from_str(s.as_str()), Some(s));
        }
        assert_eq!(ReservationStatus::from_str("SEATED_MAYBE"), None);
        assert_eq!(ReservationStatus::from_str(""), None);
    }

    // ── The request ──

    fn req(party: u16, slot: i64) -> ReservationRequest {
        ReservationRequest {
            id: 1,
            venue: "dubin-sushi".into(),
            party,
            slot_min: slot,
            occasion: "Birthday".into(),
        }
    }

    #[test]
    fn a_request_in_the_window_is_accepted() {
        assert_eq!(validate_request(&req(6, 1_000_100), &policy(), 1_000_000), Ok(()));
    }

    #[test]
    fn a_past_slot_is_refused() {
        assert_eq!(
            validate_request(&req(6, 999_999), &policy(), 1_000_000),
            Err(ReservationError::SlotInPast {
                slot_min: 999_999,
                now_min: 1_000_000
            })
        );
    }

    #[test]
    fn a_slot_past_the_horizon_is_refused() {
        let p = policy();
        let far = 1_000_000 + p.horizon_min + 1;
        assert!(matches!(
            validate_request(&req(2, far), &p, 1_000_000),
            Err(ReservationError::SlotTooFar { .. })
        ));
    }

    #[test]
    fn party_size_is_the_venues_to_set() {
        let p = BookingPolicy {
            min_party: 2,
            max_party: 4,
            ..BookingPolicy::default_policy()
        };
        assert_eq!(
            validate_request(&req(1, 1_000_100), &p, 1_000_000),
            Err(ReservationError::PartySize(1))
        );
        assert_eq!(
            validate_request(&req(5, 1_000_100), &p, 1_000_000),
            Err(ReservationError::PartySize(5))
        );
        assert_eq!(validate_request(&req(4, 1_000_100), &p, 1_000_000), Ok(()));
    }

    #[test]
    fn horizon_overflow_fails_closed() {
        let p = BookingPolicy {
            horizon_min: i64::MAX,
            ..BookingPolicy::default_policy()
        };
        assert_eq!(
            validate_request(&req(2, i64::MAX), &p, 1),
            Err(ReservationError::SlotOverflow)
        );
    }

    #[test]
    fn a_decision_reads_no_clock() {
        // The same request judged at two different `now_min` values gives two
        // different answers, and nothing inside the function can tell which is
        // "really" now. That is what makes an offline replay deterministic.
        let r = req(2, 1_000_100);
        assert_eq!(validate_request(&r, &policy(), 1_000_000), Ok(()));
        assert!(validate_request(&r, &policy(), 1_000_200).is_err());
    }

    // ── Cancellation window and no-show ──

    #[test]
    fn cancelling_early_is_free_and_late_is_not() {
        let p = policy(); // free_cancel_min = 120
        assert_eq!(cancellation_is_free(1_000_000, &p, 999_000), Ok(true));
        assert_eq!(cancellation_is_free(1_000_000, &p, 999_880), Ok(true));
        assert_eq!(cancellation_is_free(1_000_000, &p, 999_881), Ok(false));
    }

    #[test]
    fn no_show_only_applies_to_a_confirmed_booking() {
        assert_eq!(is_no_show(Requested, 1_000_000, 15, 9_999_999), Ok(false));
        assert_eq!(is_no_show(Seated, 1_000_000, 15, 9_999_999), Ok(false));
        assert_eq!(is_no_show(Confirmed, 1_000_000, 15, 1_000_014), Ok(false));
        assert_eq!(is_no_show(Confirmed, 1_000_000, 15, 1_000_016), Ok(true));
    }

    #[test]
    fn no_show_arithmetic_fails_closed() {
        assert_eq!(
            is_no_show(Confirmed, i64::MAX, 1, 0),
            Err(ReservationError::SlotOverflow)
        );
    }

    // ── The red line ──

    #[test]
    fn this_module_scores_nobody() {
        // `DECISIONS.md` D0: trust is a signed capability, never a score. A
        // guest-reliability field here would be the same forbidden thing as a
        // courier score. The tokens are split so this check cannot match itself.
        let src = include_str!("reservation.rs");
        for token in [
            concat!("guest_", "score"),
            concat!("guest_", "rating"),
            concat!("reliability_", "score"),
            concat!("courier_", "score"),
            concat!("reputation"),
        ] {
            let hits = src.matches(token).count();
            // The word may appear in prose exactly where this test names it.
            assert!(
                hits <= 1,
                "{token} appears {hits} times — a score has crept into the reservation domain"
            );
        }
    }
}

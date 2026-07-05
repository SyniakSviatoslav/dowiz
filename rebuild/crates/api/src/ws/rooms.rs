//! `Room` — the typed WS room identity, ported from the wire string prefixes
//! `packages/platform/src/lib/registry.ts:47-50` actually mint (`order:<id>`,
//! `location:<id>:dashboard`, `location:<id>:couriers`, `courier:<id>`) and the admission
//! predicates `apps/api/src/websocket.ts` / `courier-room-authz.ts` read (`room.startsWith(...)`).
//!
//! `courier:<id>:shift` (Q-WS-SHIFT-ROOM) is deliberately NOT a variant here: it has zero FE
//! subscribers (grep-confirmed) and the courier subscribe gate only ever admits an EXACT
//! `courier:<sub>` match anyway (`websocket.ts:411`), so a `:shift`-suffixed room could never pass
//! subscribe on the old stack either — it RETIREs post-cutover (room count 5→4). Parsing a
//! `courier:<id>:shift` string here falls through to `None` (unrecognized), which the caller
//! treats as DENY — behaviorally identical to today's dead room.
//!
//! `RoomRegistry` is the fan-out chokepoint (Q-WS-RELAY-GUARD / the retired
//! `local/no-raw-courier-ws-send` ESLint rule, now a Rust module-visibility property): the only
//! way to reach a member's socket sender is [`RoomRegistry::members_of`], which is
//! `pub(crate)` — reachable from `ws::mod`'s bus-fan-out task (which routes every member through a
//! [`super::guard`] policy or a direct relay for principals with no revocable binding), never from
//! arbitrary handler code.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::protocol::WireMessage;

/// A parsed, typed room identity. `Display`/`FromStr`-equivalent helpers round-trip the exact wire
/// strings the Node stack mints (`registry.ts`) so a Rust-side room key and a Node-published NOTIFY
/// channel name always agree during the overlap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Room {
    Order(Uuid),
    LocationDashboard(Uuid),
    LocationCouriers(Uuid),
    CourierSelf(Uuid),
}

impl Room {
    /// Parses a wire room string. Anything that doesn't match one of the four live shapes
    /// (including the dead `courier:<id>:shift`) is `None` — every caller treats an unparsed room
    /// as DENY (fail closed), matching the Node `else return DENY` branches.
    pub fn parse(raw: &str) -> Option<Room> {
        if let Some(rest) = raw.strip_prefix("order:") {
            return Uuid::parse_str(rest).ok().map(Room::Order);
        }
        if let Some(rest) = raw.strip_prefix("location:") {
            if let Some(id) = rest.strip_suffix(":dashboard") {
                return Uuid::parse_str(id).ok().map(Room::LocationDashboard);
            }
            if let Some(id) = rest.strip_suffix(":couriers") {
                return Uuid::parse_str(id).ok().map(Room::LocationCouriers);
            }
            return None;
        }
        if let Some(rest) = raw.strip_prefix("courier:") {
            // Exact `courier:<id>` only — `courier:<id>:shift` (extra segment) is NOT this variant
            // (Q-WS-SHIFT-ROOM, dead room; see module doc).
            return Uuid::parse_str(rest).ok().map(Room::CourierSelf);
        }
        None
    }

    /// The exact wire string — round-trips through [`Room::parse`]. Used both to re-derive the
    /// canonical channel name for `PgListener` `LISTEN`/`UNLISTEN` and as the relay-guard cache key
    /// prefix (mirrors the TS `` `${room} ${id}` `` key shape).
    pub fn wire(self) -> String {
        match self {
            Room::Order(id) => format!("order:{id}"),
            Room::LocationDashboard(id) => format!("location:{id}:dashboard"),
            Room::LocationCouriers(id) => format!("location:{id}:couriers"),
            Room::CourierSelf(id) => format!("courier:{id}"),
        }
    }
}

/// A per-connection handle the registry can push frames through. `principal` carries whatever the
/// fan-out dispatcher (`ws::mod`'s bus-consumer task) needs to run the right guard check for THIS
/// member on THIS frame — read fresh from here on every frame, never cached (the cache lives
/// inside the guard, keyed on the identity `principal` carries).
#[derive(Clone)]
pub struct MemberHandle {
    pub conn_id: u64,
    pub sender: UnboundedSender<WireMessage>,
    pub principal: super::admission::Principal,
}

/// In-process room membership. One process, one registry — `PgListener` fan-out
/// (`super::pg_fanout`) delivers ACROSS processes; this delivers to the sockets THIS process holds.
///
/// Eager teardown: a room's entry is removed the instant its member set empties (P1-WSDUP parity,
/// `websocket.ts:196-210,498-517`) so a rejoin never stacks a second logical subscription — the
/// `PgListener` LISTEN/UNLISTEN lifecycle in `pg_fanout` mirrors this same "room exists" signal.
#[derive(Default)]
pub struct RoomRegistry {
    rooms: Mutex<HashMap<Room, HashMap<u64, MemberHandle>>>,
}

impl RoomRegistry {
    pub fn new() -> Self {
        RoomRegistry::default()
    }

    /// Join `room`. Returns `true` iff this was the room's FIRST member (the caller then knows to
    /// `LISTEN` the channel — see `ws::mod`'s subscribe handler).
    pub fn join(&self, room: Room, member: MemberHandle) -> bool {
        let mut rooms = self
            .rooms
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let is_new_room = !rooms.contains_key(&room);
        rooms
            .entry(room)
            .or_default()
            .insert(member.conn_id, member);
        is_new_room
    }

    /// Leave `room`. Returns `true` iff the room is now EMPTY (the caller then `UNLISTEN`s + drops
    /// the room, eager-teardown parity).
    pub fn leave(&self, room: Room, conn_id: u64) -> bool {
        let mut rooms = self
            .rooms
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(members) = rooms.get_mut(&room) else {
            return false;
        };
        members.remove(&conn_id);
        let now_empty = members.is_empty();
        if now_empty {
            rooms.remove(&room);
        }
        now_empty
    }

    /// Remove `conn_id` from EVERY room it belongs to (socket close — `websocket.ts:498-517`).
    /// Returns the rooms that became empty as a result (caller UNLISTENs each).
    pub fn leave_all(&self, conn_id: u64) -> Vec<Room> {
        let mut rooms = self
            .rooms
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut emptied = Vec::new();
        rooms.retain(|room, members| {
            members.remove(&conn_id);
            if members.is_empty() {
                emptied.push(*room);
                false
            } else {
                true
            }
        });
        emptied
    }

    /// The fan-out chokepoint: every member currently in `room`. `pub(crate)` — reachable only from
    /// within the `ws` module's own bus-consumer task, which routes each one through a relay-guard
    /// policy (or a direct relay for a principal with no revocable binding) before ever calling
    /// `sender.send(..)`. No other module can reach a member's raw sender (Q-WS-RELAY-GUARD).
    pub(crate) fn members_of(&self, room: Room) -> Vec<MemberHandle> {
        let rooms = self
            .rooms
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        rooms
            .get(&room)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Room-count observability (proposal §11: "per-room member gauge").
    #[allow(
        dead_code,
        reason = "forward-looking observability seam — no metrics endpoint reads this yet in this \
                  dark build; exercised by this module's own eager-teardown tests"
    )]
    pub fn room_count(&self) -> usize {
        self.rooms
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// Every member across every room (deduplicated by connection — a member subscribed to
    /// several rooms is only sent one copy). REV-S6-6: the ONLY caller is the `PgListener`
    /// degraded→healthy recovery signal, which is content-free (no room-scoped data, just "please
    /// refetch") — a cross-room broadcast of that specific signal carries no cross-tenant leak risk,
    /// unlike a real room-fanout frame (which always goes through [`RoomRegistry::members_of`] +
    /// a guard).
    pub(crate) fn all_member_senders(&self) -> Vec<UnboundedSender<WireMessage>> {
        let rooms = self
            .rooms
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for members in rooms.values() {
            for (conn_id, handle) in members {
                if seen.insert(*conn_id) {
                    out.push(handle.sender.clone());
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::admission::Principal;
    use super::*;

    fn chan() -> (
        UnboundedSender<WireMessage>,
        tokio::sync::mpsc::UnboundedReceiver<WireMessage>,
    ) {
        tokio::sync::mpsc::unbounded_channel()
    }

    /// A throwaway customer principal — the room-registry tests below only care about connection
    /// bookkeeping (join/leave/fan-out membership), not which principal kind it is.
    fn member(conn_id: u64, sender: UnboundedSender<WireMessage>) -> MemberHandle {
        MemberHandle {
            conn_id,
            sender,
            principal: Principal::Customer {
                order_id: Uuid::nil(),
                location_id: Uuid::nil(),
                sub: Uuid::nil(),
            },
        }
    }

    #[test]
    fn parses_and_round_trips_every_live_room_shape() {
        let order = Uuid::new_v4();
        let loc = Uuid::new_v4();
        assert_eq!(
            Room::parse(&format!("order:{order}")),
            Some(Room::Order(order))
        );
        assert_eq!(
            Room::parse(&format!("location:{loc}:dashboard")),
            Some(Room::LocationDashboard(loc))
        );
        assert_eq!(
            Room::parse(&format!("location:{loc}:couriers")),
            Some(Room::LocationCouriers(loc))
        );
        assert_eq!(
            Room::parse(&format!("courier:{order}")),
            Some(Room::CourierSelf(order))
        );

        for room in [
            Room::Order(order),
            Room::LocationDashboard(loc),
            Room::LocationCouriers(loc),
            Room::CourierSelf(order),
        ] {
            assert_eq!(
                Room::parse(&room.wire()),
                Some(room),
                "round-trip must be lossless"
            );
        }
    }

    #[test]
    fn dead_shift_room_and_garbage_are_unparsed_deny() {
        let id = Uuid::new_v4();
        assert_eq!(Room::parse(&format!("courier:{id}:shift")), None);
        assert_eq!(Room::parse("bogus"), None);
        assert_eq!(Room::parse("order:not-a-uuid"), None);
        assert_eq!(Room::parse(""), None);
    }

    #[test]
    fn join_reports_first_member_and_leave_reports_emptied() {
        let registry = RoomRegistry::new();
        let room = Room::Order(Uuid::new_v4());
        let (tx1, _rx1) = chan();
        let (tx2, _rx2) = chan();

        assert!(
            registry.join(room, member(1, tx1)),
            "first joiner creates the room"
        );
        assert!(
            !registry.join(room, member(2, tx2)),
            "second joiner does not re-create it"
        );
        assert_eq!(registry.members_of(room).len(), 2);

        assert!(
            !registry.leave(room, 1),
            "one member remains — not empty yet"
        );
        assert!(
            registry.leave(room, 2),
            "last member leaving empties the room"
        );
        assert_eq!(registry.members_of(room).len(), 0);
        assert_eq!(
            registry.room_count(),
            0,
            "an emptied room is torn down eagerly"
        );
    }

    #[test]
    fn leave_all_removes_a_connection_from_every_room_it_joined() {
        let registry = RoomRegistry::new();
        let room_a = Room::Order(Uuid::new_v4());
        let room_b = Room::CourierSelf(Uuid::new_v4());
        let (tx, _rx) = chan();
        registry.join(room_a, member(7, tx.clone()));
        registry.join(room_b, member(7, tx));

        let emptied = registry.leave_all(7);
        assert_eq!(emptied.len(), 2);
        assert_eq!(registry.room_count(), 0);
    }

    /// Cross-tenant fan-out isolation, at the registry layer: two tenants' `order:` rooms are
    /// disjoint member sets — a member of tenant A's room is structurally absent from tenant B's
    /// `members_of` result. The guard layer (`super::guard`) adds the per-frame re-authz on top;
    /// this proves the registry itself never conflates two rooms' membership.
    #[test]
    fn members_of_never_leaks_across_distinct_rooms() {
        let registry = RoomRegistry::new();
        let room_a = Room::Order(Uuid::new_v4());
        let room_b = Room::Order(Uuid::new_v4());
        let (tx_a, _rx_a) = chan();
        let (tx_b, _rx_b) = chan();
        registry.join(room_a, member(1, tx_a));
        registry.join(room_b, member(2, tx_b));

        let a_members: Vec<u64> = registry
            .members_of(room_a)
            .iter()
            .map(|m| m.conn_id)
            .collect();
        let b_members: Vec<u64> = registry
            .members_of(room_b)
            .iter()
            .map(|m| m.conn_id)
            .collect();
        assert_eq!(a_members, vec![1]);
        assert_eq!(b_members, vec![2]);
    }
}

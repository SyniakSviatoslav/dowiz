//! Who people are, which venues exist, and which sessions are live — in the
//! platform's own bebop images.
//!
//! THE LAST FAMILY, AND THE ONE WITH THE BLAST RADIUS. A bad cutover here locks
//! everyone out, so it lands last and behind a fallback. What it replaces is
//! `users`, `organizations`, `memberships`, `platform_admins`,
//! `auth_refresh_tokens`, `owner_api_keys`, `couriers`, `courier_locations`,
//! `courier_sessions`, `courier_invites`, `courier_audit_log`, `locations` and
//! `customers` — thirteen tables and roughly a dozen indexes.
//!
//! EVERY ACCESS PATH IS A KEY WRITTEN ON PURPOSE. There is no `WHERE` and no
//! query planner; the Kv layout keeps its keys sorted, so a prefix scan is the
//! whole of `ORDER BY` and a composite index. The keys are composed HERE, in
//! one place, by every reader and every writer — a key composed in two places
//! is a key composed two ways, and the reader would then find nothing while
//! reporting no error at all.
//!
//! WHY THE PLATFORM OBJECT AND NOT THE VENUE'S. These are the facts that are
//! NOT one venue's: a person may belong to two restaurants, a login has to
//! resolve before a venue is known, and the registry is what turns a host into
//! a venue in the first place. Everything that IS a venue's — its catalogue,
//! its orders, its couriers' shifts — has already moved into that venue's own
//! object, which is what makes tenancy structural there. Here the tenancy check
//! is explicit and that is the honest arrangement: a membership record IS the
//! check, and it is keyed by both sides.

use worker::*;

use crate::platform_store::{self, COURIERS, IDENTITY, REGISTRY, SESSIONS};
use dowiz_hub::table::Table;

// ── record kinds ───────────────────────────────────────────────────────────

pub const K_USER: &str = "user";
pub const K_MEMBER: &str = "member";
pub const K_ADMIN: &str = "admin";
pub const K_LOC: &str = "loc";
pub const K_REFRESH: &str = "refresh";
pub const K_APIKEY: &str = "apikey";
pub const K_CSESSION: &str = "csession";
pub const K_COURIER: &str = "courier";
pub const K_ROSTER: &str = "roster";
pub const K_INVITE: &str = "invite";

// ── the keys, composed once ────────────────────────────────────────────────

/// An address finds exactly one person. UNIQUE, checked inside the object's own
/// turn — which is the one thing a Worker holding a copy of an image could
/// never do correctly, because between its read and its write anything could
/// have run.
pub fn user_by_email(email: &str) -> String {
    format!("user.email/{}", email.trim().to_ascii_lowercase())
}

/// A membership is keyed by BOTH sides, written together, so neither direction
/// is a scan of everything. `memberships_lookup_idx` was
/// `(location_id, user_id, role, status)`; this is that index and its mirror.
pub fn member_id(location_id: &str, user_id: &str) -> String {
    format!("{location_id}/{user_id}")
}

pub fn member_by_venue(location_id: &str, user_id: &str) -> String {
    format!("member.venue/{location_id}/{user_id}")
}

pub fn member_by_user(user_id: &str, location_id: &str) -> String {
    format!("member.user/{user_id}/{location_id}")
}

/// A venue's public name, and the host that answers for it.
pub fn loc_by_slug(slug: &str) -> String {
    format!("loc.slug/{}", slug.trim().to_ascii_lowercase())
}

/// A refresh token's family, ordered in time.
///
/// ZERO-PADDED, because keys sort as strings: unpadded, a family's tenth token
/// would sort before its second and "everything issued after this one" would
/// return the wrong set — which is the reuse-detection query, so getting it
/// wrong means a stolen token is not revoked.
pub fn refresh_family(family_id: &str, created_at_ms: i64) -> String {
    format!("refresh.family/{family_id}/{created_at_ms:014}")
}

/// An owner's keys, listable without scanning every key on the platform.
pub fn apikey_at(location_id: &str, hash: &str) -> String {
    format!("apikey.loc/{location_id}/{hash}")
}

/// A courier is found by the phone or the email they log in with. UNIQUE, and
/// SEPARATE: the old schema matched both against one shared hash space, where a
/// phone could in principle resolve an email's record. Two prefixes cannot.
pub fn courier_by_phone(phone_hash: &str) -> String {
    format!("courier.phone/{phone_hash}")
}

pub fn courier_by_email(email_hash: &str) -> String {
    format!("courier.email/{email_hash}")
}

/// Which venues a courier is on the roster of, and who is on a venue's roster.
pub fn roster_id(location_id: &str, courier_id: &str) -> String {
    format!("{location_id}/{courier_id}")
}

pub fn roster_by_venue(location_id: &str, courier_id: &str) -> String {
    format!("roster.venue/{location_id}/{courier_id}")
}

pub fn roster_by_courier(courier_id: &str, location_id: &str) -> String {
    format!("roster.courier/{courier_id}/{location_id}")
}

/// An invite, found by the phone it was sent to and listable per venue.
pub fn invite_by_phone(phone: &str) -> String {
    format!("invite.phone/{phone}")
}

pub fn invite_at(location_id: &str, id: &str) -> String {
    format!("invite.loc/{location_id}/{id}")
}

// ── reading ────────────────────────────────────────────────────────────────

pub async fn identity(env: &Env) -> Result<Table> {
    Ok(platform_store::load(env, IDENTITY).await?.table)
}

pub async fn registry(env: &Env) -> Result<Table> {
    Ok(platform_store::load(env, REGISTRY).await?.table)
}

pub async fn sessions(env: &Env) -> Result<Table> {
    Ok(platform_store::load(env, SESSIONS).await?.table)
}

pub async fn couriers(env: &Env) -> Result<Table> {
    Ok(platform_store::load(env, COURIERS).await?.table)
}

pub async fn with_identity<F, T>(env: &Env, f: F) -> Result<T>
where
    F: FnMut(&mut Table) -> Result<T>,
{
    platform_store::with(env, IDENTITY, f).await
}

pub async fn with_registry<F, T>(env: &Env, f: F) -> Result<T>
where
    F: FnMut(&mut Table) -> Result<T>,
{
    platform_store::with(env, REGISTRY, f).await
}

pub async fn with_sessions<F, T>(env: &Env, f: F) -> Result<T>
where
    F: FnMut(&mut Table) -> Result<T>,
{
    platform_store::with(env, SESSIONS, f).await
}

pub async fn with_couriers<F, T>(env: &Env, f: F) -> Result<T>
where
    F: FnMut(&mut Table) -> Result<T>,
{
    platform_store::with(env, COURIERS, f).await
}

/// One record as JSON, or nothing.
pub fn rec(t: &Table, kind: &str, id: &str) -> Option<serde_json::Value> {
    t.get(kind, id).and_then(|j| serde_json::from_str(&j).ok())
}

pub fn s_of(v: &serde_json::Value, k: &str) -> String {
    v.get(k).and_then(serde_json::Value::as_str).unwrap_or("").to_string()
}

pub fn i_of(v: &serde_json::Value, k: &str) -> i64 {
    v.get(k).and_then(serde_json::Value::as_i64).unwrap_or(0)
}

/// The person who holds this address, if anyone.
pub fn user_id_for_email(t: &Table, email: &str) -> Option<String> {
    t.lookup(&user_by_email(email))
}

/// The courier who logs in with this phone or email, if anyone.
pub fn courier_id_for_phone(t: &Table, phone_hash: &str) -> Option<String> {
    t.lookup(&courier_by_phone(phone_hash))
}

pub fn courier_id_for_email(t: &Table, email_hash: &str) -> Option<String> {
    t.lookup(&courier_by_email(email_hash))
}

/// Every index key a courier record owns, from the record itself. Used by the
/// writers and by `rebuild_index`, so the two cannot disagree.
pub fn courier_index(id: &str, rec: &serde_json::Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let p = s_of(rec, "phone_hash");
    if !p.is_empty() {
        out.push((courier_by_phone(&p), id.to_string()));
    }
    let e = s_of(rec, "email_hash");
    if !e.is_empty() {
        out.push((courier_by_email(&e), id.to_string()));
    }
    out
}

/// An ACTIVE membership of this venue, or nothing.
///
/// THE WHOLE TENANCY CHECK, in one function. It used to be six different
/// queries in five files, and four of them got it wrong in a different way:
/// one asked "is this user an owner of ANYTHING", one took `ORDER BY
/// created_at_ms LIMIT 1`, one dropped the scope entirely. One function, one
/// key, one answer.
pub fn membership(t: &Table, location_id: &str, user_id: &str) -> Option<serde_json::Value> {
    rec(t, K_MEMBER, &member_id(location_id, user_id))
        .filter(|m| s_of(m, "status") == "active")
}

/// Every venue this user is an active member of, with their role.
pub fn memberships_of(t: &Table, user_id: &str) -> Vec<(String, String)> {
    t.scan(&format!("member.user/{user_id}/"))
        .into_iter()
        .filter_map(|(key, _)| {
            let loc = key.rsplit('/').next()?.to_string();
            let m = membership(t, &loc, user_id)?;
            Some((loc, s_of(&m, "role")))
        })
        .collect()
}

#[cfg(test)]
mod key_tests {
    use super::*;

    /// An address is one person however it was typed.
    #[test]
    fn an_email_key_does_not_care_about_case_or_spaces() {
        assert_eq!(user_by_email("  Kitchen@Dubin.AL "), user_by_email("kitchen@dubin.al"));
    }

    /// The reuse-detection query is "everything this family issued after that
    /// one", and it is a sorted prefix scan.
    #[test]
    fn a_familys_tokens_sort_in_time_order() {
        let mut keys: Vec<String> = [5i64, 50, 500, 1_789_000_000_000]
            .iter()
            .map(|t| refresh_family("fam", *t))
            .collect();
        keys.sort();
        assert_eq!(keys[0], refresh_family("fam", 5));
        assert_eq!(keys[1], refresh_family("fam", 50));
        assert_eq!(keys[2], refresh_family("fam", 500));
        assert_eq!(keys[3], refresh_family("fam", 1_789_000_000_000));
    }

    /// The padding has to outlast the product. Milliseconds since the epoch
    /// need fourteen digits until the year 5138.
    #[test]
    fn the_time_padding_does_not_overflow_within_the_products_life() {
        let far = 1_789_000_000_000i64 * 4;
        assert_eq!(format!("{far:014}").len(), 14);
    }

    /// One family's scan cannot reach another's.
    #[test]
    fn a_family_prefix_is_that_family_alone() {
        assert!(refresh_family("fam1", 1).starts_with("refresh.family/fam1/"));
        assert!(!refresh_family("fam12", 1).starts_with("refresh.family/fam1/"));
    }

    /// A membership is keyed BOTH ways and the two agree about which pair they
    /// name. Getting this wrong means a user who is a member by one index and
    /// not by the other.
    #[test]
    fn both_directions_of_a_membership_name_the_same_pair() {
        let (loc, user) = ("dubin-durres", "usr_1");
        assert!(member_by_venue(loc, user).ends_with(&format!("/{loc}/{user}")));
        assert!(member_by_user(user, loc).ends_with(&format!("/{user}/{loc}")));
        assert_eq!(member_id(loc, user), format!("{loc}/{user}"));
    }

    /// The venue prefix is exact. `dubin-durres` and `dubin-durres-2` are two
    /// restaurants, and a scan of the first must not reach the second.
    #[test]
    fn a_venue_prefix_does_not_reach_a_venue_whose_id_starts_the_same() {
        let p = "member.venue/dubin-durres/";
        assert!(member_by_venue("dubin-durres", "u").starts_with(p));
        assert!(!member_by_venue("dubin-durres-2", "u").starts_with(p));
        // The same rule for the two courier indexes and the api keys.
        assert!(!roster_by_venue("dubin-durres-2", "c")
            .starts_with("roster.venue/dubin-durres/"));
        assert!(!apikey_at("dubin-durres-2", "h").starts_with("apikey.loc/dubin-durres/"));
        assert!(!invite_at("dubin-durres-2", "i").starts_with("invite.loc/dubin-durres/"));
    }

    #[test]
    fn a_slug_key_is_case_folded_like_a_host_is() {
        assert_eq!(loc_by_slug("Dubin-Sushi"), loc_by_slug("dubin-sushi"));
    }
}

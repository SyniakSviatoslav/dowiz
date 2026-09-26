//! The owner's routes, opened to the member of staff who holds the word.
//!
//! OPERATOR Q8 (2026-09-26): a kitchen login opens THE SAME HUB as the owner
//! and manages the menu, the ingredients and the stock there. The routes that
//! do that were written for the owner (`owner_and_venue`, `owner_beside`);
//! these are their twins with one extra question -- "or a member of staff
//! holding one of `needs`, at this venue".
//!
//! AN OWNER LOSES NOTHING. Any token that is not a staff token is handed to the
//! owner's own guard, unchanged, so the console, an owner's API key and the
//! owner's venue resolution are exactly what they were. Only a staff token
//! takes the new path, and it is asked `room_admits_any` -- a staff principal of
//! another venue is a 404, a staff principal without the word is a 403.

use worker::*;

use crate::auth::{self, Cap, Claims};

/// THE THREE FAMILIES, named once so the law each route asks is tested here.
/// The menu: dishes, categories, translations, the 86, supplies, recipes, imports.
pub(crate) const MENU: [Cap; 1] = [Cap::Catalog];
/// The shelf: levels, the waste report, a delivery received, a count.
pub(crate) const SHELF: [Cap; 1] = [Cap::Stock];
/// A write-off: the drawer's holder at midnight, or the shelf's.
pub(crate) const BIN: [Cap; 2] = [Cap::OpenTill, Cap::Stock];

/// Is the bearer a staff token? Signature and expiry only; the principal (the
/// session row, the live roster word) is built by `authenticate` after this.
fn staff_claim(req: &Request, ctx: &RouteContext<crate::Req>) -> Option<Claims> {
    auth::bearer(req)
        .ok()
        .and_then(|raw| auth::verify(&ctx.env, &raw, ctx.data.now_ms).ok())
        .filter(|c| matches!(c, Claims::Staff { .. }))
}

/// Which venue a staff request means: `?location_id=` first, then the venue
/// its token was minted at. The same order `owner_and_venue` uses.
pub(crate) fn staff_venue_of(query: Option<String>, claim: &Claims) -> Option<String> {
    query.or_else(|| match claim {
        Claims::Staff { active_location_id, .. } => Some(active_location_id.clone()),
        _ => None,
    })
}

/// `owner_and_venue`, or a member of staff holding ANY of `needs`:
/// `(signer, venue)`.
pub(crate) async fn staff_venue(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
    needs: &[Cap],
) -> std::result::Result<(String, String), Response> {
    let Some(claim) = staff_claim(req, ctx) else {
        return crate::owner::owner_and_venue(req, ctx).await;
    };
    let venue = staff_venue_of(crate::owner::location_of(req), &claim)
        .ok_or_else(|| Response::error("which venue?", 400).unwrap())?;
    let p = auth::authenticate(req, &ctx.env, ctx.data.now_ms)
        .await
        .map_err(|e| e.into_response().unwrap())?;
    let (by, _) =
        auth::room_admits_any(&p, &venue, needs).map_err(|(s, m)| Response::error(m, s).unwrap())?;
    Ok((by, venue))
}

/// `owner_beside`, or a member of staff holding ANY of `needs`:
/// `(signer, venue, work)`. The venue that was authorised is the venue that
/// was read (`Place::must_be`), exactly as on the owner's path.
pub(crate) async fn staff_beside<F, T>(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
    place: &crate::hubstore::Place,
    needs: &[Cap],
    work: F,
) -> std::result::Result<(String, String, T), Response>
where
    F: std::future::Future<Output = Result<T>>,
{
    if staff_claim(req, ctx).is_none() {
        return crate::owner::owner_beside(req, ctx, place, work).await;
    }
    let (by, venue) = staff_venue(req, ctx, needs).await?;
    place.must_be(&venue)?;
    let out = work.await.map_err(|e| Response::error(format!("hub unavailable: {e}"), 503).unwrap())?;
    Ok((by, venue, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn staff_at(loc: &str) -> Claims {
        Claims::Staff {
            sub: "p1".into(),
            active_location_id: loc.into(),
            jti: "s1".into(),
            caps: "advance,catalog,stock".into(),
            iat: 0,
            exp: 1,
        }
    }

    /// The query names the venue when it is given; otherwise the token's own
    /// venue -- never "whichever membership comes first".
    #[test]
    fn the_query_wins_then_the_token() {
        assert_eq!(staff_venue_of(Some("b".into()), &staff_at("a")), Some("b".into()));
        assert_eq!(staff_venue_of(None, &staff_at("a")), Some("a".into()));
    }

    fn staff(p: dowiz_hub::caps::Preset) -> auth::Principal {
        auth::Principal::Staff {
            person_id: "p1".into(),
            active_location_id: "v".into(),
            session_id: "s1".into(),
            caps: p.caps(),
        }
    }

    /// Each route family, asked of each staff word, through the real door.
    /// The kitchen passes all three; the waiter none; the counter bins only.
    #[test]
    fn each_family_admits_exactly_its_words() {
        use dowiz_hub::caps::Preset::*;
        for (fam, name) in [(&MENU[..], "menu"), (&SHELF[..], "shelf"), (&BIN[..], "bin")] {
            assert!(auth::room_admits_any(&staff(Kitchen), "v", fam).is_ok(), "kitchen refused {name}");
            assert_eq!(auth::room_admits_any(&staff(Waiter), "v", fam).map(|x| x.0).map_err(|e| e.0), Err(403), "waiter got {name}");
            assert_eq!(auth::room_admits_any(&staff(Kitchen), "w", fam).map(|x| x.0).map_err(|e| e.0), Err(404), "{name} crossed venues");
            let owner = auth::Principal::Owner { user_id: "o".into(), active_location_id: Some("v".into()) };
            assert!(auth::room_admits_any(&owner, "v", fam).is_ok(), "owner refused {name}");
        }
        assert!(auth::room_admits_any(&staff(CounterManager), "v", &BIN).is_ok());
        assert!(auth::room_admits_any(&staff(CounterManager), "v", &MENU).is_err());
        assert!(auth::room_admits_any(&staff(CounterManager), "v", &SHELF).is_err());
    }

    /// A claim that is not staff names no venue here: the owner's path decides
    /// those, never this function.
    #[test]
    fn a_non_staff_claim_names_no_venue() {
        let owner = Claims::Owner {
            sub: "u".into(),
            user_id: "u".into(),
            active_location_id: Some("a".into()),
            iat: 0,
            exp: 1,
        };
        assert_eq!(staff_venue_of(None, &owner), None);
        assert_eq!(staff_venue_of(Some("q".into()), &owner), Some("q".into()));
    }
}

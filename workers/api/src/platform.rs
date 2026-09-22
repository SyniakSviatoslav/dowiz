//! The main hub — where a client's hub is created.
//!
//! ONE CLIENT, ONE SUBDOMAIN. `dowiz.org` is the platform and
//! `sushi-durres.dowiz.org` is that venue's hub: its storefront, its console,
//! its courier app, its Durable Object, its images. This module is the only
//! place a new one comes into existence.
//!
//! IT IS NOT `bootstrap`, AND THE DIFFERENCE IS THE POINT. `bootstrap` is
//! seeded by a shared secret in the Worker's environment: anyone holding that
//! string can seed any hub, it is the same string for every tenant, and it
//! cannot be revoked for one person without being rotated for everyone. That
//! was honest for a single venue being filled once. It is not a way to run a
//! platform. Here the caller is a NAMED HUMAN in `platform_admins`, so the
//! audit answer to "who created this venue" is a user id and not "whoever had
//! the secret".
//!
//! THE SLUG IS A HOSTNAME, which is the constraint that shapes the rest of this
//! file. It is not a display name with a tidy-up applied; it becomes a public
//! DNS label on the platform domain, so it may hold only what a label may hold,
//! and it may not be one of the names the platform answers to itself. A venue
//! that managed to register `www` would own `www.dowiz.org`.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use crate::auth::{self};
use crate::owner::now_ms;

/// Names a venue may never take, because the platform answers to them.
///
/// `www` and `api` are the obvious ones. `admin` and `courier` are here because
/// they are paths on every venue host and a venue named `admin` would make
/// `admin.dowiz.org/admin` mean two different things to two readers. The rest
/// are the names infrastructure conventionally claims, reserved now while it
/// costs nothing rather than after someone owns one.
pub const RESERVED_SLUGS: &[&str] = &[
    "www", "api", "admin", "courier", "app", "mail", "smtp", "imap", "ftp", "ns",
    "ns1", "ns2", "dns", "mx", "webhook", "static", "assets", "cdn", "img",
    "images", "media", "dashboard", "platform", "status", "health", "support",
    "help", "docs", "blog", "shop", "store", "test", "staging", "dev", "demo",
];

/// Is this a legal DNS label AND a name the platform is willing to give away?
///
/// RFC 1123: letters, digits and hyphens; not starting or ending with a hyphen;
/// at most 63 octets. Lowercase only, because a hostname is case-insensitive
/// and storing two spellings of one venue is how a slug stops being unique.
pub fn slug_problem(slug: &str) -> Option<&'static str> {
    if slug.is_empty() {
        return Some("slug is empty");
    }
    if slug.len() > 63 {
        return Some("slug is longer than a DNS label may be (63)");
    }
    if !slug
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Some("slug may hold only lowercase letters, digits and hyphens");
    }
    if slug.starts_with('-') || slug.ends_with('-') {
        return Some("slug may not start or end with a hyphen");
    }
    // A label with two hyphens in positions 3 and 4 is the IDN prefix form
    // (`xn--`). Refusing the shape keeps a venue from minting something a
    // resolver will read as punycode.
    if slug.len() > 4 && &slug[2..4] == "--" {
        return Some("slug may not use the reserved xn-- form");
    }
    if RESERVED_SLUGS.contains(&slug) {
        return Some("that name is reserved by the platform");
    }
    None
}

/// The caller must be a named platform administrator.
///
/// TWO CHECKS, NOT ONE. The token proves who they are; `platform_admins` proves
/// what they may do. An owner token is a perfectly genuine token and must not
/// be enough to create a venue, or every restaurant owner on the platform could
/// mint hubs on the platform's domain.
pub(crate) async fn admin_only(
    req: &Request,
    ctx: &RouteContext<()>,
    _db: &D1Database,
) -> std::result::Result<String, Response> {
    let bearer = auth::bearer(req).map_err(|e| e.into_response().unwrap())?;
    let user_id = match auth::verify(&ctx.env, &bearer, now_ms()) {
        Ok(auth::Claims::Owner { user_id, .. }) => user_id,
        Ok(_) => return Err(Response::error("forbidden role", 403).unwrap()),
        Err(e) => return Err(e.into_response().unwrap()),
    };

    #[derive(Deserialize)]
    struct Row {
        user_id: String,
    }
    let row = crate::identity_store::identity(&ctx.env)
        .await
        .map_err(|e| Response::error(format!("store: {e}"), 500).unwrap())?
        .get(crate::identity_store::K_ADMIN, &user_id)
        .map(|_| Row { user_id: user_id.clone() });

    match row {
        Some(r) => Ok(r.user_id),
        // NOT 403 WITH A REASON. "You are not a platform admin" tells an owner
        // that this surface exists and that there is a table to be added to.
        // It is the platform's own door; a caller who is not behind it learns
        // only that there is nothing here.
        None => Err(Response::error("not found", 404).unwrap()),
    }
}

/// `GET /api/platform/hubs` — every client hub, newest first.
pub async fn hubs(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    if let Err(r) = admin_only(&req, &ctx, &db).await {
        return Ok(r);
    }
    let platform = ctx
        .var("PLATFORM_HOST")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "dowiz.org".to_string());

    #[derive(Deserialize)]
    struct Row {
        id: String,
        slug: String,
        name: String,
        status: String,
        created_at_ms: i64,
    }
    // `ORDER BY created_at_ms DESC` over a handful of venues is a sort, not an
    // index: the platform has two of them and will not have thousands before
    // the registry gains a key for it.
    let mut rows: Vec<Row> = crate::identity_store::registry(&ctx.env)
        .await?
        .all(crate::identity_store::K_LOC)
        .into_iter()
        .filter_map(|(_, j)| serde_json::from_str::<Row>(&j).ok())
        .collect();
    rows.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms));

    let out: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "slug": r.slug,
                "name": r.name,
                "status": r.status,
                "createdAtMs": r.created_at_ms,
                // The address the client actually uses. Built here rather than
                // in the console so one reader decides what a hub's URL is.
                "host": format!("{}.{}", r.slug, platform),
                "url": format!("https://{}.{}/", r.slug, platform),
                "console": format!("https://{}.{}/admin/", r.slug, platform),
                "courier": format!("https://{}.{}/courier/", r.slug, platform),
            })
        })
        .collect();

    Response::from_json(&json!({ "hubs": out, "platform": platform }))
}

/// The catalogue object a brand-new venue starts with.
///
/// EVERY FIELD `LocRow` READS, and that is not defensive padding. The public
/// menu route deserialises this object into a struct whose fields are not
/// `#[serde(default)]`, so a seed missing one does not degrade -- it fails to
/// parse and the storefront answers 500. A four-key seed did exactly that, and
/// it reached the live service because nothing here had ever been parsed by the
/// reader that consumes it. `the_seed_is_readable_by_the_storefront` now is.
///
/// The values are the `locations` table's own defaults, so a venue created here
/// and a venue created by a migration start identical.
pub(crate) fn location_seed(
    id: &str,
    slug: &str,
    name: &str,
    phone: &str,
) -> serde_json::Value {
    json!({
        "id": id,
        "slug": slug,
        "name": name,
        "phone": phone,
        "address": serde_json::Value::Null,
        "status": "closed",
        "closes_at": serde_json::Value::Null,
        "delivery_eta": "30-45",
        "delivery_fee": 0,
        "free_delivery_threshold": serde_json::Value::Null,
        "min_order": 0,
        "currency_code": "ALL",
        "menu_version": 1,
        "supported_locales": "[\"sq\",\"en\",\"uk\"]",
        "default_locale": "sq",
        "delivery_paused": 0,
    })
}

#[derive(Deserialize)]
pub struct NewHub {
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub owner: Option<NewOwner>,
}

#[derive(Deserialize)]
pub struct NewOwner {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// `POST /api/platform/hubs` — bring a client's hub into existence.
///
/// EVERYTHING IS CHECKED BEFORE ANYTHING IS WRITTEN. A half-created venue --
/// a `locations` row with no owner, or an owner with no membership -- is a hub
/// nobody can sign into and that the platform still lists as a client. D1 has
/// no transaction across statements here, so the order is: validate, then check
/// the slug is free, then write the venue, then the owner, then the membership.
/// The last two are `ON CONFLICT DO NOTHING` so a retry of a partly-applied
/// call converges instead of failing.
pub async fn create_hub(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let admin = match admin_only(&req, &ctx, &db).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let body: NewHub = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };

    let slug = body.slug.trim().to_ascii_lowercase();
    if let Some(why) = slug_problem(&slug) {
        return Response::error(format!("slug refused: {why}"), 400);
    }
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Response::error("a venue needs a name", 400);
    }

    // FREE BEFORE WRITTEN. The column is UNIQUE, so a race still ends in a
    // constraint error rather than a duplicate; this exists to answer the
    // ordinary case with a sentence instead of a SQL error.
    let mut owner_email: Option<String> = None;
    // The venue id and the slug are the same string at birth. They are separate
    // fields because a venue may be renamed on the web without its hub, its
    // Durable Object and everything that names it moving with it -- see `Place`.
    let now = now_ms();
    let id = slug.clone();

    // THE SLUG CHECK AND THE WRITE ARE ONE TURN. `SELECT ... LIMIT 1` followed
    // by an INSERT is a check that can be stale by the time it is acted on; the
    // uniqueness is a key here, and the object runs the two in one go.
    let (i2, s2, n2, ph2) = (id.clone(), slug.clone(), name.clone(), body.phone.trim().to_string());
    let taken = crate::identity_store::with_registry(&ctx.env, move |t| {
        if t.lookup(&crate::identity_store::loc_by_slug(&s2)).is_some() {
            return Ok(true);
        }
        let rec = serde_json::json!({
            "id": i2, "slug": s2, "name": n2, "phone": ph2,
            "status": "closed", "created_at_ms": now, "updated_at_ms": now,
        })
        .to_string();
        t.put(
            crate::identity_store::K_LOC,
            &i2,
            &rec,
            &[(crate::identity_store::loc_by_slug(&s2), i2.clone())],
            &[&crate::identity_store::loc_by_slug(&s2)],
        )
        .map_err(|e| Error::RustError(format!("registry: {e}")))?;
        Ok(false)
    })
    .await?;
    if taken {
        return Response::error(format!("slug '{slug}' is already a hub"), 409);
    }

    if let Some(o) = &body.owner {
        let email = o.email.trim().to_lowercase();
        let hash = crate::auth::hash_password(&o.password)
            .map_err(|e| Error::RustError(format!("cannot hash password: {e:?}")))?;
        let uid = crate::edge_id().ok_or_else(|| Error::RustError("no CSPRNG".into()))?;
        // ONE TURN AGAIN, and it replaces an `ON CONFLICT DO NOTHING` followed
        // by a read-back whose comment explained the subtlety: the email may
        // already belong to someone, in which case THAT person becomes the
        // owner and no second account is made. Here the existing holder is
        // simply looked up first.
        let (e2, n3, h2, u2, loc2) = (email.clone(), o.name.clone().unwrap_or_default(), hash, uid, id.clone());
        crate::identity_store::with_identity(&ctx.env, move |t| {
            let user_id = match crate::identity_store::user_id_for_email(t, &e2) {
                Some(existing) => existing,
                None => {
                    let rec = serde_json::json!({
                        "id": u2, "email": e2, "display_name": n3,
                        "password_hash": h2, "created_at_ms": now,
                    })
                    .to_string();
                    t.put(
                        crate::identity_store::K_USER,
                        &u2,
                        &rec,
                        &[(crate::identity_store::user_by_email(&e2), u2.clone())],
                        &[&crate::identity_store::user_by_email(&e2)],
                    )
                    .map_err(|e| Error::RustError(format!("user: {e}")))?;
                    u2.clone()
                }
            };
            let m = serde_json::json!({
                "user_id": user_id, "location_id": loc2,
                "role": "owner", "status": "active", "created_at_ms": now,
            })
            .to_string();
            t.put(
                crate::identity_store::K_MEMBER,
                &crate::identity_store::member_id(&loc2, &user_id),
                &m,
                &[
                    (
                        crate::identity_store::member_by_venue(&loc2, &user_id),
                        user_id.clone(),
                    ),
                    (
                        crate::identity_store::member_by_user(&user_id, &loc2),
                        loc2.clone(),
                    ),
                ],
                &[],
            )
            .map_err(|e| Error::RustError(format!("membership: {e}")))
        })
        .await?;
        owner_email = Some(email);
    }

    // THE CATALOGUE IMAGE IS SEEDED WITH THE VENUE, and skipping this made a
    // "created" hub one whose storefront answered 404.
    //
    // `locations` in D1 is the pointer; the CATALOGUE IMAGE is the venue. The
    // public menu route reads the image, finds the location there, and checks
    // its slug against the URL -- so a hub with a D1 row and an empty image is
    // a hub the platform lists, whose owner can sign in, and whose storefront
    // tells a customer the restaurant does not exist. Writing one object into
    // the image is what makes the hub real.
    //
    // It is written AFTER the D1 row rather than before, because a venue whose
    // image exists and whose row does not is unreachable by every lookup here.
    let seed = location_seed(&id, &slug, &name, body.phone.trim());
    let place = crate::hubstore::Place::of(&req, &ctx, Some(&id))?;
    let seeded = crate::hubstore::seed_fresh_hub(
        &place,
        &serde_json::to_string(&seed).unwrap_or_else(|_| "{}".into()),
    )
    .await;
    if let Err(e) = seeded {
        // SAY SO RATHER THAN REPORT A HUB THAT HALF EXISTS. The row is written
        // by now, so the honest answer names what is missing and leaves the
        // slug taken -- re-running with the same slug returns 409 and the
        // operator can see the hub in the list without a working storefront.
        return Response::error(
            format!("venue row created but its hub images were not written: {e}"),
            500,
        );
    }

    let platform = ctx
        .var("PLATFORM_HOST")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| "dowiz.org".to_string());
    Response::from_json(&json!({
        "id": id,
        "slug": slug,
        "name": name,
        "status": "closed",
        "ownerEmail": owner_email,
        "createdBy": admin,
        "host": format!("{slug}.{platform}"),
        "url": format!("https://{slug}.{platform}/"),
        "console": format!("https://{slug}.{platform}/admin/"),
        "courier": format!("https://{slug}.{platform}/courier/"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slug becomes a public hostname, so the things a hostname cannot hold
    /// are the things this must refuse -- and the platform's own names are the
    /// ones it must refuse hardest.
    #[test]
    fn a_slug_that_cannot_be_a_hostname_is_refused() {
        assert_eq!(slug_problem("sushi-durres"), None);
        assert_eq!(slug_problem("dubin2"), None);

        assert!(slug_problem("").is_some());
        assert!(slug_problem("Sushi").is_some(), "uppercase");
        assert!(slug_problem("sushi durres").is_some(), "space");
        assert!(slug_problem("sushi.durres").is_some(), "dot makes a second label");
        assert!(slug_problem("-sushi").is_some(), "leading hyphen");
        assert!(slug_problem("sushi-").is_some(), "trailing hyphen");
        assert!(slug_problem("sushi_durres").is_some(), "underscore");
        assert!(slug_problem(&"a".repeat(64)).is_some(), "too long for a label");
        assert!(slug_problem("xn--fsq").is_some(), "punycode form");
    }

    /// THE SEED MUST BE READABLE BY THE THING THAT READS IT. A hub is created
    /// here and its storefront is served by `storefront::menu`, which parses
    /// this object into `LocRow` -- a struct with no `#[serde(default)]` on any
    /// field. The first seed carried four keys, parsed nowhere in this crate,
    /// and every new venue's storefront answered 500 until a live request said
    /// so. This is that live request, made at compile time: add a field to
    /// `LocRow` without adding it here and this fails.
    #[test]
    fn the_seed_is_readable_by_the_storefront() {
        let seed = location_seed("sushi-durres", "sushi-durres", "Sushi Durrës", "+355 69 000 0000");
        let text = serde_json::to_string(&seed).expect("seed serialises");
        let parsed: crate::storefront::LocRow =
            serde_json::from_str(&text).expect("the storefront must be able to read a fresh venue");
        assert_eq!(parsed.slug, "sushi-durres");
        // The slug check in `menu` compares this against the URL, so a seed
        // whose slug disagreed would 404 its own storefront.
        assert_eq!(parsed.id, "sushi-durres");
        assert_eq!(parsed.status, "closed", "a new venue must not open itself");
    }

    /// The platform answers to these names itself. A venue that took one would
    /// own a name the platform needs -- `www.dowiz.org` most of all.
    #[test]
    fn the_platforms_own_names_cannot_be_taken() {
        for name in ["www", "api", "admin", "courier", "platform"] {
            assert!(
                slug_problem(name).is_some(),
                "'{name}' was available and it must not be"
            );
        }
    }
}

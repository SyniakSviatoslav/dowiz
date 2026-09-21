//! THE ONLY PLACE SQL IS STILL ALLOWED, and only to empty the database.
//!
//! The no-SQL ratchet (`tools/gates/no-sql.sh`) counts prepared statements in
//! `workers/api/src/*.rs` and refuses any commit that adds one. A migration
//! route legitimately needs to READ the table it is emptying, so it would fight
//! the ratchet for as long as the migration takes -- and the way that fight
//! usually ends is the baseline being raised "just this once".
//!
//! So the exemption is a FILE, named in the gate, and the file holds nothing
//! else. Every statement in here reads a table that is on its way out, and this
//! whole module is deleted in the same commit as the D1 binding. An exemption
//! that is one whole file is one you can see the size of; an exemption that is
//! a comment marker is one that spreads.
//!
//! Every route here is administrators-only and IDEMPOTENT: a migration you
//! cannot run twice is a migration you cannot verify.

use worker::*;

use worker::wasm_bindgen::JsValue;

use crate::hubstore::{from_hex, SLICE, SLICES};
use crate::platform::admin_only;


/// `POST /api/platform/migrate/i18n` — move the translations into the venues'
/// own images. Administrators only, idempotent, and safe to run again.
///
/// WHY A ROUTE AND NOT A SCRIPT. The rows are in D1 and the destination is a
/// Durable Object; only the Worker can reach both. A wrangler script would have
/// to re-implement the object's chunking and its generation guard, which is
/// two more places the storage format has to be right.
///
/// WHICH VENUE DOES A ROW BELONG TO? `content_i18n` HAS NO VENUE COLUMN -- that
/// is the defect this migration ends -- so membership has to be INFERRED, and
/// the only honest source is each venue's own catalogue. A row whose entity id
/// is in no venue's catalogue is an ORPHAN: it is reported by id and left in
/// D1 rather than guessed into a venue, because guessing which restaurant a
/// string belongs to is exactly how this platform's six tenancy defects
/// happened.
///
/// A row matching TWO venues is reported as contested and also left. It cannot
/// happen with the ids this product mints, and if it ever does, a human should
/// see it rather than a loop should pick one.
pub async fn migrate_i18n(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    if let Err(r) = admin_only(&req, &ctx, &db).await {
        return Ok(r);
    }
    #[derive(serde::Deserialize)]
    struct Loc {
        id: String,
    }
    #[derive(serde::Deserialize)]
    struct Row {
        entity_type: String,
        entity_id: String,
        locale: String,
        field: String,
        value: String,
    }
    let locs: Vec<Loc> = db
        .prepare("SELECT id FROM locations")
        .all()
        .await?
        .results()?;
    let rows: Vec<Row> = db
        .prepare("SELECT entity_type,entity_id,locale,field,value FROM content_i18n")
        .all()
        .await?
        .results()?;

    // Every venue's own ids, from its own catalogue image.
    let mut owners: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let legacy = ctx.var("LEGACY_VENUE").ok().map(|v| v.to_string()).filter(|v| !v.is_empty());
    for l in &locs {
        let (Ok(d), Ok(ns)) = (ctx.d1("DB"), ctx.durable_object("HUB")) else { continue };
        let place = crate::hubstore::Place {
            db: d,
            ns,
            venue: l.id.clone(),
            legacy_venue: legacy.clone(),
        };
        let cat = crate::hubstore::load_catalog(&place).await?.catalog;
        let mut ids: Vec<String> = cat.products().into_iter().map(|(id, _)| id).collect();
        ids.extend(cat.categories().into_iter().map(|(id, _)| id));
        for id in ids {
            owners.entry(id).or_default().push(l.id.clone());
        }
    }

    let mut batches: std::collections::HashMap<String, Vec<(String, String, String, String, String)>> =
        std::collections::HashMap::new();
    let mut orphans: Vec<String> = Vec::new();
    let mut contested: Vec<String> = Vec::new();
    for r in rows {
        match owners.get(&r.entity_id).map(|v| v.as_slice()) {
            Some([venue]) => batches.entry(venue.clone()).or_default().push((
                r.entity_type,
                r.entity_id,
                r.locale,
                r.field,
                r.value,
            )),
            Some(_) => contested.push(r.entity_id),
            None => orphans.push(r.entity_id),
        }
    }

    let mut wrote = serde_json::Map::new();
    for (venue, batch) in batches {
        let (Ok(d), Ok(ns)) = (ctx.d1("DB"), ctx.durable_object("HUB")) else { continue };
        let place = crate::hubstore::Place {
            db: d,
            ns,
            venue: venue.clone(),
            legacy_venue: legacy.clone(),
        };
        let n = batch.len();
        // ONE IMAGE WRITE for the whole venue. Re-running replaces the same
        // keys with the same values, so the image's fold does not move and a
        // second run is free -- which is what "idempotent" has to mean for a
        // migration nobody wants to be afraid of.
        crate::hubstore::with_table(
            &place,
            crate::hubstore::IMAGE_I18N,
            crate::hubstore::I18N_BYTES,
            move |t| {
                for (entity_type, id, locale, field, value) in &batch {
                    let key = crate::hubstore::i18n_key(locale, entity_type, id, field);
                    if value.trim().is_empty() {
                        t.remove(crate::hubstore::I18N_KIND, &key);
                    } else {
                        t.put(crate::hubstore::I18N_KIND, &key, value, &[], &[])
                            .map_err(|e| Error::RustError(format!("i18n: {e}")))?;
                    }
                }
                Ok(())
            },
        )
        .await?;
        let root = crate::hubstore::load_table(
            &place,
            crate::hubstore::IMAGE_I18N,
            crate::hubstore::I18N_BYTES,
        )
        .await?
        .table
        .root();
        wrote.insert(venue, serde_json::json!({ "entries": n, "root": root }));
    }
    orphans.sort();
    orphans.dedup();
    contested.sort();
    contested.dedup();
    Response::from_json(&serde_json::json!({
        "wrote": wrote,
        "orphans": orphans,
        "contested": contested,
    }))
}

/// The translations a venue has NOT yet migrated, read from the table on its
/// way out.
///
/// WHY A FALLBACK EXISTS AT ALL. The read path now reads the venue's own image,
/// and that image is empty until `migrate_i18n` has run. Deploying the cutover
/// without one would mean every menu served in a second language falls back to
/// the venue's own words for however long it takes somebody to notice and run
/// the migration — visible, harmless, and entirely avoidable.
///
/// This is expand-and-contract as the no-SQL blueprint prescribes it: read the
/// new home, fall back to the old, SAY SO OUT LOUD, and delete the fallback
/// when the old home is empty. The loud part is what stops a fallback from
/// becoming the permanent path — a silent one would work forever and nobody
/// would ever finish the migration.
///
/// It returns `(entity_id, field) -> value`, the shape the menu wants.
pub async fn i18n_fallback(
    db: &D1Database,
    locale: &str,
    ids: &[String],
) -> Option<std::collections::HashMap<(String, String), String>> {
    #[derive(serde::Deserialize)]
    struct Row {
        entity_id: String,
        field: String,
        value: String,
    }
    /// D1 refuses a statement with more than a hundred bound values, and a
    /// 165-dish catalogue plus its headings is 186 of them. One bind is kept
    /// back for the locale.
    const MAX_BINDS: usize = 100;
    let mut out = std::collections::HashMap::new();
    for chunk in ids.chunks(MAX_BINDS - 1) {
        let marks =
            (2..chunk.len() + 2).map(|i| format!("?{i}")).collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT entity_id, field, value FROM content_i18n \
             WHERE locale = ?1 AND field IN ('name','description','ingredients') \
             AND entity_id IN ({marks})"
        );
        let mut binds: Vec<worker::wasm_bindgen::JsValue> = vec![locale.into()];
        binds.extend(chunk.iter().map(|i| i.clone().into()));
        let rows = match db.prepare(&sql).bind(&binds) {
            Ok(stmt) => stmt.all().await.and_then(|r| r.results::<Row>()),
            Err(e) => Err(e),
        };
        match rows {
            Ok(list) => {
                for r in list {
                    out.insert((r.entity_id, r.field), r.value);
                }
            }
            Err(e) => {
                console_error!("i18n fallback {locale}: {e}");
                return None;
            }
        }
    }
    if out.is_empty() {
        return None;
    }
    console_error!(
        "i18n fallback served {} entries for {locale}: THIS VENUE HAS NOT BEEN MIGRATED. \
         Run POST /api/platform/migrate/i18n.",
        out.len()
    );
    Some(out)
}

/// The D1 reader, kept for exactly one job: seeding an object that has never
/// held this image.
///
/// THE LAST PREPARED STATEMENT OUTSIDE THIS FILE, moved here rather than
/// deleted, because deleting it is a decision about whether every venue's
/// object has been written at least once — and that is checked against the
/// live platform, not asserted from the tree. It is called by
/// `hubstore::do_image` for the one venue named in `LEGACY_VENUE` and by
/// nothing else; when that variable is unset there is no caller at all.
pub(crate) async fn load_images_d1(
    db: &D1Database,
    ids: &[&str],
) -> Result<std::collections::HashMap<String, (Vec<u8>, i64)>> {
    /// The slices are SPELLED OUT rather than collected with `#[serde(flatten)]`
    /// into a map. Flatten needs `deserialize_any`, which serde-wasm-bindgen
    /// supports only partially, and a deserialiser that fails here fails for
    /// every image at once on a surface that cannot be tested from this box.
    /// Four named fields cannot do that, and the assert below is what keeps them
    /// honest with `SLICES`.
    #[derive(serde::Deserialize)]
    struct Row {
        id: String,
        generation: i64,
        h0: String,
        h1: String,
        h2: String,
        h3: String,
    }
    const _: () = assert!(SLICES == 4, "Row has exactly this many hN fields");
    // `id = ?n OR id LIKE ?n || '#%'` per image. Built rather than fixed
    // because the caller decides how many it needs, and a query per image is
    // the thing being removed.
    let mut wheres = Vec::new();
    let mut binds: Vec<JsValue> = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        wheres.push(format!("id = ?{n} OR id LIKE ?{n} || '#%'", n = i + 1));
        binds.push((*id).into());
    }
    // `substr` on a BLOB counts BYTES and is 1-based, so slice k starts at
    // k*SLICE+1. Past the end it yields an empty blob, and `hex` of that is the
    // empty string -- so a short image simply has empty trailing slices and
    // needs no length column that could disagree with the bytes.
    let cols: String = (0..SLICES)
        .map(|k| {
            format!(
                ", ifnull(hex(substr(image, {start}, {SLICE})), '') AS h{k}",
                start = k * SLICE + 1
            )
        })
        .collect();
    let sql = format!(
        "SELECT id, generation{cols} FROM hub_image WHERE {}",
        wheres.join(" OR ")
    );
    let rows: Vec<Row> = db.prepare(&sql).bind(&binds)?.all().await?.results()?;

    // DECODED AS EACH ROW IS CONSUMED, so a row's hex is freed before the next
    // row's is touched. Holding all of them and decoding afterwards would put
    // every image's hex -- twice the bytes of every image, by definition -- in
    // the isolate at once, which is the cost this whole change exists to avoid
    // paying. `into_iter` on the array moves each slice out and drops it at the
    // end of its own iteration.
    let mut by_base: std::collections::HashMap<String, Vec<(usize, Vec<u8>, i64)>> =
        std::collections::HashMap::new();
    for r in rows {
        let Some((base, n)) = ids.iter().find_map(|base| {
            if r.id == **base {
                Some(((*base).to_string(), 0usize))
            } else {
                r.id.strip_prefix(*base)
                    .and_then(|rest| rest.strip_prefix('#'))
                    .and_then(|n| n.parse::<usize>().ok())
                    .map(|n| ((*base).to_string(), n))
            }
        }) else {
            continue;
        };

        let hex_len = r.h0.len() + r.h1.len() + r.h2.len() + r.h3.len();
        let mut bytes = Vec::with_capacity(hex_len / 2);
        // IN SLICE ORDER. Out of order the image reassembles with its bytes
        // transposed, which reads as a corrupt arena rather than as a bug here.
        for (k, hex) in [r.h0, r.h1, r.h2, r.h3].into_iter().enumerate() {
            if !from_hex(&hex, &mut bytes) {
                return Err(Error::RustError(format!(
                    "image {base} chunk {} slice {k} is not hex",
                    r.id
                )));
            }
        }
        by_base.entry(base).or_default().push((n, bytes, r.generation));
    }

    // SORTED IN RUST, NOT IN SQL. `ORDER BY id` is a string sort, and a string
    // sort puts "log#10" before "log#2" -- so an image that ever reached ten
    // chunks would be reassembled with its bytes in the wrong order, which
    // reads as a corrupt store rather than as a sorting bug.
    let mut out: std::collections::HashMap<String, (Vec<u8>, i64)> =
        std::collections::HashMap::new();
    for base in ids {
        let Some(mut parts) = by_base.remove(*base) else { continue };
        parts.sort_by_key(|(n, _, _)| *n);
        // A tail with no head is not an image: chunk zero carries the
        // generation the guard is checked against, and assembling from chunk
        // one would silently drop the first 900 KB.
        if parts[0].0 != 0 {
            continue;
        }
        let total: usize = parts.iter().map(|(_, b, _)| b.len()).sum();
        // The FIRST chunk's buffer becomes the image, rather than a fresh one
        // it is copied into. Every image in this store is a single chunk today,
        // and the copy would be the largest allocation in the request.
        let mut it = parts.into_iter();
        let (_, mut buf, generation) = it.next().expect("checked non-empty above");
        buf.reserve_exact(total - buf.len());
        for (_, bytes, _) in it {
            buf.extend_from_slice(&bytes);
        }
        out.insert((*base).to_string(), (buf, generation));
    }
    Ok(out)
}

/// `POST /api/platform/migrate/all` — move every remaining table into the
/// images. Administrators only, idempotent, and safe to run again.
///
/// THE ORDER MATTERS AND IS NOT ALPHABETICAL. The registry first, because
/// everything else is keyed by a venue that has to exist; identity next,
/// because a session points at a person; the venue-scoped families last, one
/// object at a time.
///
/// IDEMPOTENT MEANS THE SAME BYTES, not "does not crash". Every write here is a
/// `put` under a key derived from the row, so a second run replaces each record
/// with itself and the image's fold does not move. The report carries each
/// image's root so two runs can be compared without trusting this sentence.
pub async fn migrate_all(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    // ── THE CHICKEN AND THE EGG, AND HOW IT IS SETTLED ──
    //
    // `admin_only` reads the identity image, and the identity image is empty
    // until THIS ROUTE has run. An administrator therefore cannot authenticate
    // to run the migration that would let them authenticate. Waiting until
    // after the deploy to discover that is how an outage becomes a long one.
    //
    // So the bootstrap secret is accepted as well. It is not a weaker door: it
    // already authorises seeding an entire venue, owner included, and it is a
    // 32-character secret that exists only as a Worker secret. Without it set
    // there is no second door at all, and the answer to a wrong one is the
    // same 404 `bootstrap` gives -- a 401 confirms there is something here.
    let by_secret = match ctx.env.secret("BOOTSTRAP_SECRET") {
        Ok(want) => {
            let want = want.to_string();
            let given = req
                .headers()
                .get("x-dowiz-bootstrap")
                .ok()
                .flatten()
                .unwrap_or_default();
            want.len() >= 32 && crate::bootstrap::secret_ok(&given, &want)
        }
        Err(_) => false,
    };
    if !by_secret {
        if let Err(r) = admin_only(&req, &ctx, &db).await {
            return Ok(r);
        }
    }
    let mut report = serde_json::Map::new();
    let legacy = ctx.var("LEGACY_VENUE").ok().map(|v| v.to_string()).filter(|v| !v.is_empty());

    // ── the registry ──────────────────────────────────────────────────────
    #[derive(serde::Deserialize)]
    struct Loc {
        id: String,
        slug: String,
        name: Option<String>,
        phone: Option<String>,
        status: Option<String>,
        created_at_ms: Option<i64>,
        updated_at_ms: Option<i64>,
    }
    let locs: Vec<Loc> = db
        .prepare("SELECT id,slug,name,phone,status,created_at_ms,updated_at_ms FROM locations")
        .all()
        .await?
        .results()?;
    let venues: Vec<String> = locs.iter().map(|l| l.id.clone()).collect();
    let n = locs.len();
    crate::identity_store::with_registry(&ctx.env, move |t| {
        for l in &locs {
            let rec = serde_json::json!({
                "id": l.id, "slug": l.slug,
                "name": l.name.clone().unwrap_or_default(),
                "phone": l.phone.clone().unwrap_or_default(),
                "status": l.status.clone().unwrap_or_else(|| "closed".into()),
                "created_at_ms": l.created_at_ms.unwrap_or(0),
                "updated_at_ms": l.updated_at_ms.unwrap_or(0),
            })
            .to_string();
            t.put(
                crate::identity_store::K_LOC,
                &l.id,
                &rec,
                &[(crate::identity_store::loc_by_slug(&l.slug), l.id.clone())],
                &[],
            )
            .map_err(|e| Error::RustError(format!("registry: {e}")))?;
        }
        Ok(())
    })
    .await?;
    report.insert("locations".into(), serde_json::json!(n));

    // ── identity ──────────────────────────────────────────────────────────
    #[derive(serde::Deserialize)]
    struct U {
        id: String,
        email: String,
        display_name: Option<String>,
        password_hash: Option<String>,
        created_at_ms: Option<i64>,
    }
    #[derive(serde::Deserialize)]
    struct M {
        user_id: String,
        location_id: String,
        role: String,
        status: String,
        created_at_ms: Option<i64>,
    }
    #[derive(serde::Deserialize)]
    struct A {
        user_id: String,
    }
    let users: Vec<U> = db
        .prepare("SELECT id,email,display_name,password_hash,created_at_ms FROM users")
        .all()
        .await?
        .results()?;
    let members: Vec<M> = db
        .prepare("SELECT user_id,location_id,role,status,created_at_ms FROM memberships")
        .all()
        .await?
        .results()?;
    let admins: Vec<A> = db.prepare("SELECT user_id FROM platform_admins").all().await?.results()?;
    let (nu, nm, na) = (users.len(), members.len(), admins.len());
    crate::identity_store::with_identity(&ctx.env, move |t| {
        for u in &users {
            let email = u.email.trim().to_ascii_lowercase();
            let rec = serde_json::json!({
                "id": u.id, "email": email,
                "display_name": u.display_name.clone().unwrap_or_default(),
                "password_hash": u.password_hash.clone().unwrap_or_default(),
                "created_at_ms": u.created_at_ms.unwrap_or(0),
            })
            .to_string();
            t.put(
                crate::identity_store::K_USER,
                &u.id,
                &rec,
                &[(crate::identity_store::user_by_email(&email), u.id.clone())],
                &[],
            )
            .map_err(|e| Error::RustError(format!("user: {e}")))?;
        }
        for m in &members {
            let rec = serde_json::json!({
                "user_id": m.user_id, "location_id": m.location_id,
                "role": m.role, "status": m.status,
                "created_at_ms": m.created_at_ms.unwrap_or(0),
            })
            .to_string();
            t.put(
                crate::identity_store::K_MEMBER,
                &crate::identity_store::member_id(&m.location_id, &m.user_id),
                &rec,
                &[
                    (
                        crate::identity_store::member_by_venue(&m.location_id, &m.user_id),
                        m.user_id.clone(),
                    ),
                    (
                        crate::identity_store::member_by_user(&m.user_id, &m.location_id),
                        m.location_id.clone(),
                    ),
                ],
                &[],
            )
            .map_err(|e| Error::RustError(format!("membership: {e}")))?;
        }
        for a in &admins {
            t.put(
                crate::identity_store::K_ADMIN,
                &a.user_id,
                &serde_json::json!({ "user_id": a.user_id }).to_string(),
                &[],
                &[],
            )
            .map_err(|e| Error::RustError(format!("admin: {e}")))?;
        }
        Ok(())
    })
    .await?;
    report.insert("users".into(), serde_json::json!(nu));
    report.insert("memberships".into(), serde_json::json!(nm));
    report.insert("platform_admins".into(), serde_json::json!(na));

    // ── couriers, their rosters and their invites ─────────────────────────
    #[derive(serde::Deserialize)]
    struct Cr {
        id: String,
        email_encrypted: Option<String>,
        email_hash: Option<String>,
        phone_encrypted: Option<String>,
        phone_hash: Option<String>,
        full_name_encrypted: Option<String>,
        password_hash: Option<String>,
        status: Option<String>,
        created_at_ms: Option<i64>,
    }
    #[derive(serde::Deserialize)]
    struct Rs {
        courier_id: String,
        location_id: String,
        role: Option<String>,
        added_at_ms: Option<i64>,
    }
    #[derive(serde::Deserialize)]
    struct Iv {
        id: String,
        location_id: String,
        created_by_owner_id: Option<String>,
        invited_phone_hash: Option<String>,
        invited_name: Option<String>,
        code_hash: Option<String>,
        expires_at_ms: Option<i64>,
        created_at_ms: Option<i64>,
        used_at_ms: Option<i64>,
        revoked_at_ms: Option<i64>,
    }
    let crs: Vec<Cr> = db
        .prepare(
            "SELECT id,email_encrypted,email_hash,phone_encrypted,phone_hash,\
             full_name_encrypted,password_hash,status,created_at_ms FROM couriers",
        )
        .all()
        .await?
        .results()?;
    let rss: Vec<Rs> = db
        .prepare("SELECT courier_id,location_id,role,added_at_ms FROM courier_locations")
        .all()
        .await?
        .results()?;
    let ivs: Vec<Iv> = db
        .prepare(
            "SELECT id,location_id,created_by_owner_id,invited_phone_hash,invited_name,\
             code_hash,expires_at_ms,created_at_ms,used_at_ms,revoked_at_ms FROM courier_invites",
        )
        .all()
        .await?
        .results()?;
    let (nc, nr, ni) = (crs.len(), rss.len(), ivs.len());
    crate::identity_store::with_couriers(&ctx.env, move |t| {
        for c in &crs {
            let rec = serde_json::json!({
                "id": c.id,
                "email_encrypted": c.email_encrypted.clone().unwrap_or_default(),
                "email_hash": c.email_hash.clone().unwrap_or_default(),
                "phone_encrypted": c.phone_encrypted.clone().unwrap_or_default(),
                "phone_hash": c.phone_hash.clone().unwrap_or_default(),
                "full_name_encrypted": c.full_name_encrypted.clone().unwrap_or_default(),
                "password_hash": c.password_hash.clone().unwrap_or_default(),
                "status": c.status.clone().unwrap_or_else(|| "active".into()),
                "created_at_ms": c.created_at_ms.unwrap_or(0),
            });
            let index = crate::identity_store::courier_index(&c.id, &rec);
            t.put(crate::identity_store::K_COURIER, &c.id, &rec.to_string(), &index, &[])
                .map_err(|e| Error::RustError(format!("courier: {e}")))?;
        }
        for r in &rss {
            let rec = serde_json::json!({
                "courier_id": r.courier_id, "location_id": r.location_id,
                "role": r.role.clone().unwrap_or_else(|| "courier".into()),
                "added_at_ms": r.added_at_ms.unwrap_or(0),
            })
            .to_string();
            t.put(
                crate::identity_store::K_ROSTER,
                &crate::identity_store::roster_id(&r.location_id, &r.courier_id),
                &rec,
                &[
                    (
                        crate::identity_store::roster_by_venue(&r.location_id, &r.courier_id),
                        r.courier_id.clone(),
                    ),
                    (
                        crate::identity_store::roster_by_courier(&r.courier_id, &r.location_id),
                        r.location_id.clone(),
                    ),
                ],
                &[],
            )
            .map_err(|e| Error::RustError(format!("roster: {e}")))?;
        }
        for i in &ivs {
            let ph = i.invited_phone_hash.clone().unwrap_or_default();
            let spent =
                i.used_at_ms.is_some() || i.revoked_at_ms.is_some();
            let rec = serde_json::json!({
                "id": i.id, "location_id": i.location_id,
                "created_by_owner_id": i.created_by_owner_id.clone().unwrap_or_default(),
                "role": "courier",
                "invited_email_hash": ph, "invited_phone_hash": ph,
                "invited_name": i.invited_name.clone().unwrap_or_default(),
                "code_hash": i.code_hash.clone().unwrap_or_default(),
                "expires_at_ms": i.expires_at_ms.unwrap_or(0),
                "created_at_ms": i.created_at_ms.unwrap_or(0),
                "used_at_ms": i.used_at_ms,
                "revoked_at_ms": i.revoked_at_ms,
            })
            .to_string();
            // A SPENT INVITE KEEPS NO PHONE INDEX. That is the rule the live
            // code enforces on use and revocation, and a migration that carried
            // one across would make a used code claimable again.
            let mut index =
                vec![(crate::identity_store::invite_at(&i.location_id, &i.id), i.id.clone())];
            if !spent && !ph.is_empty() {
                index.push((crate::identity_store::invite_by_phone(&ph), i.id.clone()));
            }
            t.put(crate::identity_store::K_INVITE, &i.id, &rec, &index, &[])
                .map_err(|e| Error::RustError(format!("invite: {e}")))?;
        }
        Ok(())
    })
    .await?;
    report.insert("couriers".into(), serde_json::json!(nc));
    report.insert("courier_locations".into(), serde_json::json!(nr));
    report.insert("courier_invites".into(), serde_json::json!(ni));

    // ── sessions ──────────────────────────────────────────────────────────
    //
    // Refresh tokens are carried across rather than dropped. Dropping them is
    // defensible — they expire anyway — but it signs every owner and courier
    // out at the moment of the deploy, which is the worst time to ask somebody
    // to find their password.
    #[derive(serde::Deserialize)]
    struct Rt {
        user_id: String,
        family_id: String,
        token_hash: String,
        used: Option<i64>,
        expires_at_ms: i64,
        created_at_ms: Option<i64>,
    }
    #[derive(serde::Deserialize)]
    struct Ak {
        id: String,
        location_id: String,
        owner_id: Option<String>,
        label: Option<String>,
        key_hash: String,
        created_at_ms: Option<i64>,
        expires_at_ms: i64,
        last_used_ms: Option<i64>,
        revoked_at_ms: Option<i64>,
    }
    #[derive(serde::Deserialize)]
    struct Cs {
        id: String,
        courier_id: String,
        family_id: Option<String>,
        token_hash: Option<String>,
        active_location_id: Option<String>,
        issued_at_ms: Option<i64>,
        expires_at_ms: i64,
        revoked_at_ms: Option<i64>,
    }
    let rts: Vec<Rt> = db
        .prepare(
            "SELECT user_id,family_id,token_hash,used,expires_at_ms,created_at_ms \
             FROM auth_refresh_tokens",
        )
        .all()
        .await?
        .results()?;
    let aks: Vec<Ak> = db
        .prepare(
            "SELECT id,location_id,owner_id,label,key_hash,created_at_ms,expires_at_ms,\
             last_used_ms,revoked_at_ms FROM owner_api_keys",
        )
        .all()
        .await?
        .results()?;
    let css: Vec<Cs> = db
        .prepare(
            "SELECT id,courier_id,family_id,token_hash,active_location_id,issued_at_ms,\
             expires_at_ms,revoked_at_ms FROM courier_sessions",
        )
        .all()
        .await?
        .results()?;
    let (nt, nk, ns) = (rts.len(), aks.len(), css.len());
    crate::identity_store::with_sessions(&ctx.env, move |t| {
        for r in &rts {
            let created = r.created_at_ms.unwrap_or(0);
            let rec = serde_json::json!({
                "user_id": r.user_id, "family_id": r.family_id,
                "used": r.used.unwrap_or(0) == 1,
                "expires_at_ms": r.expires_at_ms, "created_at_ms": created,
            })
            .to_string();
            t.put(
                crate::identity_store::K_REFRESH,
                &r.token_hash,
                &rec,
                &[(
                    crate::identity_store::refresh_family(&r.family_id, created),
                    r.token_hash.clone(),
                )],
                &[],
            )
            .map_err(|e| Error::RustError(format!("refresh: {e}")))?;
        }
        for k in &aks {
            let rec = serde_json::json!({
                "id": k.id, "location_id": k.location_id,
                "owner_id": k.owner_id.clone().unwrap_or_default(),
                "label": k.label.clone().unwrap_or_default(),
                "key_hash": k.key_hash,
                "created_at_ms": k.created_at_ms.unwrap_or(0),
                "expires_at_ms": k.expires_at_ms,
                "last_used_ms": k.last_used_ms, "revoked_at_ms": k.revoked_at_ms,
            })
            .to_string();
            t.put(
                crate::identity_store::K_APIKEY,
                &k.id,
                &rec,
                &[(crate::identity_store::apikey_at(&k.location_id, &k.id), k.id.clone())],
                &[],
            )
            .map_err(|e| Error::RustError(format!("api key: {e}")))?;
        }
        for c in &css {
            let rec = serde_json::json!({
                "courier_id": c.courier_id,
                "family_id": c.family_id.clone().unwrap_or_default(),
                "token_hash": c.token_hash.clone().unwrap_or_default(),
                "active_location_id": c.active_location_id.clone().unwrap_or_default(),
                "issued_at_ms": c.issued_at_ms.unwrap_or(0),
                "expires_at_ms": c.expires_at_ms, "revoked_at_ms": c.revoked_at_ms,
            })
            .to_string();
            t.put(crate::identity_store::K_CSESSION, &c.id, &rec, &[], &[])
                .map_err(|e| Error::RustError(format!("courier session: {e}")))?;
        }
        Ok(())
    })
    .await?;
    report.insert("auth_refresh_tokens".into(), serde_json::json!(nt));
    report.insert("owner_api_keys".into(), serde_json::json!(nk));
    report.insert("courier_sessions".into(), serde_json::json!(ns));

    // ── per venue ─────────────────────────────────────────────────────────
    let mut per_venue = serde_json::Map::new();
    for venue in &venues {
        let (Ok(d), Ok(ns_do)) = (ctx.d1("DB"), ctx.durable_object("HUB")) else { continue };
        let place = crate::hubstore::Place {
            db: d,
            ns: ns_do,
            venue: venue.clone(),
            legacy_venue: legacy.clone(),
        };
        let counts = migrate_venue(&db, &place, venue).await?;
        per_venue.insert(venue.clone(), counts);
    }
    report.insert("venues".into(), serde_json::Value::Object(per_venue));

    // THE ROOTS, so two runs can be compared without trusting the word
    // "idempotent". A second run must print the same four.
    let mut roots = serde_json::Map::new();
    for (name, t) in [
        ("registry", crate::identity_store::registry(&ctx.env).await?),
        ("identity", crate::identity_store::identity(&ctx.env).await?),
        ("sessions", crate::identity_store::sessions(&ctx.env).await?),
        ("couriers", crate::identity_store::couriers(&ctx.env).await?),
    ] {
        roots.insert(name.into(), serde_json::json!(t.root()));
    }
    report.insert("roots".into(), serde_json::Value::Object(roots));

    Response::from_json(&serde_json::Value::Object(report))
}

/// One venue's own families: assignments, shifts, customers, bookings, ledger,
/// threads and the inbox.
async fn migrate_venue(
    db: &D1Database,
    place: &crate::hubstore::Place,
    venue: &str,
) -> Result<serde_json::Value> {
    let mut out = serde_json::Map::new();

    // assignments and shifts, into the ops image
    #[derive(serde::Deserialize)]
    struct Asg {
        order_id: String,
        courier_id: String,
        assigned_at_ms: Option<i64>,
        picked_up_at_ms: Option<i64>,
        delivered_at_ms: Option<i64>,
        cash_due: Option<i64>,
        cash_collected: Option<i64>,
    }
    #[derive(serde::Deserialize)]
    struct Sh {
        id: String,
        courier_id: String,
        started_at_ms: Option<i64>,
        ended_at_ms: Option<i64>,
        deliveries: Option<i64>,
        cash_collected: Option<i64>,
    }
    let asgs: Vec<Asg> = db
        .prepare(
            "SELECT order_id,courier_id,assigned_at_ms,picked_up_at_ms,delivered_at_ms,\
             cash_due,cash_collected FROM courier_assignments WHERE location_id = ?1",
        )
        .bind(&[venue.into()])?
        .all()
        .await?
        .results()?;
    let shs: Vec<Sh> = db
        .prepare(
            "SELECT id,courier_id,started_at_ms,ended_at_ms,deliveries,cash_collected \
             FROM courier_shifts WHERE location_id = ?1",
        )
        .bind(&[venue.into()])?
        .all()
        .await?
        .results()?;
    let (na, nsh) = (asgs.len(), shs.len());
    crate::hubstore::with_table(
        place,
        crate::hubstore::IMAGE_OPS,
        crate::hubstore::OPS_BYTES,
        move |t| {
            for a in &asgs {
                let rec = serde_json::json!({
                    "order_id": a.order_id, "courier_id": a.courier_id,
                    "assigned_at_ms": a.assigned_at_ms.unwrap_or(0),
                    "cash_due": a.cash_due.unwrap_or(0),
                    "picked_up_at_ms": a.picked_up_at_ms,
                    "delivered_at_ms": a.delivered_at_ms,
                    "cash_collected": a.cash_collected,
                })
                .to_string();
                t.put("asg", &a.order_id, &rec, &[], &[])
                    .map_err(|e| Error::RustError(format!("assignment: {e}")))?;
            }
            // ONE SHIFT RECORD PER COURIER, keyed by the courier, so a history
            // of closed shifts collapses to the newest. The live code cannot
            // represent two open shifts and this must not import a pair.
            let mut newest: std::collections::HashMap<&str, &Sh> =
                std::collections::HashMap::new();
            for s in &shs {
                let keep = match newest.get(s.courier_id.as_str()) {
                    None => true,
                    Some(prev) => {
                        // An open shift always wins; otherwise the later start.
                        (s.ended_at_ms.is_none() && prev.ended_at_ms.is_some())
                            || (s.ended_at_ms.is_none() == prev.ended_at_ms.is_none()
                                && s.started_at_ms.unwrap_or(0)
                                    > prev.started_at_ms.unwrap_or(0))
                    }
                };
                if keep {
                    newest.insert(s.courier_id.as_str(), s);
                }
            }
            for (cid, s) in newest {
                let rec = serde_json::json!({
                    "id": s.id, "courier_id": cid,
                    "started_at_ms": s.started_at_ms.unwrap_or(0),
                    "ended_at_ms": s.ended_at_ms,
                    "deliveries": s.deliveries.unwrap_or(0),
                    "cash_collected": s.cash_collected.unwrap_or(0),
                })
                .to_string();
                t.put("shift", cid, &rec, &[], &[])
                    .map_err(|e| Error::RustError(format!("shift: {e}")))?;
            }
            Ok(())
        },
    )
    .await?;
    out.insert("courier_assignments".into(), serde_json::json!(na));
    out.insert("courier_shifts".into(), serde_json::json!(nsh));

    // customers
    #[derive(serde::Deserialize)]
    struct Cu {
        id: String,
        phone_hash: String,
        name: Option<String>,
        created_at_ms: Option<i64>,
    }
    let cus: Vec<Cu> = db
        .prepare("SELECT id,phone_hash,name,created_at_ms FROM customers WHERE location_id = ?1")
        .bind(&[venue.into()])?
        .all()
        .await?
        .results()?;
    let ncu = cus.len();
    crate::hubstore::with_table(
        place,
        crate::hubstore::IMAGE_PEOPLE,
        crate::hubstore::PEOPLE_BYTES,
        move |t| {
            for c in &cus {
                let rec = serde_json::json!({
                    "id": c.id, "phone_hash": c.phone_hash,
                    "name": c.name.clone().unwrap_or_default(),
                    "created_at_ms": c.created_at_ms.unwrap_or(0),
                })
                .to_string();
                t.put("cust", &c.phone_hash, &rec, &[], &[])
                    .map_err(|e| Error::RustError(format!("customer: {e}")))?;
            }
            Ok(())
        },
    )
    .await?;
    out.insert("customers".into(), serde_json::json!(ncu));

    // bookings: the reservations and their events, into one image, because a
    // status is the fold of the events and the two must not be two writes.
    #[derive(serde::Deserialize)]
    struct Rv {
        id: String,
        user_id: Option<String>,
        party: i64,
        slot_min: i64,
        occasion: Option<String>,
        contact_name: Option<String>,
        contact_phone: Option<String>,
        status: String,
        created_at_ms: Option<i64>,
    }
    #[derive(serde::Deserialize)]
    struct Rve {
        reservation_id: String,
        to_status: String,
        seq: i64,
        actor: Option<String>,
        reason: Option<String>,
        at_ms: Option<i64>,
    }
    let rvs: Vec<Rv> = db
        .prepare(
            "SELECT id,user_id,party,slot_min,occasion,contact_name,contact_phone,status,             created_at_ms FROM reservations WHERE location_id = ?1",
        )
        .bind(&[venue.into()])?
        .all()
        .await?
        .results()?;
    let rves: Vec<Rve> = db
        .prepare(
            "SELECT reservation_id,to_status,seq,actor,reason,at_ms FROM reservation_events \
             WHERE location_id = ?1",
        )
        .bind(&[venue.into()])?
        .all()
        .await?
        .results()?;
    let (nrv, nrve) = (rvs.len(), rves.len());
    let venue_owned = venue.to_string();
    crate::hubstore::with_table(
        place,
        crate::booking::IMAGE_BOOKINGS,
        crate::booking::BOOKINGS_BYTES,
        move |t| {
            for r in &rvs {
                let user_id = r.user_id.clone().filter(|u| !u.trim().is_empty());
                let rec = serde_json::json!({
                    "id": r.id, "location_id": venue_owned, "party": r.party,
                    "slot_min": r.slot_min,
                    "occasion": r.occasion.clone().unwrap_or_default(),
                    "contact_name": r.contact_name.clone().unwrap_or_default(),
                    "contact_phone": r.contact_phone.clone().unwrap_or_default(),
                    "status": r.status,
                    "created_at_ms": r.created_at_ms.unwrap_or(0),
                    "user_id": user_id,
                })
                .to_string();
                let index: Vec<(String, String)> = match &user_id {
                    Some(u) => vec![(
                        crate::booking::user_key(u, r.slot_min, &r.id),
                        r.id.clone(),
                    )],
                    None => vec![],
                };
                t.put("rsv", &r.id, &rec, &index, &[])
                    .map_err(|e| Error::RustError(format!("reservation: {e}")))?;
            }
            for e in &rves {
                let rec = serde_json::json!({
                    "to_status": e.to_status, "seq": e.seq,
                    "actor": e.actor.clone().unwrap_or_default(),
                    "reason": e.reason.clone().unwrap_or_default(),
                    "at_ms": e.at_ms.unwrap_or(0),
                })
                .to_string();
                t.put("ev", &crate::booking::ev_key(&e.reservation_id, e.seq), &rec, &[], &[])
                    .map_err(|x| Error::RustError(format!("reservation event: {x}")))?;
            }
            Ok(())
        },
    )
    .await?;
    out.insert("reservations".into(), serde_json::json!(nrv));
    out.insert("reservation_events".into(), serde_json::json!(nrve));

    // the ledger: a transaction carries its own postings, so the join happens
    // here and the two are never separable again.
    #[derive(serde::Deserialize)]
    struct Tx {
        id: String,
        kind: String,
        reverses: Option<String>,
        memo: Option<String>,
        at_ms: i64,
    }
    #[derive(serde::Deserialize)]
    struct Po {
        tx_id: String,
        account: String,
        minor: i64,
        currency: String,
    }
    let txs: Vec<Tx> = db
        .prepare(
            "SELECT id,kind,reverses,memo,at_ms FROM ledger_tx WHERE location_id = ?1 \
             ORDER BY at_ms ASC, id ASC",
        )
        .bind(&[venue.into()])?
        .all()
        .await?
        .results()?;
    let pos: Vec<Po> = db
        .prepare(
            "SELECT tx_id,account,minor,currency FROM ledger_postings WHERE location_id = ?1",
        )
        .bind(&[venue.into()])?
        .all()
        .await?
        .results()?;
    let ntx = txs.len();
    // An append log cannot be re-written, so this is written ONLY when the log
    // is empty. Re-running the migration against a log that already holds these
    // entries would append them a second time and the journal would double.
    let existing = crate::hubstore::load_log(place, crate::wallet::IMAGE_LEDGER)
        .await?
        .log
        .len();
    if existing == 0 {
        crate::hubstore::with_log(place, crate::wallet::IMAGE_LEDGER, move |log| {
            for t in &txs {
                let legs: Vec<serde_json::Value> = pos
                    .iter()
                    .filter(|p| p.tx_id == t.id)
                    .map(|p| {
                        serde_json::json!({
                            "account": p.account, "minor": p.minor, "currency": p.currency,
                        })
                    })
                    .collect();
                let rec = serde_json::json!({
                    "id": t.id, "kind": t.kind, "reverses": t.reverses,
                    "memo": t.memo.clone().unwrap_or_default(),
                    "at_ms": t.at_ms, "postings": legs,
                })
                .to_string();
                log.append("tx", &t.id, &rec)
                    .map_err(|e| Error::RustError(format!("ledger: {e:?}")))?;
            }
            Ok(())
        })
        .await?;
        out.insert("ledger_tx".into(), serde_json::json!(ntx));
    } else {
        // Reported rather than silently skipped: "0 written because there were
        // already 12 entries" is a different fact from "0 to write".
        out.insert(
            "ledger_tx".into(),
            serde_json::json!(format!("skipped: log already holds {existing}")),
        );
    }

    // the venue's pass key, into its settings
    #[derive(serde::Deserialize)]
    struct Pk {
        key_b64: String,
    }
    let pk: Option<Pk> = db
        .prepare("SELECT key_b64 FROM venue_pass_keys WHERE location_id = ?1")
        .bind(&[venue.into()])?
        .first(None)
        .await?;
    if let Some(pk) = pk {
        let key = pk.key_b64.clone();
        crate::hubstore::with_settings(place, move |s| {
            // NEVER OVERWRITTEN. A pass already issued was signed with whatever
            // the venue had; replacing the key would make every one of them
            // fail to verify.
            if s.get("venue.pass.key").filter(|v| !v.is_empty()).is_none() {
                s.set("venue.pass.key", &key);
            }
            Ok(())
        })
        .await?;
        out.insert("venue_pass_keys".into(), serde_json::json!(1));
    }

    Ok(serde_json::Value::Object(out))
}

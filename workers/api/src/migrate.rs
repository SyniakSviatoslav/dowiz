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

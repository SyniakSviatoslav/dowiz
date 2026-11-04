//! WHO SOMEONE IS, ANSWERED BY THE PLATFORM OBJECT IN A FEW BYTES (W-OWNER-CPU2):
//! `POST /fold/verdict`.
//!
//! WHAT IT REPLACES, MEASURED (wrangler tail on 4e42be75, dubin-durres, 2026-11-04, six
//! owner routes at once x3): every owner route that was ALREADY a relay still cost the
//! stateless Worker 6-15 ms of its Free-plan 10 ms -- ebills/sales 7-13, categories 7-10,
//! analytics 6-9, dashboard 13-15, products 10-15 and `exceededCpu` -- while the forwarded
//! `/api/owner/orders` cost 2-4. The difference is the authorisation: `owner_and_venue`
//! fetched the WHOLE identity image from `__platform` and `Table::load`ed it in the Worker
//! (`Verify::Reachable` walks every object and checks every crc) to read ONE membership
//! record; a staff token read identity AND sessions, a courier sessions AND couriers, an API
//! key read sessions and WROTE it back. The image grows with every person on the platform
//! and the Worker paid for all of them to learn about one.
//!
//! WHAT IT IS. The Worker still verifies the token (HMAC + `exp`, pure, no I/O) and still
//! DECIDES (`auth.rs`, `owner.rs`: the same checks, the same refusals, in the same order).
//! What it asks the platform object is the few RECORDS the decision reads -- the membership,
//! the session row, the roster mark -- and the object, which has the images in hand and a
//! 30 s budget, loads them in process and answers ~100-300 bytes. Authority is still
//! re-derived from the live images on every request: an owner removed a moment ago is
//! refused on the next call holding a valid token, exactly as before, because the object
//! reads what is stored now. Nothing is memoised here; the Worker's own memo
//! (`auth/memo.rs`) is unchanged and still never remembers an owner.
//!
//! ONLY THE PLATFORM OBJECT ANSWERS. A venue's object holds no identity image; asked, it
//! would read an empty one and say "nobody", which is a refusal that looks like a verdict.
//! It answers 409 instead, so a misrouted ask is loud.
//!
//! THE ONE DECISION TAKEN HERE is the API key's (`Ask::ApiKey`): its secret is checked
//! against the stored hash and `last_used_ms` is stamped in the same turn, because the
//! stamp is a write to the sessions image and a write belongs in the object that owns it.
//! The judgement itself is the pure `auth::verdict::judge_api_key`, shared with the tests.

use super::HubImages;
use crate::auth::verdict::{Answer, Ask};
use crate::identity_store as ids;
use crate::platform_store::{self, PLATFORM};
use crate::wire::{Call, Reply};
use dowiz_hub::table::Table;
use serde_json::{json, Value};
use worker::*;

impl HubImages {
    /// One platform image, loaded in this object's own memory; empty when it was never written.
    async fn platform_table(&self, image: &str) -> Result<Table> {
        let ceiling = platform_store::ceiling(image);
        match self.image(image).await? {
            Some((_, bytes)) => Table::load(&bytes, ceiling).map_err(|_| Error::RustError(format!("image {image} is unreadable"))),
            None => Table::create(ceiling).map_err(|_| Error::RustError(format!("cannot create image {image}"))),
        }
    }

    /// `POST /fold/verdict` with an `Ask` body: the records one sign-in decision reads.
    pub(super) async fn verdict_route(&self, mut req: Call) -> Result<Reply> {
        if !self.state.is_me(PLATFORM) {
            return Reply::error("verdict: only the platform object knows who someone is", 409);
        }
        let ask: Ask = crate::body::parse(&mut req).await?;
        let answer = match ask {
            Ask::Member { venue, user } => {
                let t = self.platform_table(platform_store::IDENTITY).await?;
                Answer { member: ids::membership(&t, &venue, &user), ..Answer::default() }
            }
            Ask::Owned { user } => {
                let t = self.platform_table(platform_store::IDENTITY).await?;
                Answer { owned: Some(ids::memberships_of(&t, &user)), ..Answer::default() }
            }
            Ask::Staff { venue, user, jti } => {
                let (ident, sess) = (self.platform_table(platform_store::IDENTITY).await?, self.platform_table(platform_store::SESSIONS).await?);
                Answer {
                    member: ids::membership(&ident, &venue, &user),
                    session: ids::rec(&sess, crate::services::identity::staff_rules::K_SSESSION, &jti),
                    ..Answer::default()
                }
            }
            Ask::Courier { venue, courier, jti } => {
                let (sess, crew) = (self.platform_table(platform_store::SESSIONS).await?, self.platform_table(platform_store::COURIERS).await?);
                Answer {
                    session: ids::rec(&sess, ids::K_CSESSION, &jti).filter(|r| ids::s_of(r, "courier_id") == courier),
                    rostered: Some(crew.get(ids::K_ROSTER, &ids::roster_id(&venue, &courier)).is_some()),
                    ..Answer::default()
                }
            }
            Ask::ApiKey { id, secret, now_ms } => return self.api_key_verdict(&id, &secret, now_ms).await,
        };
        Reply::from_json(&answer)
    }

    /// The API key judged and, when it holds, stamped `last_used_ms` -- in this turn, under the
    /// sessions image's own guard (`with_at`): a key that works must not stop working because the
    /// bookkeeping write lost a generation, so the stamp's failure is not the answer's.
    async fn api_key_verdict(&self, id: &str, secret: &str, now_ms: i64) -> Result<Reply> {
        let sess = self.platform_table(platform_store::SESSIONS).await?;
        let rec = ids::rec(&sess, ids::K_APIKEY, id);
        let judged = crate::auth::verdict::judge_api_key(rec.as_ref(), secret, now_ms);
        if let Ok((owner_id, location_id)) = &judged {
            let key_id = id.to_string();
            let Some(base) = self.state.base_env() else {
                return Reply::error("verdict: no platform to stamp a key against", 500);
            };
            let env = self.own_env(&base, PLATFORM);
            // BOXED: the write reaches this object again through `Stub::Own` -> `route`, and an
            // async cycle needs one heap hop to have a size (as `command_route` does).
            let _ = Box::pin(platform_store::with(&env, platform_store::SESSIONS, move |t| {
                if let Some(mut r) = ids::rec(t, ids::K_APIKEY, &key_id) {
                    r["last_used_ms"] = json!(now_ms);
                    let loc = ids::s_of(&r, "location_id");
                    let index = vec![(ids::apikey_at(&loc, &key_id), key_id.clone())];
                    t.put(ids::K_APIKEY, &key_id, &r.to_string(), &index, &[]).map_err(|e| Error::RustError(format!("api key: {e}")))?;
                }
                Ok(())
            }))
            .await;
            return Reply::from_json(&Answer { key: Some(json!({"owner_id": owner_id, "location_id": location_id})), ..Answer::default() });
        }
        let why = judged.err().unwrap_or("no such key");
        Reply::from_json(&Answer { refused: Some(why.to_string()), ..Answer::default() })
    }
}

/// The answer's size is the point: a membership record, a session row, a mark.
#[cfg(test)]
pub(crate) fn bytes_of(v: &Value) -> usize {
    v.to_string().len()
}

#[cfg(test)]
mod tests;

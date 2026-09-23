//! THE FORGET COMMAND, in the venue's object, in one turn (§3.3).
//!
//! The Worker has already decided WHO (it holds the secret the key is an HMAC
//! under); this does WHAT, with the pure functions of
//! `services::customers::forget`, and writes in the order that header argues
//! for: people, consent, the hot log with its declaration, then each archive.

use super::HubImages;
use crate::command::Refused;
use crate::services::customers::forget as pure;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use worker::*;

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ForgetIn {
    /// `customer_key`, 16 hex, derived in the Worker.
    pub key: String,
    pub reason: String,
    /// The owner acting.
    pub by: String,
    /// The Worker's clock (`clock.sh`): the declaration's instant and seq.
    pub now_ms: i64,
    /// The person's order ids, hot and archived, as the Worker's folds found them.
    pub orders: Vec<String>,
    /// The archives holding any of them.
    pub archives: Vec<String>,
    /// Card ids the person was filed under before the re-key.
    pub legacy: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ForgetOut {
    pub key: String,
    /// Records redacted by this run, hot and archived.
    pub redacted: usize,
    pub archives: Vec<(String, usize)>,
    /// What this run's declaration names (0: nothing new to declare).
    pub declared: usize,
    pub people: usize,
    pub consent_redacted: usize,
    pub consent_withdrawn: usize,
}

fn unreadable(what: &str) -> Error {
    Error::RustError(format!("{what} image is unreadable"))
}

impl HubImages {
    /// Write `bytes` over `id` at the generation it was read at; a moved
    /// generation is a refusal the owner can retry, never a silent loss.
    async fn put_or_refuse(&self, id: &str, gen: i64, bytes: &[u8]) -> Result<std::result::Result<(), Refused>> {
        Ok(match self.put_image(id, gen, bytes).await? {
            Some(_) => Ok(()),
            None => Err(Refused::Append(format!("{id} moved during forget; nothing after it was written"))),
        })
    }

    pub(super) async fn forget(&self, input: ForgetIn) -> Result<std::result::Result<ForgetOut, Refused>> {
        if !pure::is_customer_key(&input.key) {
            return Ok(Err(Refused::Invalid("not a customer key".into())));
        }
        let key = input.key.as_str();

        // 1. PEOPLE. Idempotent; nothing counts it.
        let people_img = crate::hubstore::IMAGE_PEOPLE;
        let mut people = 0;
        if let Some((meta, bytes)) = self.image(people_img).await? {
            let mut t = dowiz_hub::table::Table::load(&bytes, crate::hubstore::PEOPLE_BYTES)
                .map_err(|_| unreadable(people_img))?;
            people = pure::forget_people(&mut t, key, &input.legacy);
            if people > 0 {
                let out = t.to_bytes().map_err(|e| Error::RustError(format!("people: {e:?}")))?;
                if let Err(r) = self.put_or_refuse(people_img, meta.generation, &out).await? {
                    return Ok(Err(r));
                }
            }
        }

        // 2. CONSENT. Redacted in place and stopped on every channel.
        let consent_img = crate::services::customers::consent_log::IMAGE_CONSENT;
        let (cgen, mut consent) = match self.image(consent_img).await? {
            Some((m, b)) => (m.generation, dowiz_hub::logimage::LogImage::load(&b).map_err(|_| unreadable(consent_img))?),
            None => (0, dowiz_hub::logimage::LogImage::create_sized(64 * 1024).map_err(|_| unreadable(consent_img))?),
        };
        let forgot = dowiz_hub::consent::forget::forget(&mut consent, key, input.now_ms)
            .map_err(|e| Error::RustError(format!("consent: {e}")))?;
        if forgot != dowiz_hub::consent::forget::Forgot::default() {
            if let Err(r) = self.put_or_refuse(consent_img, cgen, &consent.to_bytes()).await? {
                return Ok(Err(r));
            }
        }

        // 3. THE LOG, and the archives. What earlier runs declared joins the
        // scope, because their phones are gone and no fold finds them again.
        let (log_gen, mut hot) = self.log_hub().await?;
        let (declared_orders, declared_archives) = pure::declared_scope(&hot, key);
        let orders: BTreeSet<String> = input.orders.into_iter().chain(declared_orders).collect();
        let wanted: BTreeSet<String> = input.archives.into_iter().chain(declared_archives).collect();
        let (mut pairs, mut gens) = (Vec::new(), Vec::new());
        for id in wanted.into_iter().filter(|id| crate::hubstore::is_archive_id(id)) {
            let Some((meta, bytes)) = self.image(&id).await? else { continue };
            let hub = dowiz_hub::Hub::load(&bytes).map_err(|_| unreadable(&id))?;
            gens.push(meta.generation);
            pairs.push((id, hub));
        }
        let act = pure::Act { key, by: &input.by, reason: &input.reason, now_ms: input.now_ms };
        let erased = pure::erase(&act, &orders, &mut hot, &mut pairs).map_err(Error::RustError)?;

        if erased.hot > 0 || erased.declared > 0 {
            if let Err(r) = self.write_both("forget", log_gen, &hot, None).await? {
                return Ok(Err(r));
            }
        }
        for ((id, n), (gen, (_, hub))) in erased.archived.iter().zip(gens.iter().zip(pairs.iter())) {
            if *n > 0 {
                if let Err(r) = self.put_or_refuse(id, *gen, &hub.to_bytes_trimmed()).await? {
                    return Ok(Err(r));
                }
            }
            // An archive is read once a night at most; do not keep it resident.
            self.mem.borrow_mut().remove(id);
        }

        Ok(Ok(ForgetOut {
            key: input.key,
            redacted: erased.hot + erased.archived.iter().map(|(_, n)| n).sum::<usize>(),
            archives: erased.archived,
            declared: erased.declared,
            people,
            consent_redacted: forgot.redacted,
            consent_withdrawn: forgot.withdrawn,
        }))
    }
}

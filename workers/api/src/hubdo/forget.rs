//! THE FORGET COMMAND, in the venue's object, in one turn (§3.3).
//!
//! The Worker has already decided WHO (it holds the secret the key is an HMAC
//! under); this does WHAT, with the pure functions of
//! `services::customers::forget`, and writes in the order that header argues
//! for: people, consent, the hot log with its declaration, then each archive.
//! G8 adds, before the log and with the same "idempotent, counted by no law"
//! standing as people and consent: the bookings image and the outbox. Every
//! step reaches the whole alias circle, not the one key the owner pressed.

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
    /// The alias circle (G8): every key the person is filed under, `key`
    /// included. Empty from an older caller: then `key` alone.
    #[serde(default)]
    pub keys: Vec<String>,
    /// Their reservations, picked by the Worker (`booking::forget::ids_of`).
    #[serde(default)]
    pub bookings: Vec<String>,
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
    /// Reservations emptied of the person (G8).
    #[serde(default)]
    pub bookings: usize,
    /// Waiting outbox entries about them, dropped (G8).
    #[serde(default)]
    pub queued: usize,
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
        // THE CIRCLE, `key` always in it. Each must be a key: a stray string
        // here would be a card or a consent act removed by a guess.
        let mut keys: BTreeSet<String> = input.keys.iter().cloned().collect();
        keys.insert(key.to_string());
        if !keys.iter().all(|k| pure::is_customer_key(k)) {
            return Ok(Err(Refused::Invalid("the circle holds something that is not a customer key".into())));
        }
        let circle: Vec<String> = keys.iter().cloned().collect();

        // 1. PEOPLE. Idempotent; nothing counts it.
        let people_img = crate::hubstore::IMAGE_PEOPLE;
        let mut people = 0;
        if let Some((meta, bytes)) = self.image(people_img).await? {
            let mut t = dowiz_hub::table::Table::load(&bytes, crate::hubstore::PEOPLE_BYTES)
                .map_err(|_| unreadable(people_img))?;
            people = pure::forget_people(&mut t, &circle, &input.legacy);
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
        let mut forgot = dowiz_hub::consent::forget::Forgot::default();
        for k in &circle {
            let f = dowiz_hub::consent::forget::forget(&mut consent, k, input.now_ms)
                .map_err(|e| Error::RustError(format!("consent: {e}")))?;
            forgot.redacted += f.redacted;
            forgot.withdrawn += f.withdrawn;
        }
        if forgot != dowiz_hub::consent::forget::Forgot::default() {
            if let Err(r) = self.put_or_refuse(consent_img, cgen, &consent.to_bytes()).await? {
                return Ok(Err(r));
            }
        }

        // 2b. BOOKINGS (G8): the reservation stays, the contact goes.
        let mut bookings = 0;
        if !input.bookings.is_empty() {
            let img = crate::booking::IMAGE_BOOKINGS;
            if let Some((meta, bytes)) = self.image(img).await? {
                let mut t = dowiz_hub::table::Table::load(&bytes, crate::booking::BOOKINGS_BYTES).map_err(|_| unreadable(img))?;
                bookings = crate::booking::forget::redact(&mut t, &input.bookings).map_err(Error::RustError)?;
                if bookings > 0 {
                    let out = t.to_bytes().map_err(|e| Error::RustError(format!("bookings: {e:?}")))?;
                    if let Err(r) = self.put_or_refuse(img, meta.generation, &out).await? {
                        return Ok(Err(r));
                    }
                }
            }
        }

        // 2c. THE OUTBOX (G8): tickets and campaign messages still waiting
        // about them are dropped, never sent after the erasure.
        let order_set: BTreeSet<String> = input.orders.iter().cloned().collect();
        let mut queued = 0;
        let img = crate::outbox::IMAGE_OUTBOX;
        if let Some((meta, bytes)) = self.image(img).await? {
            let mut t = dowiz_hub::table::Table::load(&bytes, crate::outbox::OUTBOX_BYTES).map_err(|_| unreadable(img))?;
            queued = crate::services::customers::forget::queued::drop_queued(&mut t, &order_set, &keys);
            if queued > 0 {
                let out = t.to_bytes().map_err(|e| Error::RustError(format!("outbox: {e:?}")))?;
                if let Err(r) = self.put_or_refuse(img, meta.generation, &out).await? {
                    return Ok(Err(r));
                }
            }
        }

        // 3. THE LOG, and the archives. What earlier runs declared joins the
        // scope, because their phones are gone and no fold finds them again.
        let (log_gen, mut hot) = self.log_hub().await?;
        let (declared_orders, declared_archives) = pure::declared_scope(&hot, key);
        let orders: BTreeSet<String> = order_set.into_iter().chain(declared_orders).collect();
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
            bookings,
            queued,
        }))
    }
}

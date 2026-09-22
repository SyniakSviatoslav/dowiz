# CRM, consent, loyalty and erasure: what a customer is in this tree, what may be written about them, what may be sent to them, and how they are forgotten

**Date:** 2026-09-22. **HEAD read:** `7b0c9871` ("roadmap: one short entry point", 2026-09-22); the tree
moved from `b949528e` to `7b0c9871` while this was written, by a commit that touches only
`docs/design/ROADMAP.md` and its entry point.
**Tree state at time of reading:** 12 paths modified or untracked (`git status --short | wc -l` = 12),
belonging to the booking and fiscal lanes: `crates/dowiz-hub/src/lib.rs` (+1 line, a `pub mod`),
`crates/dowiz-hub/src/tables{.rs,/}`, `workers/api/src/booking.rs`, four storefront files. **Line numbers
for `crates/dowiz-hub/src/lib.rs` below are from HEAD** (`git show HEAD:…`), not the working tree, and will
drift by one once that lane lands. `booking.rs` is cited from the working tree and marked so.

**Method.** Every "today" statement names a path and a line where one was read. `measured` means a command
was run on this box and its output is quoted; `hypothesis` means it was not. Where the brief that asked for
this document and the tree disagree, §0 says so first. Nothing about the code was taken from a document
about the code when the code itself was readable; the two design documents that *were* read for what they
decided (`owner_surface.rs`'s erasure port, the P0 privacy counsel opinion) are cited as decisions, not as
facts about the tree.

**What this document is for.** The roadmap already asked for it: the campaign lane "is blocked on its own
consent-ledger mini-blueprint" because "recipient lists are PII" (`docs/design/ROADMAP.md:1692`, now marked
history by `7b0c9871` but still the only statement of that requirement). This is that blueprint, widened
to the four things a consent ledger is useless without: a record to attach consent to, an identity to attach
the record to, an erasure that survives the append-only log, and a clear statement of what loyalty may and
may not be under `CLAUDE.md:102`.

---

## 0. The brief's six facts, checked against the tree

| # | Claim in the brief | Verdict | Evidence |
|---|---|---|---|
| 1 | "A customer is a FOLD, not a record … there is nowhere to write anything down" | **Half wrong, and the wrong half matters.** A record IS written: every placement with a phone upserts `{id, phone_hash, name, created_at_ms}` into the venue's `people` Table under kind `cust` (`workers/api/src/storefront.rs:1159-1206`, `t.put("cust", …)` at `:1202`). **Nothing reads it** — `grep -rn '"cust"' workers/api/src` finds the write and two idempotency test fixtures (measured). And its id is `auth::sha256_hex(&body.contact.phone)` (`storefront.rs:1071`), an UNKEYED hash of the raw string, which is not the handle anything else uses (§1.1). So: a record exists, holds nothing anyone wants, is keyed so that nothing can join to it, and is a phone directory behind a hash that enumerates in seconds. The brief's conclusion stands; its premise does not. | `storefront.rs:1071, 1159-1206`; `hubstore.rs:831-833` (`IMAGE_PEOPLE`, "the customer registry that `customers` was") |
| 2 | No record of marketing consent | **True.** `grep -rn -i 'consent\|marketing\|opt.in\|unsubscribe'` over `workers/api/src`, `crates/dowiz-hub/src`, `crates/dowiz-core/src` and `workers/api/public`: zero hits in product code (measured; the only matches are `maplibre-gl.js`, `gossip.rs`, `hydra.rs`, `intake.rs`). The checkout asks for a name and an optional phone and nothing else (`public/store/checkout.js:165-168`). | measured grep |
| 3 | No segments, campaigns or send history; the `outbox` is per order | **True.** `outbox::Entry.id` is "the ORDER's id plus the kind" (`workers/api/src/outbox.rs:65-68`); `kind` is a closed set and `to` is "a Telegram chat id, today" (`:69-77`). Nothing in the tree sends a message to a customer that the customer did not start: `notify.rs` tells the kitchen (`notify.rs:1-10`), `channels.rs` sends a reply inside a WhatsApp/Instagram thread the customer opened (`channels.rs:1-19`). | `outbox.rs:33-35, 62-82` |
| 4 | No loyalty; the wallet is money | **True.** `crates/dowiz-core/src/wallet/mod.rs:15-18` ("Money is `crate::money::Money` … NO CRDT — single-writer LWW is strictly correct"); the Worker's `wallet.rs` is "the transport for `dowiz_kernel::ledger_account`" with "no `balance` column anywhere … and there must never be one" (`workers/api/src/wallet.rs:1-16`). The nearest thing to loyalty is `dowiz_hub::promo` — "a REFUSAL ENGINE, not a calculator" whose used-count is folded from the order log (`crates/dowiz-hub/src/promo.rs:1-21`). No points, stamps or tiers anywhere (grep, measured). | as cited |
| 5 | "Someone who ordered by phone and someone who holds a wallet are two customers" | **Wrong.** The wallet key IS the customer key: `own_wallet_key` reads the phone off the token's order and calls `services::customers::handlers::customer_key` — "the same one the customer list and the reveal audit use, so a wallet and a customer row are the same person" (`workers/api/src/wallet.rs:352-374`). The multiplicity that does exist is different and worse — §1.1 counts FOUR handles for one phone. | `wallet.rs:352-374` |
| 6 | `reveal` with audit exists and is well made; "delete everything about me" does not | **True, with one addition.** The reveal appends `EventKind::Revealed` BEFORE answering (`services/customers/handlers.rs:91-159`, the ordering rule at `:88-90`) and the payload "carries no contact details itself" (`crates/dowiz-hub/src/lib.rs:73-80` at HEAD). No erasure route exists in the Worker. BUT the core crate already carries a designed-and-tested erasure port that nothing uses: `CustomerErasureAction`/`ErasureEvent`/`ErasureLedger` in `crates/dowiz-core/src/ports/owner_surface.rs:97-145, 818-830`, "crypto-erasure, §3.6", with tests `g6_erasure_removes_all_pii_folds` and `g6_erasure_is_owner_signed` (`:1555, :1587`); `grep -rn Erasure workers/api/src tools/native-spa-server/src | wc -l` = 0 (measured). §2.2 says why that design was right for the node it was written for and is not the one to build here. | as cited |

Two more claims in the brief's framing need correcting because the design leans on them:

- **"DECISIONS.md — trust is a signed capability, never a score, and a CI job fails the build…"** The sentence
  is in `CLAUDE.md:102-106`, not `DECISIONS.md`; `DECISIONS.md` carries the ruling that produced it, OD-8
  "Remove `reputation.rs` (courier-scoring red-line divergence)" (`DECISIONS.md:350-351`). **The CI job does
  not exist.** `grep -rn -i 'scor\|reputation\|rating' .github/workflows/*.yml` is empty, and
  `grep -rln courier_score` across `*.rs, *.sh, *.yml, *.toml` finds one file — the doc comment in
  `crates/dowiz-core/src/domain.rs:133-139` that describes the job (measured). What actually enforces the
  rule is the type system: the routing enums omit `Ord`. §3.5 answers the tier question against the rule as
  written, and §5 G6 proposes the job the comment promises, widened to cover this design.
- **"The nightly witness writes a census to a second object"** — true, and the witness is in the Worker,
  not the hub: `workers/api/src/witness/mod.rs:1-29` (census, tip written to the platform object's `witness`
  log AND the nightly S3 copy). It seals the ORDER log only — `night::take(place, hub: &dowiz_hub::Hub, …)`
  (`witness/night.rs:19-25`). The `LogImage` logs (`audit`, `inbox`, `ledger`, `outbox`) are not witnessed.
  That asymmetry is what makes §3.3 possible at a cost the witness can bear.

---

## 1. Ground truth

### 1.1 One phone, four handles

A customer today is whatever can be folded from the order log plus three side records, and the four
places disagree about what the person's key is:

| Where | Handle | How derived | Readers |
|---|---|---|---|
| Customer list, reveal audit, wallet account | `customer_key` | `hex(HMAC-SHA256(AUTH_SIGNING_KEY, digits-only phone))[..8]` — `services/customers/handlers.rs:18-22`; secret from `signing_secret` `:24-31` (falls back to `b"dowiz-unconfigured"`) | `roll.rs:64-74`; `handlers.rs:118`; `wallet.rs:359-374`; `Revealed` subject `cust:<key>` `handlers.rs:146` |
| `people` Table record `cust` | `sha256_hex(raw phone string)` | `storefront.rs:1071` | none (measured) |
| WhatsApp inbox thread | `wa_id` — the E.164 number without `+`, as Meta sends it | `channels.rs:240-246, 262-268` (`peer: from`), stored as subject `whatsapp/<peer>` (`channels.rs:318-320, 334-337`) | the owner's inbox `:441-470` |
| Booking (working tree, another lane) | `contact_phone` as typed | `booking.rs:81-82, 209-210` | the booking list |

Two spellings of one number are two `customer_key`s: `digits` strips punctuation but does not normalise
the country code, so `+355 69 123 4567` → `355691234567` and `069 123 4567` → `0691234567` hash apart. The
fold's own test covers the same spelling twice, not two spellings (`services/customers/tests.rs:37-50`,
both orders are `"+355 69 1"`). This is measured against the code, not the live venue; the live rate of
national-vs-international spellings is unknown (§7).

**The fold itself is sound and should stay a fold.** `roll` is pure, keyed by a function passed in "because
it needs the hub's signing secret, and a fold that needed a secret could not be tested" (`roll.rs:57-60`);
an order without a phone "is not a person" and is skipped (`roll.rs:61-63, 74-76`); money uses
`venue_took` so refused orders and tips do not inflate "spent" (`roll.rs:44-56`). 6 tests pass
(measured: `cd workers/api && cargo test --lib --offline customers` → `6 passed; 222 filtered out`).
The mask lives once, in `dowiz_hub::redact`, after `1992b498` found that "first three, five dots, last two"
showed all four digits of a four-digit number in both copies (`crates/dowiz-hub/src/redact.rs:20-55`,
5 tests, measured green). Everything derivable from orders — count, spend, last visit — must remain
derived, because a stored counter "is a second number that can disagree with them, and the one that
disagrees is always the counter" (`promo.rs:17-21`, the same law as `wallet.rs:11-16`).

### 1.2 What the log commits to, and where copies of it live

- **The payload is the whole order.** `Hub::append` builds `[kind][id_len][id][order_json]`
  (`crates/dowiz-hub/src/lib.rs:363-379`, HEAD) and `order_json` is the kernel's serialised order, whose
  `contact: Contact { phone, name }` (`crates/dowiz-core/src/domain.rs:94-97, 131`) and
  `fulfilment.address.line` travel in the clear. Every `Placed` and every delta `Advanced` that touches
  them is a record whose id commits to the phone number.
- **The id is `content_id_chained(prev, payload)`** = `content_id(prev ‖ payload)` (`lib.rs:822-827`), and
  `content_id` is **FNV-1a over four lanes, "deliberately NOT sha256: this crate has zero dependencies, and
  the cryptographic chain commitment lives in the kernel where the keys are"** (`lib.rs:854-857`). So the
  chain is an integrity check against accident and against an editor who does not recompute; it is not a
  commitment an adversary with write access cannot forge. The witness header says the same in its own words:
  "anyone who can write the image can recompute every id in it" (`witness/mod.rs:8-9`). Any erasure design
  that "works" by finding a payload with the same FNV id is therefore not a design; it is the attack.
- **`chain_check` already tolerates two id schemes** — `chained` and `legacy` (`lib.rs:676-690`), "which is
  a fact about them rather than a fault" (`:829-832`). A third class is precedent, not novelty.
- **The witness sees truncation and rewrite-from-scratch, not content.** It writes `(records, tip)` per
  image to the platform object and to S3 and checks the previous tip is still `holds()` (`witness/mod.rs`;
  `lib.rs:700-713`). A record whose payload changes but whose id is kept, followed by the same records, keeps
  the same tip and count. The witness cannot see it — and, as §3.3 argues, must be TOLD.
- **Copies.** (a) `rotate` moves records "VERBATIM — same ids, same `prev` links, same payloads" into an
  archive image (`lib.rs:580-583`), named in settings under `log.archives` (`hubstore.rs:1041`) and sealed
  once by the witness (`witness/mod.rs:42-45`). (b) The nightly cron `17 3 * * *` (`cloud.rs:27-28`;
  `wrangler.toml:133-138`) pushes `export` — `IMAGES = [log, catalog, settings, posts, stock]`
  (`hubstore.rs:1567-1568`) plus each archive once (`:1570-1571`, `archives_pending`) — to the venue's own
  S3-compatible bucket at **`<prefix>/<venue>/<YYYYMMDD>T<HHMMSS>Z.json[.gz]`** (`cloud.rs:345`, `push_place`
  `:381-385`). **Dated, so the copies accumulate**; there is no lifecycle rule in this tree (S3 lifecycle is
  bucket configuration, not code). `people`, `inbox`, `ledger`, `audit`, `outbox` are NOT in the bundle.
  (c) The Durable Object is SQLite-backed (`wrangler.toml:128-130`, `new_sqlite_classes = ["HubImages"]`),
  and Cloudflare keeps a 30-day point-in-time change log for that storage; the repo already names
  "Cloudflare's own thirty-day time travel" as the only other safety net (`hubstore.rs:1555-1560`).
  **Nothing written in this codebase can shorten that 30 days.** It is the floor under every erasure design
  in §2.2, and it is why the honest promise is "within a month", which is also the statutory one.

### 1.3 What can be sent to a customer today

- To the KITCHEN: Telegram (`notify::telegram`, `notify.rs:59`) and WhatsApp to the venue's own
  `notify.whatsapp.to` (`channels.rs:80-84`), now via the per-order `outbox` drained by the minute cron
  (`outbox.rs:14-25`; `wrangler.toml:138`, `"* * * * *"`).
- To a CUSTOMER: only a reply inside a thread the customer opened over Meta's webhook
  (`channels.rs:8-15`), which Meta itself confines to a 24-hour customer-care window. There is no route that
  sends to a phone number the venue chose. This is the property §5 G1 makes permanent for every send that
  is not consented.
- The storefront remembers `dw_name`, `dw_phone`, `dw_addr` in the diner's own `localStorage`
  (`checkout.js:344`) — on the device, never sent except inside an order. Worth naming because it is the one
  place a "remember me" already exists, and it is the right place for it.

### 1.4 What is already decided on paper

- `dowiz-core`'s erasure port (`owner_surface.rs:97-145`) was designed for the MANIFESTO node — a peer with
  a local database whose log is replicated to other nodes (MANIFESTO C4, §2). There, ciphertext + key
  destruction is the only mechanism that reaches a copy the node does not hold. Its `CustomerRef` says
  "There is NO durable customer identity (P49 deferral) — the only stable handle is the channel-shaped
  address plus the order-ids" (`:97-100`). That deferral is what this document ends.
- The P0 privacy counsel opinion already named the gap: contact details "immortalized in a Telegram history
  the customer never consented to and can never reach to delete" (`docs/design/p0-privacy-hardening/
  counsel-opinion.md:16`), and asked for a "what we keep about you and for how long" line for the courier as
  well as the customer (`:93`). Both are still open.
- `roll.rs` and `redact.rs` state the doctrine this design must keep: "the venue holds exactly what it
  held before" (`services/customers/mod.rs:3-8`). A CRM by definition makes the venue hold more. Every field
  in §3.1 has to justify itself against that sentence.

---

## 2. Research, made concrete for this codebase

### 2.1 Consent as a record

**What the law requires.** Albania's Law No. 124/2024 "On Personal Data Protection" entered into force on
31 January 2025, repealing Law 9887/2008 and aligning with Regulation (EU) 2016/679; direct marketing by
electronic means (SMS, email, automated calls, social/messaging) requires the recipient's prior, explicit,
informed consent, withdrawable at any time, and the controller must keep documented proof of it and offer
an easy withdrawal ([Karanovic & Partners](https://www.karanovicpartners.com/news/albania-adopts-new-direct-marketing-rules-aligned-with-gdpr/),
[IDP text](https://idp.al/wp-content/uploads/2025/04/Law-no.124-2024-DP.pdf), [KPMG](https://kpmg.com/al/en/insights/2025/02/new-law-on--personal-data-protection-.html)).
GDPR Art. 7(1): the controller "shall be able to demonstrate that the data subject has consented"; Art.
7(3): withdrawal "as easy as" giving; Recital 32: a pre-ticked box is not consent. Whether Albania carries
the ePrivacy "soft opt-in" for existing customers (Directive 2002/58 Art. 13(2)) was **not confirmed** in
what was read; this design does not rely on it (§7).

**What the platform requires, separately.** Meta's WhatsApp Business Messaging Policy requires an opt-in
before business-initiated messages outside the 24-hour window; since November 2024 a general marketing
opt-in collected on any channel satisfies Meta *provided it names the business and complies with local law*
([Meta developers](https://developers.facebook.com/documentation/business-messaging/whatsapp/getting-opt-in),
[Infobip](https://www.infobip.com/docs/whatsapp/compliance/user-opt-ins)). Meta's rule and the law are two
rule sets; passing one does not pass the other. Telegram has the opposite shape: a bot cannot message a user
who has not pressed Start, so the channel is opt-in by construction — but Start is consent to be messaged
by that bot, not consent to marketing, and the record must still say which.

**What a record must hold to be worth anything** — the ICO's formulation is "who, when, how, and what you
told people" ([ICO, Consent](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/lawful-basis/a-guide-to-lawful-basis/consent/)).
Made concrete:

| Field | Why it is load-bearing | Source in this tree |
|---|---|---|
| `key` (the pseudonymous handle) | "who", without holding the number in the record | `customer_key`, §1.1 |
| `purpose` | consent is per purpose; "marketing" is one, "loyalty count" would be another | Art. 6(1)(a), Recital 32 |
| `channel` | Meta's opt-in is per channel in practice; a WhatsApp consent is not a Telegram one | §2.1 |
| `state` = `given` \| `withdrawn` | the withdrawal is the one record you must keep to prove you stopped | Art. 7(3) |
| `at_ms` | "when"; from `ctx.data.now_ms`, never a handler clock (`tools/gates/clock.sh`) | measured gate |
| `method` | "how": `checkout_box` (unticked by default), `whatsapp_keyword` (the customer wrote START/STOP), `owner_entered` (paper, verbal) | Recital 32 |
| `evidence` | for `owner_entered` only: what the owner saw; without it the method is refused | ICO "how" |
| `wording_id` | "what you told people": the hash of the exact sentence and locale shown, with the sentence stored once under that id — the checkout copy exists in three languages (`public/store/i18n.js:1`) and each is its own wording | ICO "what" |
| `via` | the order id, thread key or owner id the act arrived on — the cross-reference a dispute needs | — |

Not a Table row that is overwritten: **an append-only log, folded to current state**. The witness module
already states the rule for evidence — "a census is evidence, and evidence is not updated in place"
(`witness/mod.rs:31-33`). Consent is evidence.

### 2.2 Erasure against an append-only, content-chained log — the decision

The tension is real and the four honest patterns each cost something specific here. Scored against §1.2:

| Pattern | What it does | Reaches the hot log | Reaches archives | Reaches the dated S3 copies | Reaches the 30-day PITR | Chain / witness | Cost in THIS tree |
|---|---|---|---|---|---|---|---|
| **A. Rebuild without the subject** (the `LogImage::keep` shape: "THE PRUNE IS A REBUILD … the ids CHANGE", `logimage.rs:232-243`) | drop the subject's records and re-chain | yes | yes, one rebuild per archive | no | no | **Breaks both by construction**: every id after the first removed record changes, the tip changes, last night's tip is no longer `holds()`. The witness's whole purpose is to make this contradict something. To allow it, the witness would have to accept a declared rewrite — and a witness that accepts declarations is a witness of nothing. | **Rejected.** |
| **B. Redaction that keeps the link** (tombstone with inherited id) | replace the subject's PERSONAL FIELDS inside `order_json`, keep `id` and `prev` as they were, mark the record redacted; append a `Forgotten` event naming what was redacted | yes | yes, in place, same seals | no | no | Tip, count, every link unchanged; `chain_check` verifies a redacted record by its LINK (`next.prev == id`) and by a `Forgotten` declaration that names it, instead of by content. The witness sees nothing move — so the declaration must be counted where the census is. | One pure function on `Hub` and one on the archive path; a new `EventKind`; a new `ChainCheck` field; **no reader changes** — `roll` already skips an order without a phone (`roll.rs:74-76`), analytics never read `contact`, money fields are untouched. |
| **C. Crypto-shredding** (key per subject; the core's own `ErasureLedger`, `owner_surface.rs:818-830`) | encrypt `contact`/address under a per-`key` data key held in the object; erase = destroy the key | yes | yes | **yes — the ciphertext everywhere is dead** | yes for the log; **no for the key image** (PITR restores the deleted key for 30 days) | Chain untouched forever; witness untouched. | A cipher in the hub (none: `dowiz_hub::crypto` has HMAC, PBKDF2, hex — `crypto.rs:38-188`; `dowiz-core` has a from-scratch `pq/aes_gcm.rs` but the hub does not depend on core); a key image that must NEVER be in a dated backup or shredding is void; and **every reader of `contact` decrypts** — 13 files, `notify.rs` 6 sites, `storefront.rs` 7, `booking.rs` 9 (measured `grep -c contact`), two of them owned by other lanes today. It also does nothing for the ~every order already in the log in clear: history needs B regardless. |
| **D. Personal data outside the event store** (order events carry `cust:<key>` only; the `people` record holds the contact) | stop writing the phone into the log | going forward only | going forward only | going forward only | n/a | Chain untouched. | The kernel's `Order` carries `contact` by design (`domain.rs:94-97, 131`) and `place_order_at` takes `customer_id` (`json_api.rs:238-244`); every consumer of `/fold/order` — kitchen ticket, courier app (`public/courier/app.js:721, 994`), owner console (`public/admin/orders.js:183, 203`) — would need a join the Worker cannot do in one read until the P1 command surface holds both images. Same history problem as C. And the `people` record then holds the phone directory, so it must be backed up — and a dated backup of it is exactly what B cannot reach. |

**Decision: B now, with two rules that make it honest; C is the re-entry design, not the current one.**

1. **Redaction in place, id inherited, declared in the log.** A `Forgotten` event (kind 7, `is_order() = false`
   like `Revealed`, so folds and the socket skip it — `hubdo.rs:455-470` already refuses non-order kinds on
   the wire) carries `{ key, records: n, archives: [ids], by, at, reason }` and no contact details — the same
   rule `Revealed` follows (`lib.rs:73-80`). Redacted records keep their bytes' shape (`evlog.rs:86-92`) with a
   flag: the kind byte's high bit (`kind | 0x80`) so `from_u8(kind & 0x7f)` still classifies the event and
   `is_order` still holds. `chain_check` gains `redacted`, verified by link and by declaration, and the
   conservation audit's census gains the same count, so the number of tombstones and the number the
   `Forgotten` events declare must agree every night. A tombstone with no declaration is `broken`.
2. **What is redacted is the PERSON, not the ORDER.** `contact.phone` → `""`, `contact.name` → `""`,
   `fulfilment.address.line` → `""`, the customer note → `""`; `total`, `tip`, `status`, `items`,
   `created_at_ms`, the ledger legs stay. The fold then produces the same money and the same statuses, the
   customer fold produces one fewer person, and the conservation laws (`e2e/gates/conservation.mjs`) do not
   move. This is also why B costs no reader change: an order with an empty phone is a shape every reader
   already handles (`roll.rs:61-63`; the courier app guards `o.contact?.phone`).
3. **The dated S3 copies are "put beyond use", and the promise says so.** The ICO accepts data in backups
   that cannot be immediately overwritten as erased when the controller will not use it, gives no one access,
   secures it and deletes it when it becomes possible — and requires being "absolutely clear with
   individuals as to what will happen to their data … including in respect of backup systems"
   ([ICO via Osborne Clarke](https://marketinglaw.osborneclarke.com/data-and-privacy/new-ico-guidance-on-deleting-personal-data/),
   [Pinsent Masons](https://www.pinsentmasons.com/out-law/news/firms-do-not-always-need-to-delete-personal-data-to-comply-with-data-protection-rules-says-ico),
   [VeraSafe](https://verasafe.com/blog/do-i-need-to-erase-personal-data-from-backup-systems-under-the-gdpr/)).
   The bucket is the venue's (`cloud.rs:1-3`); a lifecycle expiry on `<prefix>/<venue>/` is the venue's
   configuration, and the privacy notice states the window. The PITR floor is 30 days whatever is chosen
   ([Cloudflare](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/)), so the window
   to state is **one month**, which is GDPR Art. 12(3)'s own deadline. A shorter promise is not available on
   this platform under ANY of A-D, and a design that claims one is lying.
4. **Images that hold a mutable personal record are backed up by OVERWRITE, never by date.** `people`
   (§3.1) and `consent` (§3.2) are not in `IMAGES` today (`hubstore.rs:1567-1568`) and must not join the
   dated bundle; they go to a fixed object name (`<prefix>/<venue>/people.json`) that tonight's copy replaces,
   so a deletion propagates within one night. This rule is what would make C viable later — a key image is
   exactly such an image — and it is cheap now (one `put` with a fixed key, `cloud.rs:381-385` shape).

**Re-entry condition for C (crypto-shredding):** the day the order log has a copy the venue cannot rewrite
within the month — replication to a peer node (MANIFESTO C4/§2, which is what the core's `ErasureLedger`
was written for), an object-locked or third-party archive, or a second venue's object holding a shared
ledger — B stops reaching every copy and per-subject keys become the only mechanism. At that point the
readers will also have moved into the object (P1 of `BLUEPRINT-ARCHITECTURE-EVOLUTION-2026-09-22.md`), which
is where decryption belongs. Build C then, on top of B, not instead of it: history in clear still needs B.

### 2.3 Identity resolution

- **Deterministic only.** Probabilistic merging (name similarity, shared address, "same device") produces a
  wrong merge that the victim discovers when a stranger's orders appear in their history or their wallet —
  and with money attached the mistake is a transfer. A restaurant has no fraud team and no appeals process.
  The gamedev record is the same lesson with more zeros: Epic Games shipped account merging in November
  2018 and closed it on 6 May 2019, permanently, after it produced disputes and abuse that could not be
  undone ([Epic account-merge notice](https://www.fortnite.com/news/account-linking-steps),
  [GamesRadar](https://www.gamesradar.com/fortnite-account-merge/)); what survived is LINKING — a platform id
  attached to one account, reversible — not merging. That is the shape to copy: **link, never merge.**
- **The safe merge key here.** `customer_key` is already a keyed, non-reversible handle (§1.1). Its one
  defect is that it is not normalised: two spellings of one Albanian number are two keys. The fix is not to
  change the function — `wallet:<key>` accounts and `cust:<key>` audit subjects would orphan — but to add
  an `alias` link: `people` kind `alias`, id = the non-canonical key, value = the canonical key, written only
  by (a) the deterministic rule "same E.164 number after normalising with the venue's country" or (b) an
  owner's explicit act that is audited exactly like a reveal. Every fold that today groups by `key` groups
  by `canonical(key)`; the rows stay; the view joins. A link can be removed; a merge cannot, which is the
  whole reason to prefer it.
- **Channels.** A WhatsApp `wa_id` IS an E.164 number, so `customer_key(wa_id)` links deterministically to
  the order-side key. A Telegram chat id is not a phone; it links only when the customer performs an act
  that carries their order token (a deep link from their own order page) or sends a contact card. Instagram
  sender ids never link by rule.
- **Money never merges.** Two `wallet:<key>` accounts under two linked keys stay two accounts; the balance
  shown is a fold over the link set; a refund posts to the account the order paid from. This is what keeps
  `wallet/mod.rs:18`'s "single-writer LWW is strictly correct" true — there is still one writer per account.

### 2.4 Loyalty — the evidence, the anti-patterns, and the answer to the tier question

**What the evidence says changes behaviour.**

- Endowed progress: Nunes & Drèze (2006) gave car-wash customers an 8-stamp card or a 10-stamp card with
  two stamps already on it; same effort, 19% vs 34% completion
  ([summary](https://www.coglode.com/nuggets/endowed-progress-effect)). Kivetz, Urminsky & Zheng (2006, JMR
  43:1): buy-10-get-1 coffee cards show purchase acceleration as the reward nears and a **post-reward reset**
  — engagement drops after the free coffee and climbs again toward the next
  ([Columbia](https://business.columbia.edu/insights/chazen-global-insights/goal-gradient-hypothesis-resurrected-purchase-acceleration),
  [paper](https://home.uchicago.edu/ourminsky/Goal-Gradient_Illusionary_Goal_Progress.pdf)). Both effects are
  about **visible progress toward a bounded, certain reward**. Neither needs points, tiers or expiry.
- Breakage: industry sources put retail programme breakage around 25% and treat it as revenue; a 2024 study
  of airline programmes found breakage inelastic to the usual levers and that promoting FREQUENT redemption
  is what lowers it ([Loyalty Magazine](https://www.loyaltymagazine.com/points-breakage-the-bane-of-loyalty-programs/),
  [ScienceDirect](https://www.sciencedirect.com/science/article/abs/pii/S0167811624000909)). Breakage is
  a stamp card's failure mode dressed as a feature: a scheme whose economics depend on people NOT collecting
  is a scheme designed against its members.

**From gamedev, and where the analogy breaks.** Progression systems are the mature form of loyalty: XP
curves are goal gradients, seasons are post-reward resets on a calendar, streaks are endowed progress with
loss aversion bolted on. The documented failure modes are the same three every time — grinding (the reward
scaled so the behaviour never ends), sunk-cost pressure (the progress bar is the reason to keep paying),
and loss-averse mechanics ("your streak ends tonight") aimed at the people least able to afford them. The
DSA's Art. 25 now prohibits interfaces that "materially distort or impair" a free decision
([EJRR](https://www.cambridge.org/core/journals/european-journal-of-risk-regulation/article/back-to-the-futureproof-four-reforms-for-the-better-regulation-of-dark-patterns-under-the-unfair-commercial-practices-directive-and-article-25-of-the-digital-services-act/B2C04326235DD360A456762AEB5BAB76));
the Norwegian Consumer Council's "Deceived by Design" is the regulator-side catalogue
([EDRi](https://edri.org/our-work/ncc-report-on-dark-patterns/)). **The analogy breaks at the door: a player
chose the game and its rules; a diner chose dinner.** A restaurant scheme that copies a mobile game's
retention mechanics — expiring points, streaks, a "you're almost Gold" nudge — without copying the game's
ethics (ratings, spend caps, parental controls, the option to quit with nothing lost) is a product
`MANIFESTO.md` refuses under C9's spirit and `CLAUDE.md:102`'s letter, and the counsel opinion's "no dark
patterns" finding (`counsel-opinion.md:25`) would flip to a STOP.

**Is a tier a score?** Under `CLAUDE.md:102` ("No rating/ranking/reputation of any participant") — **yes,
and the test is the direction of the gaze.** A number the person can see, that belongs to them, that they
spend and that changes nothing about how they are treated until they spend it, is a **balance** — the
wallet is one, a stamp count is one. A number the VENUE holds about a person, that orders persons against
each other and changes how each is treated (priority, price, tone, whether the courier hurries), is a
**rating** — whatever it is called and whatever it is computed from. Gold/Silver/Bronze is the second kind
by construction: its entire function is to make staff treat two persons differently on the strength of a
stored rank. The rule that deleted `reputation.rs` for couriers (`DECISIONS.md:350-351`) applies to diners
with no change of wording. Two corollaries for what exists: `Sort::Spent` (`roll.rs:24-32`) is a query the
owner runs, stored nowhere and acted on by nothing, and stays; a persisted "VIP" flag or a derived tier that
any code path branches on would be the first participant rating in the tree, and §5 G6 makes it fail the
build.

### 2.5 Campaigns: segment, schedule, send, measure — and where "did we already tell them" lives

- **Segment** = a pure filter over the customer fold ⋈ the consent fold ⋈ the record's tags, evaluated at
  SEND time, never stored as a list: a stored list is a recipient list the roadmap calls PII
  (`ROADMAP.md:1692`), it goes stale the moment someone withdraws, and its existence is the thing an
  erasure would have to find.
- **Schedule** = the minute cron that drains the outbox (`wrangler.toml:138`). No new entry point.
- **Send** = one `Entry` per recipient, in a `campaign` log, id = `<campaign>:<key>` so a retried command
  produces one message — the property the outbox already has for orders (`outbox.rs:65-68`) — with the
  rendered text at enqueue time (`:72-75`, the same reason: the catalogue must not change under a queued
  message). Each entry is minted ONLY through a function that takes a `Consented` value the consent fold
  alone can produce (§5 G1).
- **History** = the same `campaign` log: `about("sent", Some(key))` answers "what has this person been
  sent" in one prefix read (`logimage.rs:210`), and `about("sent", Some(campaign))` answers "who got this".
  Two kinds, one image, no table.
- **Measure** = a promo code per campaign, whose use is folded from the order log by `dowiz_hub::promo`
  (`promo.rs:17-21`). Redemptions are the only attribution this product needs and the only one that needs
  no tracking. Open/click tracking is recommended AGAINST (§4).
- **From gamedev, telemetry and consent:** a well-built game stores what the player did on the device and
  sends an opt-in subset; Apple's ATT made the default "no" in 2021 and opt-in settled around 25-35% with
  games slightly higher ([Business of Apps](https://www.businessofapps.com/data/att-opt-in-rates/),
  [Adjust via AppsFlyer](https://www.appsflyer.com/glossary/app-tracking-transparency/)). The lesson for a
  venue: **the storefront already keeps `dw_phone` on the diner's device (`checkout.js:344`); keep the
  preference there too**, and let the server hold only what a send needs — the key, the channel, the
  consent, the send history. Two thirds of people will say no to a well-worded box; a venue that designs
  for that number rather than against it does not need dark patterns.

---

## 3. The design

### 3.1 The customer record

**Where.** The venue's existing `people` image (`hubstore.rs:831-834`, 2 MiB), a `dowiz_hub::table::Table`
— "for sets bounded by the venue's own size: people, dishes, keys, memberships" (`table.rs:32-36`), which a
customer record is; the order HISTORY stays in the log and the record never duplicates it. Same object as
the log, so a placement that also touches the record is one turn once P1 lands, and today it is the second
`with_*` the one-image gate already counts (`tools/gates/one-image.sh:26-34`) — the write at
`storefront.rs:1175-1206` is ALREADY that second write, so the gate's baseline does not move.

**Key.** `customer_key` (§1.1), replacing `sha256_hex(raw)`. The current `cust` records are re-keyed by
a one-shot rebuild inside the object (`rebuild_index`'s shape, `table.rs:227`); nothing reads them, so the
rebuild has no consumer to break (measured, §0 row 1).

**What it holds — and only what a fold cannot.** Every field is something the venue would otherwise write
on a paper card by the till:

| Field | Held because | Not held |
|---|---|---|
| `note` (≤ 280 chars) | "always asks for extra ginger"; the paper card | free text about health, family, opinions — the note is shown with a one-line reminder that it is disclosable to the person |
| `tags` (from a venue-defined closed list) | "regular", "office lunch"; the closed list is what keeps tags from becoming a rating | any tag that ranks (`vip`, `difficult`) — refused at the vocabulary, §5 G6 |
| `allergens` (EU-14 codes, the vocabulary `allergens.rs:1-14` already refuses unknown codes for dishes) | the one field that prevents harm; matched against the dish declaration at placement | free-text allergies |
| `usual_table`, `lang` (`sq`/`en`/`uk`) | the console and any send speak the person's language | — |
| `birthday` as `MM-DD` | a greeting; the only marketing-adjacent field | the YEAR — age is not needed for a greeting and is the field that makes the record sensitive |
| `created_at_ms`, `updated_at_ms` | audit | — |

**Not in the record:** name, phone, address, order count, spend, last visit. The first three live in the
orders (and, for a send, are read through the consent fold, §3.2); the last three are folds
(`roll.rs:44-56, 64-114`). A record that repeats a fold acquires the counter-disagreement defect the wallet
and promo modules were written to avoid (§1.1).

**Who may write it:** an owner, from the console row that today shows the mask; every write is a
`Table::put` with the `updated_at_ms` from `ctx.data.now_ms`. **Who may read it:** the owner's console and
the placement path (allergen check). The courier never receives it — the order carries what a delivery
needs and nothing else, which the P0 opinion made a principle (`counsel-opinion.md:16`).

### 3.2 The consent model

**Where.** A `LogImage` named `consent` in the venue's object (append-only; `logimage.rs:1-7`); kinds `c`
(a consent act) and `w` (a wording, keyed by its hash, written once). Backed up by overwrite (§2.2 rule 4).
The fold — `consent::state(entries, key, purpose, channel) -> Option<Given>` — is pure, lives in
`services/customers/consent.rs`, and is the ONLY producer of the `Consented` witness type (§5 G1).

**Record shape:** §2.1's table, exactly. **Withdrawal** is a `c` entry with `state: withdrawn`; the fold
takes the newest per `(key, purpose, channel)`. Withdrawal channels: the same checkout box unticked on a
later order; the word STOP in a WhatsApp thread (`channels.rs`'s inbound path, one match); the owner on
request, with `evidence`. Withdrawal must be at least as easy as the grant (Art. 7(3)); a STOP that reaches
the inbox and is not folded within the minute is a defect, not a delay.

**Where the grant is collected.** One unticked box on the checkout under the phone field
(`checkout.js:165-168`), with the exact sentence in the diner's language: "Dubin & Sushi may send me
offers on WhatsApp to this number. I can stop at any time by replying STOP." The sentence's hash is the
`wording_id`; the three translations are three wordings. **Placing the order is not consent** (Recital
32); the box is separate, unticked, and its state travels in the order body as
`consent: { marketing_whatsapp: true, wording: <id> }`, written to the `consent` log in the same placement
turn. A checkout with no phone shows no box: there is nothing to consent to.

**What may not be sent without it — stated once:** *anything that is not (a) about an order this person
placed, sent to the contact they gave for it, while that order is live or being refunded; or (b) a reply
inside a conversation this person opened, within the channel's own window.* Everything else — an offer, a
"we miss you", a birthday greeting, a new-menu announcement, a loyalty count — is marketing and requires a
`given` consent for that purpose on that channel, at the moment of the send, not at the moment of the
campaign's creation.

### 3.3 The erasure design, and its cost

**Route:** `POST /api/owner/customers/:key/forget` with `{ reason }`, owner only, on the venue the caller is
authorised for (`Place::of_authorised`, the `one-venue` rule). The customer asks the venue — by any means —
and the venue acts; the audit is the venue's. A self-service route for the diner is not built (§4): the
diner's only credential is one order's token (`wallet.rs:354-356`), and a token that can erase a person is
a token whose theft erases a person.

**What happens, in one object turn once P1 exists, and today as a sequence with the log last:**

1. `people`: `remove("cust", key)` and every `alias` pointing at it (`table.rs:202`, "the record and every
   key that found it").
2. `consent`: nothing is removed. A `w`-less `c` entry `{ state: withdrawn, method: erasure }` is appended
   for every purpose/channel, so the proof of stopping outlives the person. The key is a pseudonym; with the
   `people` record and the log's contact gone, it identifies nobody.
3. `inbox`: threads whose subject is `whatsapp/<wa_id>` for a linked `wa_id` are removed by the
   `LogImage::keep` mechanism generalised to a predicate — a rebuild, ids change, and that is fine: the
   inbox is not witnessed (§0, last bullet), and its own `chain_check` verifies the new chain
   (`logimage.rs:232-243`).
4. `ledger`: **not touched.** Postings under `wallet:<key>` are money the venue owes or has taken; an
   accounting record is retained under Art. 17(3)(b)/(e), the account is pseudonymous already, and its
   memos are provider refs, not names (`wallet.rs:325`). A non-zero balance at erasure time is refunded
   FIRST if the customer asks and otherwise stays claimable through the venue; this document does not decide
   which and §7 lists it.
5. `log` and every archive in `log.archives`: the redaction of §2.2 — `Hub::forget(pred) -> Forgotten`
   in `dowiz-hub`, pure, tested natively: for every record whose payload's `contact.phone` hashes to `key`
   (or to a linked alias), rewrite the payload with the personal fields emptied and the kind's high bit set,
   keep `id` and `prev`; then `append(EventKind::Forgotten, "cust:<key>", declaration)`. Archives are loaded,
   rewritten with the same function, stored under the same name; their `(records, tip)` seal is unchanged, so
   the witness's `Seal` comparison passes without knowing.
6. `audit`: the `Revealed` events under `cust:<key>` stay — they record that an owner looked, and the
   subject is a pseudonym. This is the same call the reveal made: "a log of who read a phone number that also
   contains the phone number has doubled the exposure" (`lib.rs:79-80`); after erasure it contains neither.
7. The response names what was done: records redacted, archives touched, and the sentence the venue owes
   the person: "copies older than tonight in the venue's own backup expire within one month, and nothing
   reads them."

**Cost.** (measured where marked) A `Hub::forget` walk is the same O(n) as `grow`'s replay (`lib.rs:391-406`
comment: "a handful of milliseconds a few times in a hub's life"); an archive rewrite is one `load` + walk +
`to_bytes` per archive the person appears in — a customer of 14 months touches up to 14 archives
(hypothesis: one rotation per month per `rotate`'s header, `lib.rs:567-573`). The Worker side is one route
and one `hubstore` helper for archives. The `ChainCheck.redacted` field, the census field, and law 8
("tombstones = declarations") in `conservation.mjs` are each a few lines. No reader changes (§2.2, row B).
**The one real cost is the witness's blindness to a redaction, and it is paid by declaring in the log what
was redacted and counting both sides nightly.** An undeclared tombstone is `broken`; a declaration that
names more than exists is `broken`; the gate proves both (§5 G3).

### 3.4 Identity: link, never merge

`people` kind `alias`, id = non-canonical `customer_key`, value `{ canonical, by: rule|owner, at_ms,
reason }`. Rule-written aliases come from one pure function `identity::canonical_digits(phone, country)`
(E.164 normalisation for `+355`; `0xx…` → `355xx…`) applied at placement and at WhatsApp ingest — a
second key is written as an alias of the first at the moment a second spelling first appears, never by a
sweep. Owner-written aliases are audited with a `Revealed`-shaped event (`Linked`, kind 8) because linking
two people is a reveal of both. Every fold groups by `canonical(key)`; `roll` gains one argument, a
`resolve: impl Fn(&str) -> String`, passed in for the same reason `key_of` is. **Unlinking** is
`remove("alias", id)`; nothing else has to change because nothing was merged.

### 3.5 Loyalty — recommended AGAINST now; the one shape allowed later

**Against, today,** for three reasons that are about this tree and not about loyalty in general: (1) there
is no consent record, so a scheme that tells anyone anything is unlawful before it is useful; (2) there is
no channel to the customer except their own order page, so a count nobody can see changes nothing (§2.4:
the evidence is about VISIBLE progress); (3) no venue has asked, and a scheme built ahead of a request is a
feature the operator's roadmap doctrine ("roadmap first") did not order.

**Re-entry condition:** §3.1-3.2 built and G1-G4 green; a venue asks in writing; and the scheme is
**a stamp card and nothing else** — a fold `stamps(orders, key) = count of DELIVERED/COLLECTED orders mod N`
over the log (no stored counter; `promo.rs:17-21`'s law), redeemed as a `Fixed` promo the placement applies
when the count hits N, shown to the customer on their own order page ("3 of 10"), **no expiry, no tiers, no
points, no streaks, no "almost there" message** — the endowed-progress result is obtained by starting the
card at 1 (the order they just placed), which is true, not by a fake stamp. The tier question is answered
in §2.4: a tier is a rating of a participant and is refused by `CLAUDE.md:102`; §5 G6 refuses it in code.

### 3.6 Campaigns

Built after §3.1-3.4 and only then: a `campaign` LogImage (`kinds: def, sent`), a segment as a pure
predicate over `(Row, Record, ConsentState)` chosen from a closed list (`everyone_consented`,
`not_seen_since(days)`, `tag(x)`, `birthday_this_week`), a preview that shows the COUNT and the cost (SMS
would be paid per message, `ROADMAP.md:1692`; WhatsApp marketing templates are metered by Meta), the
`Consented` witness on every `Entry`, the outbox's own backoff and `Verdict::Abandon` reused unchanged
(`outbox.rs:44-60, 99-125`), attribution by promo code. Sends go out on the minute cron in batches of at most the
outbox's size budget; the health pane shows depth and age the way it does for orders (`outbox.rs:134-149`).

---

## 4. Recommended AGAINST, with the re-entry condition for each

| Item | Why not | Re-enter when |
|---|---|---|
| **Crypto-shredding now** (§2.2 C) | no cipher in the hub; ~40 `contact` read sites across 13 files, two of them another lane's this week; the key image would have to be exempt from dated backups (a rule this tree does not yet have); PITR keeps the deleted key 30 days anyway; history in clear needs B regardless | the log gains a copy the venue cannot rewrite within a month (replication, object lock, a third party); or readers have moved into the object (P1) |
| **Rebuild-without-the-subject** (§2.2 A) | changes the tip; the witness exists to contradict exactly that; a witness that accepts declared rewrites is decorative | never for the order log; it is the right tool for `inbox` and other unwitnessed logs, and §3.3 uses it there |
| **Probabilistic identity** (§2.3) | a wrong merge with money attached is a transfer; no appeals process; Epic's 2019 shutdown is the scar | never; the deterministic alias covers the real cases (spelling, `wa_id`) |
| **Merging rows** | irreversible; the alias link buys the same view and can be undone | never |
| **Points, tiers, expiry, streaks, "almost Gold"** (§2.4) | a tier is a rating of a participant (`CLAUDE.md:102`); expiry is breakage by design; streaks are loss aversion aimed at a diner who did not sign up for a game | tiers: never. Stamp card: §3.5's condition |
| **A persisted counter of anything derivable** | the counter always disagrees eventually (`promo.rs:17-21`, `wallet.rs:11-16`) | never |
| **Open/click tracking** | a pixel is a third-party read of a person's behaviour with no purpose the venue can name; promo-code redemption answers the only question that matters | a venue asks for a per-campaign number that redemption cannot give AND states the purpose |
| **Storing a birthday year, free-text health notes** | not needed for what they are used for; each makes the record special-category-adjacent | never for year; allergens use the EU-14 code list |
| **A diner-facing self-service erasure route** | the diner's only credential is an order token (`wallet.rs:354-356`); an erasure the token can perform is one its thief can | a durable customer credential exists (the device-bound keypair of `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md`) |
| **Email as a channel** | no email is collected anywhere on the venue side (`grep '"email"' storefront.rs` → none, measured); the platform's waitlist mail is a different principal | a venue collects email with its own consent box |
| **Owner-entered consent without evidence** | "how" is the field a dispute turns on; an unevidenced verbal consent is a claim | never; `evidence` is required by the type |
| **A second Durable Object class or a new image family** | `platform_store.rs:10-16` already argued this once; `people` exists and is sized | never |
| **Building any of §3.6 before §3.1-3.4** | a campaign system without a consent fold is a spam system with a queue | in order, §6 |

---

## 5. The gates

All in `tools/gates/`, `sh` + Python where braces must be matched (the `one-venue.sh`/`one-image.sh`
shape), with a `.baseline` that may only fall (`clock.sh`'s mechanism), wired into `.github/workflows/ci.yml`
beside the ten there (`ci.yml:40-86`), and each with a `*.prove.*` that runs the gate against a stub and
asserts it goes RED on the defect and GREEN on the fix — the shape `e2e/gates/conservation.prove.mjs:1-16`
already established ("a gate is triggered before it is trusted").

**G1 — an un-consented send is unrepresentable, not discouraged.** Type-level: `campaign::Entry::new`
takes a `Consented` value; `Consented` has a private constructor and is produced only by
`consent::state(...)` returning `Some`; the compiler is the gate. Grep-level (`consent.sh`): every call of
`channels::whatsapp_text`, `channels::instagram_dm`, `notify::telegram` whose recipient argument is not the
venue's own `notify.*.to`/`chat` settings key must sit in a brace-matched body that holds a `Consented`
binding or a `Reply` (thread) binding; count the bodies that do not; baseline = 0 on the day it lands.
**Prove:** a native test builds a segment of three keys, withdraws one, runs the campaign → exactly two
`sent` entries and the withdrawn key absent; and `consent.prove.sh` inserts a bare `whatsapp_text(…, "+355…")`
into a scratch copy and asserts exit 1. A `cargo mutants` run over `consent::state` (the CI already runs
mutants on kernel files, `ci.yml:94-95`) must kill the mutant that returns `Some` unconditionally.

**G2 — the record holds nothing a fold already knows.** `record.sh`: the `cust` record's JSON keys are
compared against a committed allow-list (`note, tags, allergens, usual_table, lang, birthday_md,
created_at_ms, updated_at_ms`); any of `orders, spent, last_at, name, phone, balance, stamps, tier, score`
appearing as a key in a `put("cust"` body is a hit; baseline 0. **Prove:** the scratch-copy method.

**G3 — tombstones equal declarations, nightly.** Law 8 in `conservation.mjs`: `chain_check.redacted`
across hot + archives == the sum of `records` in `Forgotten` declarations; `broken` stays 0. **Prove:** in
`conservation.prove.mjs`, a stub with one tombstone and no declaration → red; one declaration and no
tombstone → red; equal → green. Native: `Hub::forget` on a 40-record image with 6 of the subject's → 6
redacted, `chain_check.chained + redacted == records`, `tip()` unchanged, `holds(old_tip)` true, the
customer fold has one fewer row, the analytics fold is byte-identical.

**G4 — every send has a history row and a withdrawal stops the next one within a minute.** e2e on the
stubbed platform: enqueue a campaign of two, drain, assert `about("sent", key)` = 1 per key; withdraw one
via a STOP in the inbox; enqueue again; assert one. **Prove:** remove the fold call in the drain in a scratch
copy → the second assertion fails.

**G5 — personal-record images are backed up by overwrite.** `backup-shape.sh`: `IMAGES` may not contain
`people` or `consent`; `cloud::push_place` must name a fixed object for them; grep for `"people"` inside the
`IMAGES` array is the hit. **Prove:** scratch-copy insertion → exit 1.

**G6 — the participant-rating job the comment promised.** `no-scoring.sh`: comments stripped, the
identifiers `courier_score, rating, reputation, tier, vip, score` as struct fields, record keys or enum
variants in `crates/dowiz-core/src`, `kernel/src`, `crates/dowiz-hub/src` and `workers/api/src` are hits;
baseline = today's count (hypothesis: small — `intake.rs:6-7, 551` and `fdr/pmu.rs:9` use `Tier` for
hardware/admission classes and will need an allow-list line each). The job named in `CLAUDE.md:103` and
`domain.rs:137` finally exists, and it covers the diner. **Prove:** scratch-copy insertion of
`pub tier: u8` into `Row` → exit 1.

---

## 6. Order of work, each with its CHECK

1. **Re-key and read the record that exists.** `people`'s `cust` id → `customer_key`; the console row
   gains `note/tags/allergens/lang/usual_table` (§3.1); the placement path matches `allergens` against the
   dish declaration and refuses with a named reason, the way `allergens.rs` refuses an undeclared dish.
   **CHECK:** G2 green at 0; a native test that the old `sha256` id is gone from every record after the
   rebuild; the customers route unchanged in output (its 6 tests, plus one that a record's note appears on
   the row).
2. **The consent log and the checkout box** (§3.2), three wordings, the fold, the `Consented` type.
   **CHECK:** G1's type exists and the compiler refuses an `Entry` without it; the placement writes one `c`
   entry when the box is ticked and none when it is not; a Playwright run ticks the box, places, and reads
   `about("c", key)` = 1 with the right `wording_id` for the language in use.
3. **`Hub::forget`, `Forgotten`, `ChainCheck.redacted`, the route, the archive path, law 8** (§3.3).
   **CHECK:** G3's native and prove tests; a live probe on a TEST venue: place three orders under one phone,
   reveal, forget → the list has no row, `/api/owner/customers/reveals` still shows the reveal under the
   pseudonym, `/api/owner/health` shows `redacted: 3, declared: 3`, the witness's next census passes.
4. **Backup by overwrite for `people` and `consent`** (§2.2 rule 4). **CHECK:** G5; two consecutive
   nightly pushes leave ONE `people.json` in the bucket (read the listing back, the way
   `cf-deploy-token-is-a-different-file` taught: verify by reading, not by the script's exit code).
5. **Aliases** (§3.4): the normaliser, the two rule sites, the owner link with its `Linked` audit event,
   `roll` grouping by canonical. **CHECK:** the tests in `services/customers/tests.rs` gain "two spellings
   of one number are one row" with `+355 69…` and `069…`; the wallet balance of a linked pair is the sum,
   the accounts are still two.
6. **Campaigns** (§3.6). **CHECK:** G1 and G4 green; a segment preview on the live venue reports a count
   and a cost before anything is queued; one real campaign to the operator's own number, with STOP, end to
   end.
7. **The stamp card — only on §3.5's condition.** **CHECK:** the count is a fold (G2 red if stored); it
   is visible on the order page; no message about it is sent without a `Consented`.
8. **Documents.** Fix `CLAUDE.md:102-106` to say what enforces the rule until G6 lands; add the
   customer-facing "what we keep and for how long" line (and the courier's, `counsel-opinion.md:93`) to the
   kit's privacy screen (`public/kit/app.js:32` routes one; its content was not read here).

**Order rationale.** 1 and 2 are the two records without which nothing else is lawful or useful; 3 is
the hardest and the one whose cost is bounded (no reader changes); 4 is a one-line rule that must exist
before any personal-record image is worth backing up; 5 closes the four-handles defect without touching
money; 6 is the feature; 7 is deliberately last and conditional.

---

## 7. What could not be determined, and the command or reading that would settle it

| Question | Status | What settles it |
|---|---|---|
| Whether Albanian law carries an ePrivacy-style soft opt-in for existing customers | not confirmed; the design does not rely on it | counsel reads Law 124/2024's direct-marketing article and Law 120/2023 on electronic communications; until then, consent for everything |
| Whether the venues' buckets have a lifecycle rule, and of what length | not readable from this tree (`cloud.rs` writes; it never lists or configures) | `aws s3api get-bucket-lifecycle-configuration --bucket …` with the venue's credentials, and the privacy notice states the answer |
| How many orders in the live log carry a phone, and how many phones are spelled two ways | not measured (no live read was made) | `/api/owner/customers` on each venue, and a one-off count of `customer_key(normalised)` collisions inside the object |
| Whether a `Forgotten` rewrite of an archive races the nightly `archive_seal` read | not checked; both run under the object's single writer, but the archive is loaded by the Worker (`hubstore.rs:1202-1222`) | run step 3's live probe at 03:17 on a test venue |
| What the ledger should do with a non-zero balance at erasure | decided AGAINST touching postings; the customer-facing outcome is open | operator decision: refund-first or claimable-through-the-venue |
| Whether Meta's marketing template category is required for a consented WhatsApp send, and its per-message price in Albania | not read | Meta's pricing page for the venue's WABA; §3.6's preview must show it |
| The `Tier` uses in `intake.rs` and `fdr/pmu.rs` and whether G6's allow-list can be two lines | hypothesis | `grep -n 'Tier' crates/dowiz-core/src/intake.rs crates/dowiz-core/src/fdr/pmu.rs` when G6 is written |
| `booking.rs`'s `contact_phone` — whether the booking lane keys or normalises it | working tree, another lane, not read beyond `:81-82, 209-210` | its blueprint; the alias rule (§3.4) must be applied at its write site too |
| Whether the kit's privacy screen already promises anything about retention | `public/kit/screens/privacy.js` not read | read it before step 8 |
| Line numbers in `crates/dowiz-hub/src/lib.rs` after the booking lane lands | cited from HEAD `7b0c9871`; +1 expected | re-cite after merge |

---

## Sources outside the tree

- ICO, *Consent* (records: "who, when, how, and what you told people"): https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/lawful-basis/a-guide-to-lawful-basis/consent/
- ICO on backups "put beyond use": https://marketinglaw.osborneclarke.com/data-and-privacy/new-ico-guidance-on-deleting-personal-data/ ; https://www.pinsentmasons.com/out-law/news/firms-do-not-always-need-to-delete-personal-data-to-comply-with-data-protection-rules-says-ico ; https://verasafe.com/blog/do-i-need-to-erase-personal-data-from-backup-systems-under-the-gdpr/
- Albania Law 124/2024: https://idp.al/wp-content/uploads/2025/04/Law-no.124-2024-DP.pdf ; https://www.karanovicpartners.com/news/albania-adopts-new-direct-marketing-rules-aligned-with-gdpr/ ; https://kpmg.com/al/en/insights/2025/02/new-law-on--personal-data-protection-.html
- Meta, WhatsApp opt-in: https://developers.facebook.com/documentation/business-messaging/whatsapp/getting-opt-in ; https://www.infobip.com/docs/whatsapp/compliance/user-opt-ins
- Cloudflare, SQLite-backed Durable Object storage and 30-day point-in-time recovery: https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/
- Nunes & Drèze (2006), endowed progress: https://www.coglode.com/nuggets/endowed-progress-effect
- Kivetz, Urminsky & Zheng (2006), *JMR* 43(1): https://home.uchicago.edu/ourminsky/Goal-Gradient_Illusionary_Goal_Progress.pdf ; https://business.columbia.edu/insights/chazen-global-insights/goal-gradient-hypothesis-resurrected-purchase-acceleration
- Breakage: https://www.loyaltymagazine.com/points-breakage-the-bane-of-loyalty-programs/ ; https://www.sciencedirect.com/science/article/abs/pii/S0167811624000909
- Epic Games account merge, ended 6 May 2019: https://www.fortnite.com/news/account-linking-steps ; https://www.gamesradar.com/fortnite-account-merge/
- DSA Art. 25 and dark patterns: https://www.cambridge.org/core/journals/european-journal-of-risk-regulation/article/back-to-the-futureproof-four-reforms-for-the-better-regulation-of-dark-patterns-under-the-unfair-commercial-practices-directive-and-article-25-of-the-digital-services-act/B2C04326235DD360A456762AEB5BAB76 ; https://edri.org/our-work/ncc-report-on-dark-patterns/
- ATT opt-in rates: https://www.businessofapps.com/data/att-opt-in-rates/ ; https://www.appsflyer.com/glossary/app-tracking-transparency/

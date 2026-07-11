# 03 — The Anonymity Architecture (honest, per-layer × per-actor)

> **Design report, 2026-07-11.** The operator has ratified **anonymity as a stated value** for dowiz's
> local-first delivery hub (beachhead: Durrës, Albania; per-vendor sovereign node + dumb relay per
> `02-local-first-architecture.md`; COD-mandatory; NO courier scoring). This report designs the
> **maximal, HONEST anonymity architecture** a real cash-on-delivery food-delivery hub can actually
> deliver — decomposed by threat-model layer, told with its floors and costs stated plainly. Zero
> coding; the only file created is this one; both repos left exactly as found.
>
> **It re-opens and partly overrules the relay report** (`docs/research/2026-07-11-relay-hetzner-tailscale-mesh.md`
> §2c), which rejected i2p on the premise "anonymity is an anti-requirement in a fiscalized
> attribution-based COD system." That premise is now **overruled by the operator and was partly wrong on
> the facts** — see §0.
>
> **Evidence labels:** **VERIFIED** (a research lane fetched the primary source this session — see
> Sources), **VERIFIED-secondary** (reputable secondary), **UNVERIFIED** (assessment/snippet, flagged),
> **DESIGN-JUDGMENT** (my synthesis, falsifiable by building it). Web research (Tor/i2p/Nym/mixnet state,
> anonymous-credential/OHTTP state, Albanian fiscal/SIM/data-retention law, privacy-preserving-delivery
> precedents) run fresh 2026-07-11 across four parallel lanes.

---

## 0. The correction of framing (the spine of this report)

**Anonymity is not one thing, and "anonymous" is meaningless without "from whom."** The relay report
collapsed both dimensions into a single "anti-requirement" verdict. Two errors followed:

1. **It treated anonymity as monolithic.** It is at least **five distinct layers**, each with its own
   physics and cost:
   - **(a) data/platform anonymity** — no central profile of you exists anywhere. *Local-first already
     delivers this for free* — there is no dowiz database to profile you in.
   - **(b) content confidentiality** — nobody in transit reads the order. *E2EE/TLS-on-the-node already
     delivers this.*
   - **(c) network/metadata anonymity** — your IP, your location, who-talks-to-whom, when. *This is the
     hard, expensive layer — Tor/i2p/Nym/mixnet territory.*
   - **(d) identity/credential anonymity** — you transact under an unlinkable pseudonym, not a name.
     *Pseudonymous keys + unlinkable orders + blind-signature/anonymous-credential territory.*
   - **(e) financial anonymity** — no card trail, no bank identity. *COD cash already delivers this — and
     Albanian law confirms it (below).*

2. **It mis-stated the fiscal constraint, and the mistake inflated the "attribution" argument.** The
   relay report implied fiscalization makes "the money trail legally attributable by design," treating
   that as a reason anonymity is pointless. **This is wrong for the buyer.** Albanian fiscalization
   (Law 87/2019) binds the **vendor's recorded sale**, not the buyer's identity: for an ordinary B2C cash
   food order, the mandatory fiscal-receipt fields are **all seller-side** (NIVF, NSLF, seller NIPT, POS
   ID, items, VAT breakdown, timestamp) — **no buyer field is in the mandatory set.** Buyer NIPT is
   required only for B2B, legal-entity buyers, personal-property sales >500,000 lek, or when the buyer
   affirmatively asks for a named invoice (VERIFIED — sherbimekontabiliteti.al, dddinvoices.com,
   fiscal-requirements.com). So a **cash B2C sale is buyer-anonymous by default and by law.** The
   operator's premise is correct; the relay report's was not. Fiscalization is a constraint on **the
   vendor's transparency to the state**, which the design *wants* — it is nearly *orthogonal* to the
   buyer's anonymity, not opposed to it.

**What the relay report got right survives, for the right reasons.** Its *conclusion* — i2p is unfit —
still holds, but on **performance and tooling** grounds (§3), not because "attribution forbids anonymity."
And Tor, which it lumped into the same rejection, turns out to be the **re-opened winner** for the one
leg where anonymizing transport actually fits (§3). The attribution the system needs (signed PoD,
counter-signed custody, fiscal sale) is **pseudonymous and vendor-scoped** — it coexists cleanly with
buyer/courier anonymity, because `pod.rs` already binds a courier's *vault-id*, not their name (VERIFIED
— `crates/bebop/src/pod.rs:1-96`).

**The two hard floors, stated up front (everything else is designed around these):**

- **Floor 1 — the delivery address must reach whoever delivers.** A customer ordering *delivery* cannot
  be address-anonymous from the courier; a physical object must arrive at a physical place. This is
  physics, not a design gap. (Mitigable only by removing the courier from the address: pickup / locker /
  "meet at corner" — §2.5.)
- **Floor 2 — the buyer is financially anonymous by default (VERIFIED above); the vendor's sale is
  intentionally visible to the state.** These are different subjects. We hide the buyer; we do not (and
  legally cannot) hide the vendor's sale — and we do not want to.

Two further floors surfaced by this session's legal research, which bound **anonymity-from-the-state**
independently of anything we build:

- **Floor 3 — Albanian prepaid SIM registration is mandatory** (passport/ID at point of sale —
  VERIFIED-secondary: howalbania.com, phonetravelwiz.com, wise.com). Any phone number used for order
  coordination is therefore traceable to a registered identity **at the carrier**. Phone-based
  pseudonymity protects the buyer from the *vendor/platform/courier*, **not from the state via the
  carrier.**
- **Floor 4 — telecom traffic-metadata retention.** Under Albania's electronic-communications regime,
  operators retain subscriber + traffic metadata for **1–2 years** (VERIFIED-secondary, old Law
  9918/2008 framework; whether the new Law 54/2024 preserves the exact window is UNVERIFIED). So a
  network observer with legal process can reconstruct who-connected-to-whom **unless the customer's leg
  is carried over anonymizing transport** (Tor) — which the one-shot web customer usually cannot use
  (§3). This is the single most important honest limit in the whole report.

---

## 1. The layer × actor anonymity matrix (the corrected framing)

Rows = the five anonymity layers. Columns = **anonymous FROM WHOM.** Cell verdicts:
**✅F** achievable free / already-on (≈zero marginal cost);
**💰C** achievable with cost (opt-in, latency, UX friction, or code/ops burden — see §5);
**⛔I** physically or legally impossible (a hard floor);
**·** not applicable / trivially moot for that actor.

| Layer ↓  \  Anonymous FROM → | **Platform (dowiz)** | **Vendor** | **Courier** | **Network/ISP observer** | **The State** | **Other users** |
|---|---|---|---|---|---|---|
| **(a) Data / no central profile** | ✅F ⁽¹⁾ | 💰C ⁽²⁾ | ✅F | · | 💰C ⁽³⁾ | ✅F ⁽⁴⁾ |
| **(b) Content confidentiality** | ✅F ⁽¹⁾ | ⛔I ⁽⁵⁾ | 💰C ⁽⁶⁾ | ✅F | ⛔I ⁽⁷⁾ | ✅F |
| **(c) Network / metadata (IP, location, who↔who)** | 💰C ⁽⁸⁾ | 💰C / ⛔I ⁽⁹⁾ | ⛔I ⁽¹⁰⁾ | 💰C ⁽¹¹⁾ | 💰C / ⛔I ⁽¹²⁾ | ✅F |
| **(d) Identity / credential (pseudonym, unlinkable)** | ✅F ⁽¹³⁾ | 💰C ⁽¹⁴⁾ | 💰C ⁽¹⁴⁾ | ✅F | ⛔I ⁽¹⁵⁾ | ✅F |
| **(e) Financial (no card/bank trail)** | ✅F ⁽¹⁶⁾ | ✅F ⁽¹⁶⁾ | ✅F ⁽¹⁶⁾ | ✅F | ✅F (buyer) ⁽¹⁷⁾ | ✅F |

**Cell notes (the load-bearing ones):**

1. **Platform learns nothing by construction (DESIGN-JUDGMENT on the ratified topology).** In
   local-first, dowiz-the-company operates *no* order database — only a **dumb SNI-passthrough relay**
   that forwards ciphertext (TLS terminates on the vendor node; relay sees SNI + IPs + timing only —
   VERIFIED-in-repo `02-local-first-architecture.md §4.4`). So (a) data, (b) content, (d) identity, (e)
   money are all ✅F *from the platform*. The one exception is (c): the relay operator (= the platform)
   does see customer-IP↔vendor + timing — hence 💰C at note 8, not ✅F.
2. **From the vendor, per-order pseudonymity is achievable but repeat-order linkage is the cost.** The
   vendor is the order authority and must hold the order — but need not learn a durable identity. Today
   the schema links a customer by `customers(location_id, phone)` UNIQUE + `no_show_count`
   (VERIFIED — `packages/db/migrations/1780310074262_orders.ts:7-15`). Unlinkable-by-default requires
   dropping/optional-izing that key (§2.2) — hence 💰C, and it trades against the no-show memory (§2.4).
3. **From the state, no *central* profile exists to seize (✅F-ish), but each vendor node is individually
   compellable** (note 7/15). Net 💰C: the *aggregate* profile is gone; a *single* order at a single
   vendor is reachable by lawful process.
4. **Other users is the easy column — ✅F across the board.** No shared database, no social graph, no
   cross-tenant scoring, per-order pseudonyms ⇒ no customer/courier/vendor learns anything about any
   other user. This is a *direct dividend of local-first + the NO-courier-scoring red line.*
5. **Content from the vendor is impossible — the cook must read the order.** You cannot hide *what to
   make and where to send it* from the party making and sending it. ⛔I by definition.
6. **Content from the courier is minimizable, not zero.** The courier needs the address + the items to
   hand over; they do **not** need the customer's name, phone, payment history, or full basket context.
   Precedent: Uber Eats "View as Delivery Person" (Jan 2023) shows couriers only name/last-initial +
   address en route, never phone/payment/photo (VERIFIED — restaurantdive.com). 💰C = the code to scope
   the courier's view.
7. **Content from the state = ⛔I via the vendor node.** E2EE protects the wire, but the vendor node
   holds order plaintext; a lawful order to the vendor produces it. Anonymity-from-the-state is bounded
   by the vendor being a compellable, identifiable, fiscally-registered business — which it is, by law.
8. **Metadata from the platform-qua-relay = 💰C.** The SNI-passthrough relay sees customer-IP↔vendor +
   timing. Removing that requires the customer to reach the vendor node over a path the relay isn't on —
   i.e. the **Tor .onion leg (§3.2), which needs no relay at all** and hides the IP. Opt-in ⇒ 💰C.
9. **Metadata from the vendor: IP is hideable (💰C via Tor), the delivery address is not (⛔I) — Floor
   1.** Pickup/locker orders remove the address and flip this to 💰C (§2.5).
10. **Metadata from the courier = ⛔I for delivery — Floor 1.** The delivering courier learns the drop
    location. Irreducible for delivery; absent for pickup.
11. **Metadata from a network/ISP observer = 💰C.** TLS hides content, but the observer sees the customer
    connecting to the vendor's relay IP + SNI + timing. Only anonymizing transport (Tor/mixnet) breaks
    the who↔who linkage — and the default one-shot web customer usually can't run it (§3.5). SNI is
    visible unless ECH; treat SNI-leak as real.
12. **Metadata from the state = 💰C with Tor, ⛔I without — Floors 3+4.** Carrier SIM-registration +
    1–2yr traffic retention let the state reconstruct the customer's connection via the ISP and tie the
    coordinating phone to an identity. Tor on the customer leg defeats the ISP-correlation half; it does
    nothing about a phone number registered to the customer (note 15).
13. **Identity from the platform = ✅F.** No accounts. The existing pre-auth **opaque track-token**
    (`?t=` → short-lived tracking capability, registered in `NO_AUTH_PATHS`) is already an
    account-less, per-order pseudonymous handle (VERIFIED — `apps/api/src/routes/customer/track.ts:7`).
14. **Identity from the vendor/courier = 💰C.** Pseudonym achievable (per-order key; name optional). The
    cost: a *contact channel* is needed to coordinate (call "I'm outside"), and today that's a phone
    number. Proxy-number masking (DoorDash/Uber pattern — VERIFIED developer.doordash.com) or an in-app
    relayed channel keeps the real number off the vendor/courier record.
15. **Identity from the state = ⛔I if a registered phone is used (Floor 3).** Prepaid-SIM registration
    ties any coordinating number to an ID at the carrier. True pseudonymity-from-the-state would require
    not using a self-registered phone at all — outside a lawful food business's realistic UX. State this
    plainly; do not pretend otherwise.
16. **Financial from everyone = ✅F — the COD superpower.** Cash settles physically; no card, bank,
    gateway, or PCI surface exists anywhere in the stack (VERIFIED-in-repo `02 §3.1`). No money moves
    through the platform, so there is nothing to trace.
17. **Financial-from-the-state, buyer = ✅F; the vendor's sale is deliberately visible (Floor 2).** The
    fiscal receipt records the *sale*, not the *buyer* (VERIFIED §0). The state learns "vendor sold €X at
    12:04," not who ate it.

**Reading the matrix:** the two right-to-hardest columns are **the vendor** (who must cook + route) and
**the state** (who can compel the vendor and the carrier). Everything *else* — anonymity from the
platform, from other users, from a passive network observer, and financial anonymity from all — is
**already free or nearly so** under local-first + COD. The *design work* concentrates in exactly three
cells: **(c)/vendor+state IP-metadata** (→ Tor, §3), **(d)/vendor+courier identity** (→ pseudonyms +
credentials, §2), and **(b)/courier content** (→ view-scoping, §2.5). This is a much narrower — and much
more achievable — problem than "make delivery anonymous."

---

## 2. Data + identity anonymity design (always-on, ≈free — ship as the pilot default)

This is the layer where dowiz can honestly claim **more anonymity than any European delivery incumbent**
at essentially zero latency/battery cost. (Market gap, VERIFIED: Bolt retains courier-visible client
data ≤1 month, Delivery Hero anonymizes after 7 days, Wolt "as necessary" — all as *GDPR compliance*,
none as a *consumer-facing feature*; only Proton markets "we can't see your data" as a differentiator —
proton.me/blog/switzerland.)

### 2.1 No central profile, no account — already true, make it a promise

Local-first already means there is no aggregate profile. Keep the account-less flow: QR → menu → cart →
signed intent → opaque track-token (VERIFIED existing). No email, no signup, no password. The customer's
*only* durable handle is whatever contact they give for coordination — minimize that to a proxy/ephemeral
channel (§2.5).

### 2.2 Unlinkable orders by default; opt-in linking only (DESIGN-JUDGMENT)

- **Default:** each order carries a **fresh per-order pseudonym** — an ephemeral keypair the customer
  page generates client-side, used to sign the intent and to authenticate the track-token. Two orders by
  the same human do **not** link at the vendor unless the human chooses to link them.
- **Derivation:** the per-order key can be a fresh random key (simplest, strongest unlinkability) or a
  **blinded/HD-derived** child of a device seed (Tor-v3 key-blinding or BIP32-hardened). Honest limit
  (VERIFIED): HD/blinded derivation gives unlinkability against an **outside observer** who sees one
  child key at a time, but **not against the issuing/verifying party** who could hold the derivation
  path — so for *vendor-side* unlinkability, fresh-random-per-order is simpler and strictly safer than
  clever derivation. Use fresh random keys; reserve key-blinding for the *device↔its-own-orders* link
  only.
- **Opt-in linking = loyalty/history.** A customer who *wants* order history or a "usual order" opts into
  a **stable pseudonym** (a reused key or a KVAC credential, §2.3). Linking becomes a customer-granted
  capability, never a default the platform imposes. This is the anonymity-preserving analogue of an
  account.

### 2.3 Anonymous credentials — "a valid customer, without who" (feasibility assessment)

The question: can dowiz prove "this is a real, rate-limited customer, not a bot/flooder" **without
learning who**? 2026 state (VERIFIED):

- **Privacy Pass is standardized** (RFC 9576/9577/9578, June 2024) and shipping at scale (Apple Private
  Access Tokens; Cloudflare/Fastly issuers). Rust `privacypass` crate exists.
- **ARC — Anonymous Rate-Limited Credentials** (draft-ietf-privacypass-arc, Apple+Cloudflare, WG-adopted
  Sept 2025) is *exactly* the "one issued credential → N unlinkable, context-bound, capped presentations"
  primitive — "valid customer, capped uses, unlinkable" in one object.
- **KVAC** (keyed-verification anonymous credentials — the scheme Signal's private group system uses,
  eprint 2019/1416) is the **pragmatic pick when issuer == verifier**, which is our case: the *vendor
  node* both issues and checks. KVAC is cheaper than public-key BBS and runs sub-second on a phone.

**Assessment (DESIGN-JUDGMENT):** feasible, but **not pilot-critical**, and honest about a ceiling. A
small self-hosted operator collapses the Privacy-Pass roles (Issuer + Attester + Origin are all the
vendor node), which weakens the *cross-party* unlinkability guarantee that Apple/Cloudflare get from
role-separation — but it is still **strictly better than IP/cookie rate-limiting**. The real payoff is
**future**: anonymous credentials are what let dowiz **drop the phone-number requirement** (Floor 3's
weak point) and still resist Sybil/flood abuse. Until then, **COD itself is the anti-abuse mechanism**
(§2.4). Ship credentials as a fast-follow/opt-in, not a pilot blocker. Any signature here inherits the
**G09 hybrid-only rule**: PQ⊕Ed25519 with the audited classical half load-bearing (VERIFIED-in-repo
`C-runtime-transport-identity.md §3.5`).

### 2.4 Reconciling anonymity with the NO-courier-scoring red line and anti-abuse

There is a real, honest tension the design must own: **unlinkable orders defeat per-customer abuse
memory** (the current `no_show_count` / `customer_signals`, VERIFIED — `apps/api/src/lib/signals/
compute.ts:86`). You cannot both "not remember who you are" and "remember that you no-showed twice."

- **The resolution is that COD structurally removes most of the need.** With cash-on-delivery there is
  **no prepayment to steal and no card to charge back** — the dominant delivery-fraud vectors don't
  exist. The residual risk is *no-show / fake orders wasting food*, and the honest mitigations that
  **don't** require a durable customer identity are: (i) **anonymous-credential rate-limiting** (§2.3 —
  cap orders-per-credential-per-window without identity); (ii) **cash skin-in-the-game** — the customer
  must physically be present to receive and pay; (iii) **per-vendor, phone-scoped** no-show memory for
  customers who *opt into* a stable pseudonym, kept strictly local to that vendor and never shared
  cross-tenant.
- **This stays inside the red line.** NO-courier-scoring is untouched — this is customer-side abuse
  control, and even that is designed to be *forgettable by default*. (Note: `reputation.rs`/`KillSwitch`
  in bebop is courier-side and remains governed by the standing collision flagged in SYNTHESIS §7 — out
  of scope here.)

### 2.5 Minimal retention + crypto-shredding — the highest-leverage, most honest win (DESIGN-JUDGMENT)

**The critical architectural move:** PII must **not** live in cleartext inside the hash-chained,
append-only, *replicated* signed event log — because you cannot later erase it from an immutable,
multi-device log (the data-sync lens already flags this: "GDPR erasure across devices you don't control
is an open problem" — VERIFIED-in-repo `B-data-sync.md:204,321`). So split the record:

- **The signed skeleton (immutable, replicated, PII-free):** order-id, item-ids + amounts (`Lek(i64)`),
  the 10-status transitions, the per-order pseudonym pubkey, the PoD proof (`pod.rs`), the NIVF fiscal
  reference. All non-PII. This is what gets signed, sequenced, gossiped, and kept for fiscal/audit.
- **The PII envelope (encrypted, erasable, minimally-replicated):** delivery address, coordination
  contact, optional name — stored **encrypted under a per-order data key**; only a **hash commitment** to
  it enters the signed log. The courier/vendor decrypt it on a need-to-act basis; it lives on the vendor
  node (and the courier's device for the duration of the run), not gossiped to the mesh.
- **Crypto-shred on expiry:** after the dispute/return window (e.g. 30 days), **destroy the per-order
  data key.** The address/contact ciphertext rots into noise everywhere it replicated; the signed
  skeleton (money, status, PoD, NIVF) survives intact and PII-free. This is the local-first-honest form
  of GDPR erasure, and 2026 regulator guidance now **endorses key-destruction as erasure**: EDPB
  Guidelines 02/2025 (blockchain, adopted Apr 2025) treats destroying keys / erasing off-chain
  components as rendering data "practically unidentifiable" where literal deletion is infeasible
  (VERIFIED — edpb.europa.eu). Product precedent: Signal/WhatsApp disappearing messages (VERIFIED).

**What bebop2 already gives this layer (VERIFIED from code):**
- `pod.rs` — **pseudonymous proof-of-delivery today**: `claim = order|courier-vault-id|ts|loc`, signed
  hybrid; the courier is a *self-cert vault id, not a name*; verifier learns "this id delivered this
  order at this place/time" with **no PII and no directory**, and replay-at-wrong-location is rejected
  (VERIFIED — `pod.rs:73-96,153-165`). This is *already* "prove a valid delivery without revealing legal
  identity" — the attribution the system needs, in pseudonymous form. It is **not** a zero-knowledge
  proof; it reveals *which* pseudonym delivered.
- `zkvm.rs` — an **honest hash-commitment seam**, explicitly *not* a real ZK system (`Proof::Stark`
  fails closed without an injected verifier — VERIFIED `zkvm.rs:86-108`). Real on-device ZK ("prove a
  valid order/delivery revealing *nothing*, not even which pseudonym") is a **future** move: 2026 mobile
  proving is real only for **narrow, purpose-built circuits** (Mopro/PSE; Google Wallet's on-device ZK
  age check, longfellow-zk, May 2025 — VERIFIED), while general zkVMs (RISC Zero/SP1) remain
  cloud-proving. Verdict: on-device ZK PoD is a credible future, **not** a pilot dependency; the
  pseudonymous PoD in `pod.rs` is the right pilot primitive.

---

## 3. Network / metadata layer — Tor / i2p / Nym re-evaluated + the latency-tolerance hybrid

This is the re-opened question. **The key design move: layer the transport by latency-tolerance.** Order
*placement* is a single, latency-tolerant request — it can ride a slow anonymizing network with
acceptable UX. Real-time courier↔vendor coordination (live dispatch, GPS, "on my way") **cannot** — every
anonymity overlay measured this session is multi-second per interaction. So: **anonymize the slow leg;
keep the fast leg on iroh/relay.**

### 3.1 The 2026 latency reality (VERIFIED — this is what forces the layering)

| Transport | Single small-request latency (2026) | Real-time-capable? | Vendor-node hosting behind CGNAT | Rust-embeddable in 2026? |
|---|---|---|---|---|
| **Tor onion v3** | **~1.0–1.5s warm** (51 KB onion ≈1s, 200 KB ≈1.2–1.5s — torperf.csv Apr–Jul 2026, VERIFIED); 7–50s cold/congested end-to-end in a 2025 study (VERIFIED-secondary) | No | **Yes — outbound-only, NO public IP / NO port-forward** (VERIFIED — community.torproject.org) | **Yes — `arti-client` 2.5.0 (Jun 2026)**, client production-ready; onion *hosting* "ready but less battle-tested than C-tor" (VERIFIED — arti.torproject.org) |
| **i2p** | **1–3s RTT** warm + tunnel-build overhead; 20–50 KB/s/tunnel; 10-min tunnel churn (VERIFIED — i2p.net/docs/overview/performance) | No | Yes — firewalled/introducer mode, no port-forward (VERIFIED-secondary) | **No (production-unsafe)** — `emissary` Rust router is v0.4.0 "experimental, not for production"; official crates are stale SAM wrappers (VERIFIED — github.com/eepnet/emissary) |
| **Nym mixnet** | **4–5s setup + single-digit-s per-hop** mix delay (VERIFIED-secondary) | No | No — client/VPN-oriented, not a hidden-service host pattern | **Risky** — `nym-sdk` pre-release, **not on crates.io** (VERIFIED-secondary) |

**Design consequence (DESIGN-JUDGMENT):** none of the three is sub-second, and **Tor dominates for our
one latency-tolerant leg** — best tooling (`arti-client` embeddable, official Android browser, Orbot on
both platforms), best CGNAT story (outbound-only, which *matches the ratified vendor-node topology
exactly*), and — decisively — it is the only one with a mature enough stack to bet a small pilot on. i2p
and Nym are **not chosen for the pilot** (i2p: Rust-embed immature + a Dec-2025 large-scale
hidden-service **deanonymization** paper via timing correlation, arXiv 2512.15510 VERIFIED; Nym:
pre-release SDK + subscription model). The relay report's i2p rejection thus **stands — for these
performance/tooling reasons, not the discredited "attribution" premise.** Nym's mixnet remains the
*strongest* metadata protection (it defeats the traffic-analysis Tor doesn't) and its **zk-nym Coconut
credentials** (offline e-cash decoupling payment from usage, live — VERIFIED nym.com/zk-nyms) are worth
watching, but it is a **future/experimental** option, not a pilot dependency.

### 3.2 The Tor onion service as a *relay-free, censorship-resistant, second front door* (the recommendation)

Run the **vendor node as a Tor v3 onion service**, *in addition to* its clearnet SNI-relay address. This
is not "route everything over Tor" — it is a parallel, opt-in path with three distinct wins:

1. **It removes the €6 relay from the path entirely for Tor users.** Onion services are **outbound-only —
   no public IP, no port-forward, no relay** (VERIFIED). So the `.onion` address is a *more sovereign*
   customer↔vendor path than the SNI-relay: no Hetzner box in the middle, no SNI leak, customer IP hidden
   from everyone including the platform. It *strengthens* the ratified topology rather than replacing it.
2. **Metadata privacy for the customer who wants it** — the (c)/vendor+state+platform IP-metadata cells
   in §1 flip from ⛔I/💰C toward achievable, *for customers on Tor.*
3. **Censorship resistance as a first-class, separate benefit.** Even setting anonymity aside, onion +
   **pluggable transports** (WebTunnel 2024, obfs4, Snowflake — all operational 2026, VERIFIED
   support.torproject.org) give dowiz a **circumvention path** if Albania ever ISP-blocks the clearnet
   domain. This is not hypothetical: Albania **blocked TikTok nationally** (Mar 2025 → Feb 2026;
   overturned by the Constitutional Court 2026-03-11 — VERIFIED-in-repo relay report §1.1). A restaurant
   ordering page is a far less likely target, but the *capability* to survive a block is real insurance,
   and it is independent of whether any given customer cares about anonymity. No evidence of Tor blocking
   in Albania was found (VERIFIED-secondary); VPNs remain legal.

**Do NOT use single-onion / non-anonymous mode** (`HiddenServiceNonAnonymousMode 1` +
`HiddenServiceSingleHopMode 1`) to shave latency: it cuts 6 hops → 3 but **fully deanonymizes the vendor
node's IP** (VERIFIED — gitlab.torproject.org #20484) — it trades away exactly the operational anonymity
the onion path exists to provide. Keep full onion mode.

**Do NOT enable onion PoW DoS defense by default:** prop-327 PoW (C-tor 0.4.8) is off-by-default and,
under attack, pushes solve-time onto client CPUs — ~115s on a 2018-class phone (VERIFIED-secondary). For
one-shot mobile customers that is a conversion-killer; enable only reactively under real attack.

### 3.3 The customer-leg honest limit (the single biggest constraint — state it plainly)

**A normal mobile browser cannot reach a `.onion`.** There is no DNS entry, no sanctioned gateway
(Tor2web is dead — v3-incompatible, deprecated 2021), and **no production "Tor-in-WASM" client in 2026**
(the one artifact, `tor-js`, is experimental and needs an external relay; official Arti-in-WASM is an
open, unresolved GitLab issue — VERIFIED). So a customer gets network-metadata anonymity **only if they
already run Tor Browser (Android) or Orbot** (Android true per-app VPN; iOS whole-device only, no
official iOS Tor browser — Onion Browser is an independent project — VERIFIED).

**The honest verdict for the one-shot web customer:** layers (a) data, (b) content, (d) identity, (e)
financial anonymity are **all achievable by default** — but **(c) network-metadata anonymity is NOT
available by default**; it requires the customer to opt in by installing Tor. Therefore:

- **Clearnet QR (default):** plain WSS through the dumb SNI-relay. Full data/identity/content/financial
  anonymity; **no customer-IP privacy** from the relay operator or ISP. This is the mass-market path and
  it is *already* very private — just not network-anonymous.
- **`.onion` mirror (opt-in, printed as a second line on the QR card / shown in-page):** for the
  privacy-conscious or censored customer who runs Tor. Full metadata privacy, relay-free.

This dual-address design is the maximal *honest* answer: it offers real metadata anonymity to whoever
wants it, without pretending the default web visitor has it. (No legal food/commerce onion-service
precedent exists — OnionShare, SecureDrop, Briar, Cwtch, Ricochet-Refresh are all messaging/file-transfer
— VERIFIED; dowiz would be a first, so treat onion-leg UX/latency at commerce scale as unvalidated.)

### 3.4 The courier and vendor legs — keep fast, keep iroh (do NOT Tor-ify)

- **Courier↔vendor real-time (dispatch accept, live GPS, coordination):** stays on **iroh** (QUIC
  hole-punch + self-hosted token-gated relay on the same €6 box) per the relay report — because it is
  real-time and every overlay is too slow. Crucially, **do not put an always-listening onion service on
  the courier phone:** Briar-class always-on Tor listening drains **25–30% battery** (VERIFIED-secondary)
  — untenable for a working courier. The courier is a **push-woken** participant (relay report §, §4.5 of
  arch lens), not an onion host.
- **Dispatch *offer* (latency-tolerant, not the live channel):** *could* be delivered over the vendor's
  onion path if courier-side metadata-hiding from the relay ever becomes a requirement — but for
  vendor-*employed* couriers whose identity the vendor already knows in person, this buys little. Default:
  keep it on iroh. (DESIGN-JUDGMENT.)
- **Metadata minimization that *does* apply to the courier leg:** scope the courier's data view (§2.5,
  Uber-Eats precedent) and mask the customer contact (proxy number / in-app relayed channel).

### 3.5 Push-notification metadata (the unavoidable leak, minimized)

Waking a locked phone requires FCM/APNs, which by design see **device-token ↔ app ↔ timestamp** metadata.
2026 state (VERIFIED-secondary): after the Dec-2023 Wyden revelation, **Apple and Google now require a
court order/warrant** (not a bare subpoena) for push records — but **the metadata still exists and is
retained**, and an April-2026 EFF piece still treats push as a live leakage vector. Design:

- **Content-free wake pings only** — the push payload is "open your app," never order contents (already
  the design — VERIFIED-in-repo `B-data-sync.md:284`). Google/Apple see *that* a courier/vendor got
  pinged, not *what*.
- **UnifiedPush / self-hosted ntfy** reduces this **on Android** (the metadata-clean case). Honest limit
  (VERIFIED): on **iOS**, even self-hosted ntfy still relays through ntfy.sh → APNs, so Apple + ntfy.sh
  see topic/timing metadata. Android-only UnifiedPush is the only fully-clean push path. Fast-follow, not
  pilot-blocking.

### 3.6 Does anonymity change the €6 relay design? (reconciliation — mostly no)

- **Clearnet SNI-passthrough relay: unchanged.** It was already dumb (ciphertext + SNI only, TLS on the
  node — VERIFIED-in-repo). Anonymity doesn't touch it.
- **Onion is an *addition*, not a change** — and a **relay-free** one. The vendor node runs `arti`/C-tor
  as a sidecar making outbound connections; **no new inbound infrastructure**, no second VPS, no change
  to the €6 line item. If anything, the onion path is *cheaper and more sovereign* than the relay for the
  customers who use it. (OHTTP was considered and **rejected** for the customer leg: adversarially, if
  the vendor is both the page-server and the OHTTP gateway it buys *nothing*, and a genuinely independent
  relay means trusting a third party (Cloudflare/Fastly) for the exact property — VERIFIED assessment.
  Tor is architecturally simpler here. iCloud Private Relay / Chrome IP Protection help only their own
  subscribers/incognito-third-party traffic and never cover first-party vendor traffic — VERIFIED — so
  they are a nice-to-have the customer may already have, not something dowiz can provision.)

---

## 4. The anonymous-COD order, traced end-to-end

One order, showing exactly what each actor learns. **Bold = the anonymity property at that step.**

```
0. QR card on the table shows TWO addresses:
   • clearnet:  https://<venue>.order.<domain>.al/s/:slug      (default, fast, no IP privacy)
   • onion:     http://<56-char>.onion/s/:slug                 (opt-in, Tor users, full metadata privacy)
   Customer taps one. NO account, NO login, NO email.        →  PLATFORM LEARNS: nothing (relay sees
                                                                 ciphertext+SNI+timing on clearnet; NOTHING on onion)

1. Page builds cart client-side; mints a FRESH per-order keypair (§2.2).      →  IDENTITY: per-order pseudonym,
   (optional) obtains an anonymous "valid-client" credential (§2.3).             unlinkable to any past order

2. Customer submits a SIGNED ORDER INTENT:
   { slug, items[], delivery_address, contact_handle, order_pubkey, nonce, exp } — NO name required, NO prices.
   • delivery_address + contact_handle are ENCRYPTED under a per-order DATA KEY;
     only H(envelope) rides in the signed intent.                              →  CONTENT: only a hash on the wire
   Transport: WSS→SNI-relay (clearnet) OR direct via Tor (onion).             →  NETWORK: IP hidden iff onion

3. Vendor node: verify credential/nonce → kernel::decide(PlaceOrder) → server-prices in-tx →
   emits SIGNED skeleton events [Priced, StatusChanged→Pending] (PII-free) to the append-only log;
   stores the encrypted PII envelope separately (NOT gossiped to the mesh).    →  VENDOR LEARNS: items (to cook),
                                                                                  address (to route), a contact handle,
                                                                                  a pseudonym — NOT a legal identity,
                                                                                  NOT a card, NOT a cross-order history

4. Fiscalization: node reports the SALE to CIS (Law 87/2019) → NIVF/NSLF receipt.
   Receipt fields are ALL seller-side; NO buyer field (§0, VERIFIED).          →  STATE LEARNS: "vendor sold €X @ 12:04"
                                                                                  — the SALE, NOT the buyer (Floor 2)

5. Dispatch: courier PUSH-WOKEN by a CONTENT-FREE ping (FCM/UnifiedPush).      →  PUSH GATEWAY LEARNS: a ping happened,
   Courier app pulls the offer over iroh, decrypts ONLY the delivery envelope:    not its contents (§3.5)
   address + items-to-hand-over + proxy contact; NO name/phone/payment/history. →  COURIER LEARNS: address (Floor 1) +
   (Uber-Eats "View as Delivery Person" scoping, §2.5.)                           handoff items + a proxy channel — nothing else

6. Delivery: courier arrives, hands food, takes CASH (anonymous).             →  FINANCIAL: no card/bank trail anywhere
   Courier signs PoD (pod.rs): claim = order|courier-VAULT-ID|ts|loc —          (§1 note 16)
   pseudonymous, non-repudiable, no PII, replay-bound to (ts,loc).             →  ATTRIBUTION: pseudonymous, not identifying
   Customer optionally counter-confirms with the per-order key.

7. Settlement: courier→vendor cash remittance, node-signed; double-entry
   obligation closed (COD "books of obligations", §3.8 of arch lens).

8. Crypto-shred: after the 30-day dispute window, DESTROY the per-order data key.
   The address/contact ciphertext rots into noise on every replica;             →  RETENTION: PII gone (EDPB-02/2025
   the signed skeleton (amounts, status, PoD, NIVF ref) survives PII-free.        key-destruction-as-erasure, §2.5)
```

**Net at rest, one week later:** the platform has nothing; the vendor has a PII-free signed skeleton +
a fiscal receipt with no buyer; the courier's device has forgotten the run; the state can see the
vendor's taxed sale but not the buyer; a network observer saw ciphertext (and, on the onion path, not
even who-to-whom). The only irreducible disclosures are the two floors: **the courier saw the address**
(delivery physics) and **the coordinating phone, if a registered SIM was used, is carrier-traceable to
the state** (SIM law). Everything else is honestly anonymous.

---

## 5. Honest cost ledger + phased recommendation

### 5.1 What each anonymity layer costs

| Anonymity mechanism | Latency | UX friction | Battery | Code complexity | Ops burden | Net verdict |
|---|---|---|---|---|---|---|
| No account / per-order pseudonym / opaque track-token | ~0 | **negative** (less friction than signup) | ~0 | Low (largely exists) | ~0 | **Ship default** |
| Unlinkable-by-default orders (fresh per-order key) | ~0 | ~0 | ~0 | Low–Med | ~0 | **Ship default** |
| PII-envelope encryption + crypto-shred | ~0 | ~0 | ~0 | **Med** (log/envelope split, key lifecycle) | Low (a shred job) | **Ship default — highest leverage** |
| Content-free push wake ping | ~0 | ~0 | ~0 | Low (already designed) | ~0 | **Ship default** |
| Courier view-scoping (address+handoff only) | ~0 | ~0 | ~0 | Low–Med | ~0 | **Ship default** |
| Proxy-number / relayed contact channel | ~0 | Low | ~0 | Med | Low–Med (masking provider or in-app relay) | **Pilot opt-in** |
| **Tor `.onion` mirror (vendor node, opt-in)** | **+1–1.5s** (customer, warm) | **High for the mass user** (needs Tor Browser/Orbot); zero for the default clearnet user | Vendor-side sidecar only (**do NOT** put on courier phone: 25–30% drain) | **Med** (arti/C-tor sidecar) | Low (outbound-only, no new infra, no €) | **Pilot opt-in / fast-follow** — also buys censorship resistance |
| Anonymous credentials (KVAC / Privacy-Pass / ARC) | Sub-s | Low | Low | **High** (crypto integration, G09 hybrid-only) | Low | **Fast-follow** (enables dropping the phone) |
| UnifiedPush / ntfy (Android metadata-clean push) | ~0 | Low | ~0 | Med | Med (self-hosted distributor) | **Fast-follow** (partial: iOS still leaks) |
| Nym mixnet order-placement | +4–5s | Med | n/a (vendor/customer) | **High** (pre-release Rust SDK) | Med (subscription/nodes) | **Future/experimental** — strongest metadata privacy |
| On-device ZK PoD (Mopro/longfellow-class) | proving cost | ~0 | Med (proving) | **Very high** (real crypto eng.) | Low | **Future** — `zkvm.rs` is only a seam today |
| i2p anything | 1–3s+ | Med | — | High (no prod Rust router) | Med (i2pd sidecar) | **Reject** (Tor dominates; Dec-2025 deanon paper) |

### 5.2 Phased recommendation

**PILOT — default-anonymous (ship in the first venue, ≈free):** account-less flow + per-order
pseudonym + opaque track-token; **unlinkable orders by default**; **PII-envelope encryption +
crypto-shred**; content-free push; courier view-scoping; cash (already anonymous, VERIFIED-legal);
NO-courier-scoring (already); anonymous-buyer fiscal receipt (already legal). Clearnet SNI-relay
unchanged. **This alone makes dowiz more privacy-preserving than any EU delivery incumbent, at ~zero
cost** — and every piece except the envelope-split is low-complexity.

**PILOT opt-in / fast-follow:** the **`.onion` mirror** on the vendor node (metadata privacy +
censorship resistance, relay-free, no new €); **proxy/relayed contact channel** (blunts Floor 3 against
vendor/courier); **UnifiedPush Android** push (blunts §3.5 on Android).

**FUTURE / earn-it (gated behind a validated MVP, per SYNTHESIS §5–6):** **anonymous credentials**
(KVAC first — issuer==verifier — to *drop the phone requirement* and keep abuse-resistance); **Nym
mixnet** for the order-placement leg if metadata-from-a-global-adversary ever becomes a real
requirement; **on-device ZK PoD** if attribution-without-even-a-pseudonym is ever needed (turn the
`zkvm.rs` seam real).

**NEVER (on current requirements):** i2p (Tor dominates on tooling + a live deanonymization result);
Tor single-onion mode (deanonymizes the vendor); always-on onion listener on courier phones (battery);
routing the real-time courier channel over any mixnet/onion (physics); OHTTP with a self-controlled
gateway (buys nothing).

### 5.3 The one-paragraph honest bottom line

Under local-first + COD, **four of the five anonymity layers are already free or nearly so** — data,
content, identity, and financial anonymity hold against the platform, other users, and a passive
observer by construction, and the fiscal law confirms the *buyer* is anonymous by default. The design
work concentrates in three cells: encrypt-and-shred the PII so the immutable log stays PII-free
(highest-leverage, ships in the pilot), scope what the vendor and courier learn to what they must act on,
and offer a **relay-free Tor `.onion` mirror** for the metadata-anonymity and censorship-resistance that
the default one-shot web customer otherwise cannot have. The two floors are honest and unmovable: **the
courier must learn the delivery address**, and **a registered SIM ties the coordinating phone to the
state via the carrier** — no architecture erases physics or Albanian telecom law. Everything the relay
report feared about "attribution" was real but *pseudonymous and vendor-scoped* (`pod.rs` already proves
delivery without a name), so it composes cleanly with a genuinely anonymous buyer — dowiz can be, and
should be, the delivery hub that *cannot* profile you, and says so as a feature.

---

## Sources

**Repo (read-only, this session):** `docs/design/local-first-hub-2026-07-11/{SYNTHESIS.md,
02-local-first-architecture.md, C-runtime-transport-identity.md, B-data-sync.md, 01-bebop2-protocol-state.md}`;
`docs/research/2026-07-11-relay-hetzner-tailscale-mesh.md`; `crates/bebop/src/{pod.rs,zkvm.rs}` (bebop-repo);
`dowiz/packages/db/migrations/1780310074262_orders.ts`; `dowiz/apps/api/src/routes/customer/track.ts`;
`dowiz/apps/api/src/lib/signals/compute.ts`; `dowiz/docs/security/hardening-findings-2026-07-02.md`.

**Web (four research lanes, fetched/confirmed 2026-07-11; VERIFIED = primary fetched):**
- *Tor/onion 2026:* metrics.torproject.org/torperf.csv (latency); community.torproject.org/onion-services/overview
  (CGNAT outbound-only); arti.torproject.org + blog.torproject.org/arti_2_5_0_released; spec.torproject.org/proposals/327
  (PoW); gitlab.torproject.org #20484 (single-onion); support.torproject.org circumvention (WebTunnel/obfs4/Snowflake);
  play.google.com Tor Browser + apps.apple.com Orbot; npmjs.com/package/tor-js + gitlab arti #20 (no prod Tor-in-WASM);
  onionshare.org, SecureDrop, Briar (battery), Cwtch, Ricochet-Refresh (precedents); blog.torproject.org/financials (funding).
- *i2p/Nym/mixnets:* i2p.net/en/docs/overview/performance (1–3s, 20–50KB/s); i2p.net/blog 2.11.0; github.com/PurpleI2P/i2pd
  releases; arXiv 2512.15510 (i2p deanonymization, Dec 2025); github.com/eepnet/emissary (Rust router experimental);
  nym.com/pricing + nym.com/zk-nyms + nym.com/blog/nym-rust-sdk; arXiv 2501.02933 (Echomix/Katzenpost); github.com/hoprnet/hoprnet;
  docs.rs/veilid-core; unifiedpush.org + docs.ntfy.sh/privacy; thehill/macrumors/techcrunch (Apple push warrant, Dec 2023);
  eff.org/deeplinks/2026/04 (push metadata).
- *Anonymous credentials / OHTTP / ZK:* RFC 9576/9577/9578 + RFC 9474 (Privacy Pass); datatracker draft-ietf-privacypass-arc
  (ARC) + batched-tokens; datatracker draft-irtf-cfrg-bbs-signatures-10; eprint.iacr.org/2019/1416 + signal.org/blog
  (KVAC/sealed-sender/usernames); RFC 9458 + blog.cloudflare.com (OHTTP); blog.cloudflare.com/icloud-private-relay +
  github.com/GoogleChrome/ip-protection; zkmopro.org + github.com/google/longfellow-zk (mobile ZK);
  edpb.europa.eu Guidelines 02/2025 (crypto-shredding) + 01/2025 (pseudonymisation).
- *Albania law + delivery precedents:* sherbimekontabiliteti.al + dddinvoices.com + fiscal-requirements.com (Law 87/2019 —
  no buyer field on B2C cash); howalbania.com/phonetravelwiz.com/wise.com (SIM registration); iapp.org + kpmg.com (Law 124/2024);
  balkaninsight.com data-retention + wilmap.stanford.edu Law 9918 (1–2yr retention); amlwatcher.com + fiscal-requirements.com
  (AML/cash thresholds); petsymposium.org/popets/2024/popets-2024-0039 (Privadome) + dl.acm.org/10.1145/3375752 (CrowdPrivacy)
  + mdpi.com/2078-2489/14/11/597 (DP food delivery) + arXiv 2210.13263 (pRide attack); developer.doordash.com + help.uber.com +
  restaurantdive.com (masking/view-as); coindesk.com + cylab.cmu.edu (OpenBazaar postmortem) + particl.io;
  proton.me/blog/switzerland + bolt.eu/deliveryhero/wolt privacy notices (EU minimization-as-compliance gap).

Labels: VERIFIED = a research lane fetched/confirmed the primary this session; VERIFIED-secondary =
reputable secondary; UNVERIFIED / DESIGN-JUDGMENT flagged inline. All facts subject to the standing G09
hybrid-only crypto constraint for any value/identity-bearing signature.

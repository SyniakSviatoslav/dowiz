# 04 — Anonymity Revision: the Multichannel Hub (supersedes 03's customer-leg + SIM-floor findings)

> **Design revision, 2026-07-11.** This doc **supersedes two specific findings** of
> `03-anonymity-architecture.md` — its §3.3 "the one-shot web customer cannot mesh / cannot reach a
> `.onion`" (its self-described "single biggest constraint") and its **Floor 3** framing that a
> registered Albanian SIM binds *the customer's ordering identity* to the state. Both rested on an
> unstated premise the operator correctly attacked: **that the customer runtime is one crippled
> throwaway web browser.** It isn't — the hub is **multichannel**, and the customer orders through
> whatever funnel they already use, some of which are real peers or anonymous. Doc 03 is **not
> rewritten**: its five-layer decomposition, the PII-envelope + crypto-shred design, the vendor-node
> `.onion` mirror, the pseudonymous `pod.rs` primitive, and the cost ledger all stand and are inherited.
>
> **Governing operator ruling (2026-07-11 redirect, binding):** **no dedicated/separate app.**
> Multichannel stays — the hub accepts orders from many funnels at once (web, messenger bots, social
> DMs — Instagram/WhatsApp/Viber/Telegram, a `.onion` web mirror, a no-phone messenger contact).
> Requiring a customer to install a bespoke dowiz app — or to install SimpleX/Session as a dowiz
> *requirement* — contradicts the hub's identity. The hub is a set of **channel adapters into the one
> `kernel::decide` door** (hub architecture review). **Anonymity must live inside that model**, split
> into two cleanly separable things (§2). This revision drops the earlier "dowiz's own in-app mesh
> messaging app" framing entirely.
>
> **Untouched standing decisions:** local-first ratified; COD mandatory; NO courier scoring; anonymity
> a stated value (SYNTHESIS §9); ratified topology (per-vendor sovereign node + one dumb €6 relay); and
> the max-EV constraint — **zero-friction adoption or nobody tries** (`2026-07-11-MAX-EV-SYNTHESIS.md`),
> the central tension this revision must survive.
>
> **Evidence labels:** **VERIFIED** (primary fetched this session) · **VERIFIED-secondary** (reputable
> secondary/tracker) · **UNVERIFIED** (assessment/snippet, flagged) · **DESIGN-JUDGMENT** (my synthesis,
> falsifiable by building). Web research on 2026 anonymous-messenger state (SimpleX, Session), mobile
> background/push reality, and messaging adoption run fresh 2026-07-11.

---

## 1. The operator's point, steelmanned — and which floors it defeats (now re-framed multichannel)

Doc 03 quietly assumed the customer is **a browser tab that visits a URL once and is thrown away.**
From that one premise two "floors" followed as if they were physics. They are not physics — they are
consequences of that runtime choice. **Change the channel and both dissolve.** But — the redirect's key
sharpening — the way the customer stops being a crippled browser is **not** by installing a dowiz app.
It is that **a multichannel hub accepts orders from channels the customer already has**, and some of
those channels are already real peers or already anonymous. The correction is real; the mechanism is
**channel choice, not a bespoke client.**

**The premise attacked.** A browser is a deliberately sandboxed guest: no raw sockets, no listening
port, no background service, no name-without-DNS (a `.onion`), no long-lived keys. But the customer is
not confined to a browser. They may already run **Tor Browser / Orbot** (which reach a `.onion` with
zero dowiz work), or a **no-phone messenger** (SimpleX/Session), or they may pick a convenient
phone-bound app. The hub's job is only to expose an **adapter** for each and funnel all of them into
`kernel::decide`. Everything doc 03 said "the customer can't do" it *can't do in a browser* — not
"can't do," and not "can't do without a dowiz app."

**Floor defeated #1 — "the customer can't mesh / can't reach `.onion`" (doc 03 §3.3).** This was never a
property of the *customer*; it was a property of the *browser sandbox*. The vendor node can expose a
**`.onion` mirror of its ordinary web order page** (it is already a Tor onion service candidate in doc
03 §3.2 — zero new app, zero new client). A customer who already runs Tor Browser/Orbot reaches it
directly, relay-free, IP hidden. Tor already works on iOS via Orbot/Onion Browser (native apps, not a
browser hack — VERIFIED-in-repo doc 03). **The mesh/anonymous path is usable for the customer leg — as
one more channel. This floor is correctly retired.**

**Floor defeated #2 — "a registered SIM binds the customer's ordering identity to the state" (doc 03
Floor 3).** Doc 03's own cell-note 15 held the escape hatch: the SIM binds identity *"if a registered
phone is used"* for coordination. The floor was really **"if the customer's chosen channel is a phone
number."** If the customer instead orders over a **no-phone channel** — the `.onion`/web link (browser,
no account), a **SimpleX** contact (*no user identifiers of any kind* — VERIFIED), a **Session** contact
(*no phone or email* — VERIFIED), or the vendor's **in-app relayed channel** carrying "I'm outside" over
the per-order pseudonym key (doc 03 §2.5) — then **no phone number is collected and the SIM law is
orthogonal to ordering.** The customer's choice, not a dowiz mandate. **This floor is retired for the
customer** (it re-lands on the courier — §4).

**Floor blunted #3 — telecom traffic-metadata retention (doc 03 Floor 4), for the customer.** If the
customer's chosen channel is onion-routed (Tor Browser to the `.onion`, or an onion-routed messenger),
the ISP's 1–2yr retained metadata shows *"used Tor / an onion-routed app at 20:14,"* **not** *"…talked
to Vendor X."* The who↔whom edge the state most wants is broken **by the customer's channel choice** —
the exact win doc 03 reserved for the tiny Tor-Browser minority, now simply one of the offered channels.

**The honest scope.** The operator defeats **two floors doc 03 mislabeled as physics and blunts a
third** — all on the *customer* leg, all by recognizing the customer picks a channel rather than being
confined to a sandbox. It does **not** defeat the floors that are genuinely physical or live on other
actors (the address, the courier's SIM, the push wall, real-world correlation — §4). And crucially,
**dowiz achieves this by offering an anonymous channel as an equal option, never by shipping or
requiring a client.** The correction is real and bounded.

---

## 2. Anonymity in a multichannel hub = a hub guarantee (always on) + channel choice (honest pass-through)

The redirect's central move: the multichannel model **cleanly separates anonymity into two things**,
one dowiz controls and one it does not. Stating them apart is the whole design.

### 2.1 (a) Hub-side, channel-INDEPENDENT anonymity — dowiz's guarantee, always on

This holds for **every** order **regardless of the funnel it arrived through**, because it lives in the
hub, behind the adapters, at the `kernel::decide` door — exactly doc 03's "four free layers," restated
as an invariant of the multichannel design:

- **No central customer profile** — local-first means no dowiz order database exists to profile you
  (doc 03 §2.1; VERIFIED-in-repo topology). Accounts are optional; the default handle is a **fresh
  per-order pseudonym** (doc 03 §2.2).
- **PII in a per-order envelope, only a hash in the signed log** — delivery address + coordination
  contact encrypted under a per-order data key; only `H(envelope)` enters the immutable, replicated
  event log (doc 03 §2.5).
- **Crypto-shred after the dispute window** — destroy the per-order key at ~30 days; the PII ciphertext
  rots to noise on every replica; the PII-free signed skeleton (amounts, status, PoD, NIVF ref)
  survives (doc 03 §2.5; EDPB 02/2025 key-destruction-as-erasure).
- **Data stays on the vendor node; not gossiped to the mesh** — the PII envelope lives on the vendor's
  own device (and the courier's for the run), never broadcast.
- **Financial anonymity by construction** — COD cash, zero card/gateway/PCI surface anywhere (doc 03
  §1 note 16).

**This is what dowiz can promise identically to a customer on a Telegram bot, an Instagram DM, or a
`.onion` link.** It is channel-independent because it is applied *after* the adapter, in the kernel.
It is the honest, always-true half of "the delivery hub that cannot profile you."

### 2.2 (b) Per-channel network/metadata anonymity — the customer's CHOICE, mostly OUTSIDE dowiz's control

Network-metadata anonymity (IP, location, who↔whom, SIM binding) is a property of **the channel the
customer picks**, and dowiz **neither adds nor removes it** — the hub just accepts the order at the far
end. dowiz **cannot** make Telegram anonymous (phone-bound, Telegram server metadata) or Instagram/
WhatsApp anonymous (Meta holds identity + metadata). The honest design move is to **map each real
channel's anonymity as pass-through and label it truthfully**, then offer an anonymous channel as an
equal option. The per-channel map (properties VERIFIED where cited; dowiz-control column is
DESIGN-JUDGMENT on the adapter model):

| Channel (adapter) | Phone/SIM bound? | IP hidden from vendor/ISP? | Third-party metadata holder | Network-anon? | dowiz's role | Honest label |
|---|---|---|---|---|---|---|
| **Web link (ordinary browser)** | No (no phone collected) | No — relay + ISP see IP↔vendor+timing | none (dumb relay sees SNI+IP+timing) | ❌ | pass-through | *"No profile, no card — but NOT network-anonymous."* |
| **Web link over Tor Browser** (customer's own choice) | No | **Yes** (customer's Tor, then relay) | Tor network | ✅ (customer-supplied) | pass-through, **zero dowiz work** | *"Network-anonymous — because YOU used Tor."* |
| **`.onion` web mirror** (vendor node = Tor onion service) | No | **Yes**, relay-free (doc 03 §3.2) | Tor network only | ✅ | **one adapter dowiz exposes** | *"Network-anonymous by default; no relay in path."* |
| **No-phone messenger contact** (SimpleX / Session) | **No** (SimpleX: no IDs; Session: no phone — VERIFIED) | Yes (queue/onion-routed) | SimpleX SMP relays / Session node network | ✅ | pass-through adapter, if a customer already uses it | *"Anonymous — via a messenger YOU already run."* |
| **Telegram bot** | **Yes** — Telegram requires a phone number (SIM floor returns) | No — Telegram servers see it | Telegram (server-side metadata; bot API not E2EE) | ❌ | pass-through | *"Convenient, NOT anonymous — Telegram knows your number."* |
| **Instagram / WhatsApp DM** | **Yes** (Meta identity/number) | No | **Meta** (identity + metadata) | ❌ | pass-through | *"Convenient, NOT anonymous — Meta holds your identity."* |
| **Viber** | **Yes** (phone number) | No | Viber/Rakuten servers | ❌ | pass-through | *"Convenient, NOT anonymous — phone-bound."* |

**Reading the map:** the channels with real Durrës reach (Telegram/IG/WhatsApp/Viber) are exactly the
**phone-bound, non-anonymous** ones — the SIM floor and third-party metadata return in full, and dowiz
*cannot* fix that from behind the adapter. The **anonymous** channels (Tor-web, `.onion` mirror, no-phone
messenger) are real and give genuine network anonymity, but they are **the customer's existing choice**,
not something dowiz installs on them. dowiz's honest contribution to (b) is narrow but real: **expose an
anonymous channel as an equal option (the `.onion` mirror is free — the vendor node already runs Tor in
doc 03), and label every channel's privacy level truthfully.**

### 2.3 Messenger / mesh runtimes reassessed as ADAPTERS (not installs), verified 2026

The prior task framed SimpleX/Session/"dowiz's own app" as *clients to build or require*. Under the
redirect they are re-cast as **potential channel adapters** the vendor *could* expose for customers who
*already* use them — never a required install. The verified 2026 facts:

- **SimpleX Chat (VERIFIED — simplex.chat, GitHub).** *"The first messaging network operating without
  user identifiers of any kind"* — no account, no phone, not even a random long-term ID; per-conversation
  **pairwise unidirectional queues** so no server reconstructs the social graph; Double-Ratchet E2EE;
  v6.5.6 (2026-06-22). **Not P2P — needs SMP relay servers** (self-hostable — VERIFIED), so the
  relay/rendezvous floor persists. AGPL-3.0; Haskell `simplexmq` core + a **TypeScript/JS bot SDK** — so
  a vendor could run a **SimpleX order bot** as an adapter (the light path) without any customer install
  beyond the customer's own SimpleX app. **iOS push caveat (VERIFIED):** Apple push must route via
  SimpleX Chat Ltd's notification server → APNs (metadata-minimized, but present). Adoption: ~2M
  *lifetime downloads* globally by New Year 2026 (VERIFIED-secondary) → **≈0 in Durrës.**
- **Session (VERIFIED — getsession.org/faq).** No phone/email; random Account ID; **onion requests**
  (3-hop, Tor-like) to **swarms of 5–7 Service Nodes**. **May-2025 migration off the Oxen blockchain
  onto a dedicated "Session Network" backed by an Ethereum-compatible L2 token (SESH)**; stewardship to
  the **Swiss Session Technology Foundation, Oct 2024**; app repos relocated `oxen-io`→`session-foundation`
  Oct–Dec 2025 (relocation, not death — ~1M+ users, ~6M lifetime downloads — VERIFIED-secondary);
  **Protocol V2 (Dec 2025)** adds forward secrecy + lattice PQ. **Live caveat for a trust-critical
  channel:** *Practical Attacks on Session Messenger and Oxen Blockchain* (Yu & Haines, ANU; EuroS&P,
  approved 2026-04-22) found **seven vulnerabilities, incl. Oxen-consensus flaws enabling realistic
  network takeover** (VERIFIED — eprint 2026/773). iOS/Android "fast mode" exposes IP+token to APNs/FCM+STF
  (VERIFIED). Adoption in Durrës: **≈0.**
- **The strongest anonymous channel is NOT an app — it is the `.onion` web mirror.** The single
  highest-leverage anonymous adapter needs **zero new client and zero new build the vendor isn't already
  getting**: the vendor node runs as a **Tor v3 onion service** (doc 03 §3.2, VERIFIED outbound-only, no
  relay, no public IP), exposing the *same web order page* at a `.onion` address printed as a second line
  on the QR card. Any customer with Tor Browser/Orbot reaches it. This is the "private tier" — **an
  anonymous channel, not an app** (§5).

**Verdict:** SimpleX/Session genuinely remove the customer SIM floor and give real network anonymity,
and either could be a niche adapter — but with ≈0 Durrës install base, requiring them would wreck the
funnel (§4). The **`.onion` web mirror** delivers the same network-anonymity win as an *equal, no-install
channel option* and is the honest recommendation for dowiz's own anonymous adapter.

---

## 3. Revised anonymity matrix — the cells that flip (delta against doc 03 §1)

Doc 03's full 5-layer × 6-actor matrix stands; only the cells whose verdict depended on the "browser
customer" premise move — and now they move **for a customer who CHOOSES an anonymous channel**, not for
a dowiz-app user. Legend: **✅F** free/on · **💰C** achievable-with-cost · **⛔I** impossible (hard floor).
A new **Push-gateway** column and a new **physical/behavioral** row are added — the correction surfaces
them as the real residuals.

| Layer ↓ \ FROM → | Platform (relay) | Vendor | Courier | Network/ISP | The State | **Push gateway (NEW)** |
|---|---|---|---|---|---|---|
| **(c) Network / metadata (IP, who↔who)** | doc03 💰C → **✅F** ⁽ᴬ⁾ | doc03 💰C/⛔I → **✅F (IP) / ⛔I (address)** ⁽ᴮ⁾ | ⛔I *(unchanged — address, Floor 1)* | doc03 💰C → **✅F on an anon channel** ⁽ᶜ⁾ | doc03 💰C/⛔I → **💰C** ⁽ᴰ⁾ | **💰C/⛔I** ⁽ᴱ⁾ |
| **(d) Identity / credential** | ✅F *(unchanged)* | 💰C *(unchanged)* | 💰C *(unchanged)* | ✅F *(unchanged)* | doc03 ⛔I (SIM) → **💰C** ⁽ᶠ⁾ | · |
| **(f) Physical / behavioral (NEW ROW)** | ✅F | ⛔I ⁽ᴳ⁾ | ⛔I ⁽ᴳ⁾ | ✅F | ⛔I ⁽ᴳ⁾ | · |

**Delta notes (load-bearing):**
- **(A)** On the `.onion` mirror the €6 SNI-relay is **not in the path at all** (onion is relay-free — doc
  03 §3.2 VERIFIED) → the platform sees *nothing*. Flips 💰C→✅F **for customers who pick the anon
  channel.** Default clearnet-web/phone-messenger customers are unchanged (still 💰C / not-anon).
- **(B)** The vendor cannot learn the customer's IP once onion-routed (✅F), but **the delivery address is
  still ⛔I — Floor 1 untouched.** Network anonymity ≠ address anonymity.
- **(C)** A passive ISP sees "used Tor/onion-routed channel," not the vendor edge — given **by the
  customer's channel choice**, not a dowiz client.
- **(D)** The state's SIM+retention reconstruction of the *customer* breaks (no phone; onion-routed), so
  it drops ⛔I→💰C — **but the vendor node stays lawfully compellable** (doc 03 note 7), yielding the
  per-order pseudonym + address, not a state-linked identity. So 💰C, not ✅F.
- **(E) — the new sharp cell.** Backgrounded status updates need APNs/FCM (or a messenger push server
  that itself uses them) → device-token ↔ app ↔ timing = a **de-anonymizing edge to Apple/Google** (§4.3).
  💰C only via Android-UnifiedPush / foreground-only; on iOS it trends ⛔I.
- **(F)** Identity-from-the-state flips ⛔I→💰C **because no registered phone is used** — the operator win,
  now gated on the customer choosing a no-phone channel. Residual: the vendor node can still be compelled.
- **(G) — the new row.** The **delivery address, repeat-address patterns, and the cash handoff the courier
  witnesses** de-anonymize the customer to vendor/courier/state **regardless of channel.** ⛔I by physics.

**What still binds after the correction:** (c)/courier address (Floor 1); (b) content-from-vendor and
content-from-state via the compellable node (doc 03, unchanged); the **push-gateway edge** (new,
sharpest); the **physical/behavioral row** (new); the **courier's own SIM** (§4.2); and — the redirect's
addition — dowiz can **label** each channel's network-anonymity honestly but **cannot enforce** it when
the customer picks convenience (§4.6).

---

## 4. The counterarguments — rigorous, not strawmanned (the operator asked for these)

§§1–3 grant the operator's point fully. Here is the honest other side. Each is a real cost.

### 4.1 The zero-friction collision — the decisive one

Max-EV's binding constraint is **zero-friction or nobody tries** (walk-in demo → scan a QR → order in
<10 min, **no install**, no account — MAX-EV §2–3). Making an *anonymous* channel the **default** blows
up the funnel:
- **SimpleX / Session have ≈0 installed base in Durrës.** SimpleX ≈2M *lifetime downloads globally*
  (not MAU) at New Year 2026; Session ≈1M users / ~6M downloads (VERIFIED-secondary). Against a market
  whose validated wedge is **InstaPorosi's QR→WhatsApp, 0-install** and where **Facebook is 95.6% of
  referral traffic** (MAX-EV market lens), requiring a niche privacy app the customer has never heard of
  converts like a cold app-store install mid-purchase — **≈0.** Even the `.onion` mirror needs Tor
  Browser, which the mass beach customer does not run.
- **The channels with reach are the non-anonymous ones.** The only messengers with real Durrës presence
  are phone-bound (WhatsApp ≈3.3B MAU Jan-2026; Viber ≈230M MAU, #1 in several Balkan states —
  VERIFIED-secondary; Telegram; IG DMs) → **the SIM floor + third-party (Meta/Telegram) metadata return
  in full.** So the adoption-viable channels are non-anonymous and the anonymous channels are
  adoption-dead. **A genuine dilemma, not a design gap** — the two goals pull opposite directions and no
  engineering collapses them. The honest resolution (§5): the anonymous channel is an **equal option**,
  never the default and never a required install.

### 4.2 The courier still needs cellular → the SIM floor moves, it doesn't die

The customer can drop the phone; **the courier cannot.** A courier moving through Durrës needs live
cellular to receive dispatch, update status, and be reachable — and Albanian prepaid SIM registration is
mandatory (doc 03 Floor 3, VERIFIED-secondary). **Floor 3 migrates from the customer to the worker.**
Softened (the courier is vendor-employed, known in person, so anonymity-from-the-vendor was never their
goal), but *anonymity-from-the-state* for the courier is bounded by carrier registration + retention,
and the courier's device-location trail is far richer than a one-shot order. And the courier leg is
real-time, so it **cannot** be onion-routed anyway (every overlay is multi-second — doc 03 §3.1).

### 4.3 Background delivery vs anonymity — the sharpest residual floor

A hub that pushes order-status to the customer hits the OS background wall regardless of channel. An app
(messenger or PWA) **backgrounded on iOS holds no sockets — ever** (C-lens §1.3, VERIFIED); Android only
with a foreground service + battery exemption + OEM-killer fight, 6h-capped. So reliable "your order is
5 min away" while backgrounded needs one of two lossy options:
- **Keep the surface foregrounded** — the only push-free path; UX-hostile (the customer must watch the
  app). Tolerable for a 10-min "on my way" window, untenable generally.
- **Push to wake** — APNs/FCM, or a messenger's own push server which *itself* relays through APNs/FCM.
  This re-introduces a **de-anonymizing Big-Tech edge**: device-token ↔ app ↔ timing, retained,
  warrant-gated but still collected (EFF Apr-2026: *"Apple and Google now both require a judge's order"*
  yet Apple *"shares data on hundreds of users"*; Wyden Dec-2023 — VERIFIED). The anonymous messengers
  prove there is no escape: **SimpleX iOS push routes through SimpleX Chat Ltd → APNs** (VERIFIED);
  **Session iOS fast-mode exposes IP+token to Apple+STF** (VERIFIED). Content-free wake pings +
  Android-only UnifiedPush (doc 03 §3.5) minimize but do not remove it. **This floor survives even the
  strongest anonymous channel** and is the single most honest limit of the revision.

### 4.4 Physical + behavioral correlation — network anonymity ≠ real-world anonymity

Even a perfect transport delivers food to **a physical address a courier sees, repeatedly.** The address,
the **repeat-address pattern** ("same flat every Friday"), the face at the door, and the **cash handoff**
de-anonymize the customer no onion route touches (matrix row (f)). A motivated adversary correlates *"who
lives at the drop"* far more cheaply than *"who owns the IP."* The mesh buys **network** anonymity, never
**delivery** anonymity.

### 4.5 Self-hosted / anonymous channels inherit the same relay / rendezvous / NAT floor

"Order over an anonymous channel" does not delete infrastructure: the **iroh** vendor↔courier leg still
needs the **€6 relay** for the ~30% of connections hole-punch can't make direct (CGNAT — C-lens §2.3,
VERIFIED); the **`.onion`** leg removes *your* relay but substitutes **the Tor network** as rendezvous
(+~1–1.5s, no legal-commerce onion precedent — doc 03 §3.2–3.3); **SimpleX** needs SMP relays; **Session**
needs the SESH-staked node network + its L2 (2026 consensus-attack caveat). **The €6 relay the correction
implied was optional is still there for the fast leg** — it was never the customer-onion leg's cost.

### 4.6 dowiz can LABEL each channel honestly, but cannot ENFORCE network anonymity (the redirect's addition)

Because anonymity is now **channel-choice + hub-guarantee** rather than a controlled client, dowiz's
power over the *network* half is only to **offer and to tell the truth** — not to enforce. If a customer
picks the convenient phone-bound channel (Telegram/IG/WhatsApp/Viber), they get the always-on **hub
guarantee** (§2.1) but **no network anonymity**, and dowiz cannot retrofit it from behind the adapter.
The best dowiz honestly does: (i) **apply the hub guarantee to every channel identically**; (ii) **offer
an anonymous channel (`.onion` mirror / no-phone messenger) as an equal option**; (iii) **label each
channel's privacy level truthfully** so the choice is informed. Promising "anonymous ordering" as a blanket
claim would be a lie for most channels; the honest claim is *"we never profile you on any channel, and we
offer a network-anonymous channel if you want one."*

---

## 5. The reconciled design — multichannel-native, no dedicated app

The counterarguments **bound** the operator's point without defeating it. The resolution is the
redirect's model: **every channel gets the always-on hub guarantee; network anonymity is the customer's
channel choice; and dowiz exposes one anonymous channel as an equal, no-install option.** Nothing is a
new floor for the default customer; the anonymous channel is purely additive.

### 5.1 The channel tiers (all funnel into one `kernel::decide` door)

| Tier | Channels (adapters) | Install? | Hub guarantee (§2.1)? | Network anonymity | Honest label |
|---|---|---|---|---|---|
| **Default (max-EV)** | Clearnet **web link** (QR); **Telegram/IG/WhatsApp/Viber bots & DMs**; coordinate via **in-app relayed channel** (no phone) or the customer's own number | **No** | **Yes — always** | ❌ (phone-bound / relay+ISP see IP) | *"No profile, no card — NOT network-anonymous. Your messenger/carrier still sees you."* |
| **Anonymous option (opt-in, equal, no install of ours)** | **`.onion` web mirror** of the same order page (vendor node = Tor onion service, doc 03 §3.2, zero new client); or a **no-phone messenger contact** (SimpleX/Session) for customers who already run one; or the **customer's own Tor Browser** to the clearnet link | **No dowiz install** (customer uses Tor Browser / a messenger they already have) | **Yes — always** | ✅ IP hidden from platform+vendor+ISP; state who↔whom edge broken; **no SIM binding** | *"Network-anonymous — reachable if you already use Tor / an anonymous messenger. Address + a push ping are the only residuals."* |

**Why this is the honest max-EV answer (DESIGN-JUDGMENT):** the default preserves the zero-friction
multichannel funnel that is the *only* thing keeping the pilot alive (§4.1) and is *already* more private
than any EU incumbent via the always-on hub guarantee — it just isn't *network*-anonymous on phone-bound
channels, and says so. The anonymous option gives the operator's correction its full due **for users who
already have the tools**, at **zero new client and near-zero cost** (the vendor node already runs Tor in
doc 03). **Anonymity becomes a channel you can pick, never an app you must install or a tax on ordering
dinner.**

### 5.2 One fully-anonymous order, over an EXISTING anonymous channel (no dowiz app)

Traced over the **`.onion` web mirror** — an ordinary web page served by the vendor node as a Tor onion
service, reached by the customer's **own Tor Browser/Orbot** — terminating in the same hub guarantees.

```
0. Vendor's QR card prints TWO lines:  clearnet https://<venue>.order.<domain>.al/s/:slug   (default)
                                        onion    http://<56-char>.onion/s/:slug              (anonymous option)
   Customer opens the .onion in Tor Browser/Orbot they ALREADY run.  → PLATFORM: nothing (onion = NO relay in path)
   No dowiz app, no account, no phone, no SIM.
1. The ordinary web page builds the cart client-side; mints a FRESH
   per-order pseudonym keypair.                                       → NETWORK/ISP: sees "Tor traffic," NOT who↔vendor
2. Signed order intent { items[], H(address+contact envelope),
   order_pubkey, nonce, exp }; address+contact ENCRYPTED under a
   per-order data key. No phone number anywhere.                      → IDENTITY: no phone → NOT SIM-bound (Floor 3 gone)
3. Adapter → kernel::decide(PlaceOrder): verify, server-price,
   emit PII-free signed skeleton; store encrypted PII envelope on
   the vendor node (NOT gossiped).            [HUB GUARANTEE, §2.1]   → VENDOR: items+address+pseudonym; NO name/IP/card/history
4. Fiscalization (Law 87/2019): reports the SALE; all seller-side.    → STATE: "vendor sold €X @ 20:14" — the sale, not the buyer
5. Order-status back to customer:
   • Tor Browser tab kept open → live over the onion channel          → PUSH GATEWAY: nothing (push-free)
   • backgrounded → content-free wake ping via APNs/FCM/UnifiedPush   → PUSH GATEWAY: token↔app↔timing   ⚠ RESIDUAL FLOOR
6. Courier (real-time on iroh — NOT onion): decrypts ONLY the
   delivery envelope; cellular on a REGISTERED SIM.                   → COURIER: address (Floor 1); courier SIM state-traceable ⚠
7. Handoff: food for CASH; courier signs pseudonymous PoD (pod.rs).   → FINANCIAL: no trail. PHYSICAL: address+face+cash seen ⚠
8. Crypto-shred the per-order data key after the dispute window.      → RETENTION: PII gone; PII-free skeleton survives
```

Same trace over a **no-phone messenger contact** (SimpleX/Session) is identical from step 3 on — the
adapter differs, the hub guarantee and floors do not; the messenger's push server replaces the direct
onion channel and adds its own APNs/FCM residual at step 5.

### 5.3 What each tier gives, costs, and the floors that survive even the anonymous channel

- **Default gives:** the full always-on **hub guarantee** (§2.1) on *every* channel — no profile, PII
  envelope + crypto-shred, cash. **Costs:** ~nothing (it's the funnel). **Does NOT give:** network
  anonymity — phone-bound channels leak identity to the carrier/Meta/Telegram; clearnet web leaks IP to
  relay+ISP. Labeled plainly.
- **Anonymous option gives:** all of the above **plus** network-metadata anonymity (IP hidden from
  platform/vendor/ISP; state who↔whom edge broken; **no SIM binding**). **Costs:** the customer must
  already have Tor Browser / an anonymous messenger (adoption-hostile → opt-in, never default); ~1–1.5s
  onion latency; the vendor exposing the `.onion` mirror (near-free — already in doc 03). **No dowiz
  client install, ever.**
- **Floors that survive even the anonymous channel (the honest bottom line):**
  1. **The delivery address** reaches the courier — physics (Floor 1).
  2. **The push/background wall** — backgrounded status needs APNs/FCM (or a messenger push server that
     uses them); foreground-only is the sole push-free path. **Sharpest residual.**
  3. **Physical/behavioral correlation** — address recurrence, the face, the cash handoff.
  4. **The courier's SIM** — the mover needs registered cellular; Floor 3 leaves the customer, lands on
     the worker.
  5. **The compellable vendor node** — lawful process still yields the per-order pseudonym + address
     (never a card, never a cross-order profile, never a name).
  6. **Rendezvous/NAT floor** — the iroh leg still needs the €6 relay; the onion leg still needs Tor.
  7. **Label ≠ enforce** — dowiz can truthfully label each channel's network-anonymity but cannot enforce
     it when the customer picks a convenient non-anonymous funnel (§4.6).

**One-paragraph verdict.** The operator is right: a multichannel hub means the customer is not a crippled
throwaway browser — they order through channels they already have, some of which are real peers or
already anonymous, so **two of doc 03's "floors" dissolve** (browser-can't-mesh, SIM-binds-customer) and
a third (ISP retention) is blunted — **and dowiz achieves this without shipping or requiring any app.**
The clean design is to split anonymity in two: an **always-on hub guarantee** dowiz applies behind every
adapter (no profile, PII-envelope + crypto-shred, cash — true on Telegram and `.onion` alike), and
**per-channel network anonymity that is the customer's honest choice** (dowiz labels every channel
truthfully and exposes a free anonymous one — the `.onion` web mirror — as an equal option). **But the
correction does not make delivery anonymous:** the address, the courier's SIM, the physical/cash handoff,
and above all the **iOS push/background wall** survive the strongest channel, and the anonymous channels
have ≈0 Durrës reach so none can be the default without destroying the funnel. dowiz can honestly be *the
delivery hub that profiles you on no channel and offers a network-anonymous one* — without pretending
every channel is anonymous, that anonymity is free, or that a hub should ever make you install its app.

---

## Sources

**Repo (read-only, this session):** `docs/design/local-first-hub-2026-07-11/{03-anonymity-architecture.md,
SYNTHESIS.md, C-runtime-transport-identity.md, 02-local-first-architecture.md, B-data-sync.md}`;
`docs/research/{2026-07-11-relay-hetzner-tailscale-mesh.md, 2026-07-11-MAX-EV-SYNTHESIS.md}`;
`crates/bebop/src/{pod.rs,zkvm.rs}` (via doc 03/C-lens citations); hub architecture review (channel-adapter model).

**Web (fetched/confirmed 2026-07-11; VERIFIED = primary fetched this session):**
- *SimpleX:* simplex.chat + simplex.chat/docs + simplex.chat/blog (no user identifiers, pairwise queues,
  SMP/XFTP relays self-hostable → not P2P, iOS notification-server design, v6.5.6); github.com/simplex-chat
  (AGPL-3.0, Haskell `simplexmq` core + TypeScript bot SDK); ~2M downloads / New Year 2026 (VERIFIED-secondary).
- *Session:* getsession.org/faq (no phone/email, onion requests, swarms 5–7 nodes, fast-mode push exposes
  IP+token to APNs/FCM+STF); blackoutvpn.au Session deep-dive (May-2025 Oxen→Session-Network L2 SESH,
  Dec-2025 Protocol V2 PFS + lattice PQ); github.com/session-foundation (repo relocation Oct–Dec 2025);
  getsession.org/blog/connecting-one-million-users + appbrain (~1M users / ~6M downloads, VERIFIED-secondary);
  Oct-2024 STF stewardship transfer.
- *Session security:* eprint.iacr.org/2026/773 — Yu & Haines (ANU), *Practical Attacks on Session Messenger
  and Oxen Blockchain*, EuroS&P, approved 2026-04-22 (seven vulns; Oxen-consensus network-takeover; V1
  group-chat flaws) — VERIFIED.
- *Push / background reality:* eff.org/deeplinks/2026/04 (push metadata, judge-order requirement, "hundreds
  of users," iOS 26.4.2/18.7.8 notification-DB fix Apr-2026); arXiv 2407.10589 (>half of 21 messengers leak
  metadata to FCM); C-lens §1.3 VERIFIED iOS/Android background limits (developer.apple.com backgroundtasks,
  developer.android.com fgs/timeout + doze, dontkillmyapp.com).
- *Channel-adoption baseline:* WhatsApp ≈3.3B MAU Jan-2026 / Viber ≈230M MAU #1 in several Balkan states
  (VERIFIED-secondary — businessofapps, sinch, similarweb); Telegram phone-number requirement; Albania-specific
  split UNVERIFIED (no country data found); InstaPorosi QR→WhatsApp wedge + Facebook 95.6% referral
  (VERIFIED-in-repo, MAX-EV market lens).

Labels: VERIFIED = primary fetched this session; VERIFIED-secondary = reputable secondary; UNVERIFIED /
DESIGN-JUDGMENT flagged inline. All value/identity-bearing crypto remains under the standing G09
hybrid-only / audited-classical-half constraint.

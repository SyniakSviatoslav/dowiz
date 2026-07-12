# Relay & Transport Decision — Hetzner vs Tailscale, and the Mesh/P2P-Overlay Question

> **Research report, 2026-07-11.** Decides the RELAY/TRANSPORT layer for the local-first hub's
> ratified topology (per-vendor sovereign node + one dumb stateless reachability relay — see
> `docs/design/local-first-hub-2026-07-11/SYNTHESIS.md §4`, `02-local-first-architecture.md §2-4`,
> `C-runtime-transport-identity.md §2`). Read-only session; the only file created is this one.
> Web research fresh 2026-07-11. Labels: **VERIFIED** (primary source fetched this session, or
> official doc quoted verbatim by the fetch), **VERIFIED-secondary** (reputable tracker/snapshot,
> not the vendor's own page), **VERIFIED-in-repo-doc** (claim carried from a sibling lens that
> cites its own primary), **UNVERIFIED**, **ESTIMATE** (my calculation/inference).
>
> Standing constraints honored throughout: COD-mandatory cash rail (serverless money),
> no-courier-scoring, local-first ratified, beachhead = Durrës, Albania. The relay's contract is
> the ratified red-line: **it carries ciphertext; it never decrypts, prices, sequences, or decides.**

---

## 0. The three headline findings (read these first)

1. **The premise "Tailscale Funnel might terminate TLS" is FALSE — verified.** Tailscale's own
   docs: *"The Tailscale server running on your device receives the encrypted request from the TCP
   proxy. It then terminates the TLS connection and passes the decrypted request to the local
   service"* and *"Funnel relay servers do not decrypt the traffic between public devices and your
   device."* (VERIFIED — tailscale.com/docs/features/tailscale-funnel, fetched today.) **Both
   candidates preserve the dumb-relay property.** The decision therefore turns on other axes —
   and the decisive one turns out to be the **domain**.
2. **The domain is the real lock-in, because of the QR codes.** Funnel can only serve
   `<node>.<tailnet>.ts.net` (ports 443/8443/10000 only; no custom domains — VERIFIED, same doc).
   dowiz's customer leg is a **printed QR on a table** encoding the storefront URL. Print a
   `ts.net` URL and Tailscale owns your physical front door: migrating means re-printing every
   sticker in every venue. With your own domain on a Hetzner box, the URL is yours forever and the
   relay behind it is swappable in one DNS record. This single fact settles primary vs fallback.
3. **Hetzner raised prices on 15 June 2026** — the sibling lens's "€4.15/mo" figure is already
   stale. CAX11 went €4.49→€5.99/mo (VERIFIED — docs.hetzner.com price-adjustment page); CPX/CCX
   more than doubled (CPX22 €7.99→€19.49, CCX13 €15.99→€42.99, same source). The relevant plan
   today is **CX23 (2 vCPU/4 GB/40 GB, 20 TB traffic incl.) at ~€5.49/mo** (VERIFIED-secondary —
   costgoat.com Hetzner snapshot, July 2026; the repo doc's €4.15 is consistent as the pre-VAT
   and/or pre-adjustment net price — ESTIMATE on the reconciliation). **Budget ≤€6/mo all-in**
   (incl. the €0.50/mo IPv4 — VERIFIED-secondary). The "€4 relay" is now a "€6 relay"; the
   conclusion survives, the line item changes.

---

## 1. Hetzner (self-hosted VPS relay) vs Tailscale (Funnel/DERP)

### 1.1 The facts, per candidate

**Hetzner self-hosted relay (nginx `stream` SNI-passthrough, or frp/rathole TCP mode):**

- **Price (2026-07):** CX23 ~€5.49/mo, 2 vCPU/4 GB/40 GB NVMe, **20 TB traffic included**
  (DE/FI locations); CAX11 (ARM) €5.99/mo official; IPv4 +€0.50/mo; overage €1/TB in EU
  (VERIFIED-secondary — costgoat July-2026 snapshot; CAX11 VERIFIED — docs.hetzner.com
  price-adjustment 15-06-2026). US locations cost ~+20% and include as little as 1 TB — irrelevant
  here; use **Falkenstein or Nuremberg** (closest to Albania) or Helsinki.
- **Traffic headroom:** order traffic ≈1.5 GB/mo at 1,000 orders/day (VERIFIED-in-repo-doc,
  `02-local-first-architecture.md §4.4`) = **0.0075% of the 20 TB allowance**. Traffic will never
  be the cost. (ESTIMATE arithmetic on a verified figure.)
- **TLS model:** nginx `stream` module routes by **SNI without terminating TLS**; the CA-valid
  cert (Let's Encrypt DNS-01, works behind CGNAT) lives **on the vendor node**. The relay sees
  SNI hostname + IPs + timing — never plaintext. (VERIFIED-in-repo-doc §4.4; standard,
  well-documented nginx behavior.)
- **Ops terms:** basic DDoS mitigation **included free, no opt-in** (VERIFIED — hetzner.com);
  Cloud SLA **99.9% monthly** with cloud-credit compensation beyond 43 min/mo downtime, SLA page
  updated 2026-03-16 (VERIFIED — docs.hetzner.com/general/company-and-policy/slas-cloud).
- **Ops burden:** one stateless process on one box. Debian + unattended-upgrades + a 20-line
  nginx stream config, rebuildable from cloud-init in minutes because **it holds no state at
  all**. Realistic effort: ~1-2 h setup, ~0 h/mo steady-state, occasional reboot for kernel
  patches (ESTIMATE). Failure mode: new customer orders can't arrive until DNS fails over;
  in-flight orders on LAN/direct paths continue (kill-the-relay drill, §7 of the architecture
  lens). No data at risk — reachability SPOF only.
- **Sovereignty/trust:** German company, EU datacenters, you hold root; nobody but you (and a
  German court order) can see metadata or cut the box. GDPR: relay metadata (IPs = personal data)
  stays in the EU under your own controllership with Hetzner as infrastructure provider —
  the cleanest possible posture, and equally clean under **Albania's Law 124/2024** (GDPR-aligned
  national DPL, in force Feb 2025, enforcement actively ramping in 2026 — VERIFIED — IAPP/KPMG/
  DLA Piper coverage).
- **Lock-in:** none. Any €5 VPS anywhere runs the same config; the public name is your domain.

**Tailscale (Funnel for the public leg; DERP/tailnet for device-to-device):**

- **Price (2026-07):** Personal plan **free** — 6 users, unlimited user devices, 50 tagged
  resources, 1,000 ephemeral resource-minutes/mo; Standard $8/user/mo; Premium $18/user/mo
  (April 2026 seat-based pricing overhaul — VERIFIED — tailscale.com/pricing + pricing-v4 blog).
  Funnel is available **on all plans including free** (VERIFIED — Funnel docs).
- **TLS model — the decisive verification:** Funnel does **NOT** decrypt. TLS terminates on your
  node's `tailscaled`; Funnel ingress is a dumb TCP proxy; *"Tailscale cannot access or read any
  content"* (VERIFIED, quoted above). DERP relays likewise *"blindly forward already-encrypted
  traffic"* — WireGuard keys never leave devices, so DERP **cannot** decrypt (VERIFIED —
  tailscale.com/docs/reference/derp-servers + kb/1504/encryption).
- **The residual trust asymmetry (don't skip this):** although Tailscale can't read traffic, it
  **controls the name and the keys' rendezvous**: the `ts.net` DNS zone, the LE cert issuance
  path for your node's public name, the proprietary coordination server that distributes public
  keys, and the DERP fleet. A coerced/compromised Tailscale could re-point your public name at a
  node it controls and mint a valid cert for it — the E2E guarantee protects *your* node's
  traffic, not the *name* customers dial (ESTIMATE — threat-model inference from verified facts:
  ts.net DNS control ⇒ DNS-01 issuance capability; coordination server is proprietary —
  VERIFIED — tailscale.com/opensource). With your own domain + registrar, that authority is yours.
  **Headscale** (open-source control plane; official clients connect unmodified; its lead
  maintainer is a Tailscale employee but the project is independent — VERIFIED — github.com/
  juanfont/headscale + tailscale.com/opensource) is a real exit ramp for the *tailnet*, but not
  for Funnel's public `ts.net` names.
- **Production-appropriateness of Funnel:** the docs themselves flag **beta status**,
  **non-configurable, unpublished bandwidth limits**, ports 443/8443/10000 only, `ts.net` names
  only, and LE rate-limit foot-guns (VERIFIED — Funnel docs). Third-party benchmarks put relay
  throughput at ~100 Mbps-class, and community guidance consistently frames Funnel as ephemeral
  sharing, not business-critical serving (VERIFIED-secondary — onidel.com/pangohost comparisons).
  For dowiz's ~KB-scale order payloads the bandwidth cap is irrelevant; the beta status, the
  unpublished limits, and above all the ts.net-only naming are what disqualify it as the
  **primary** front door.
- **Sovereignty/GDPR:** Tailscale Inc. is a **Canadian** company (Toronto; ~$275M raised,
  Series C — VERIFIED-secondary — CBInsights/PitchBook/Craft), not US as commonly assumed —
  but its DPA relies on **SCCs for EU→US transfers** (VERIFIED — tailscale.com/dpa), i.e.
  coordination metadata does transit US infrastructure. Coordination server stores metadata +
  public keys only, never private keys or payload (VERIFIED — tailscale.com/security). Net:
  legally workable, materially more third-party exposure than a self-held EU box.
- **DERP geography:** nearest DERP regions to Albania are Frankfurt/Nuremberg, Warsaw, Madrid,
  Paris, Amsterdam — **no Balkan DERP** (VERIFIED — derp-servers doc). Relayed traffic from
  Durrës hairpins through central Europe — the same ~35-60 ms ballpark as dialing a Hetzner
  Falkenstein box directly (ESTIMATE; no Albania-specific latency data found).
- **Lock-in:** the `ts.net` name in anything printed/bookmarked; the proprietary coordination
  plane (Headscale mitigates for tailnet, not Funnel); per-seat pricing if the fleet outgrows
  the free tier.

**Albania reachability (both candidates):** no evidence was found of Hetzner IP ranges, `ts.net`,
or Tailscale endpoints being blocked or degraded in Albania (UNVERIFIED — absence of evidence
after search, not proof). The one national precedent cuts both ways: Albania **did** block an
entire platform (TikTok, Mar 2025 → Feb 2026) at national level — proof the state can and will
order ISP-level blocks — but the Constitutional Court ruled on 2026-03-11 that the ban violated
freedom of expression (VERIFIED — Balkan Insight / France24 / OBCT), establishing judicial
recourse. A no-name Hetzner IP fronting a restaurant ordering page is about the least likely
blocking target imaginable; a large foreign platform domain (`ts.net`) is categorically more
exposed to collateral blocking than your own `.al`-branded domain (ESTIMATE — judgment).
Albania's IPv6 sits at ~39% (VERIFIED-in-repo-doc §4.4's table), so CGNAT IPv4 remains the
planning assumption; both candidates handle it identically (that's their whole job).

### 1.2 Decision table

| Axis | Hetzner self-hosted relay (CX23) | Tailscale Funnel / DERP | Winner |
|---|---|---|---|
| Price | ~€5.49-5.99/mo + €0.50 IPv4, 20 TB incl. (VERIFIED-sec.) | €0 (free plan incl. Funnel) (VERIFIED) | Tailscale on sticker price; **tie in practice** — €6/mo is below materiality |
| "Dumb relay never decrypts" | ✅ SNI passthrough, TLS on vendor node (VERIFIED) | ✅ **Funnel does not decrypt; TLS on node** (VERIFIED) | **Tie — the premise that this axis decides is dead** |
| Naming authority / QR permanence | Your domain, your registrar, LE DNS-01 on node | `ts.net` only; Tailscale controls DNS + cert path | **Hetzner, decisively** (printed QR = physical lock-in) |
| Production posture | 99.9% SLA w/ credits; free DDoS mitigation (VERIFIED) | Funnel in **beta**, unpublished non-configurable bandwidth caps (VERIFIED) | **Hetzner** |
| Who can cut/see traffic | You + German legal process; metadata stays EU | Tailscale (CA co., EU→US SCCs) can suspend acct/re-point name; can't read payload | **Hetzner** |
| Albania blocking exposure | Own low-profile IP + `.al`-branded domain | Big-platform domain, foreign coordination plane | **Hetzner** (marginal; no current blocking either way — UNVERIFIED) |
| Ops burden | ~1-2 h setup, ~0 steady-state; you patch one stateless box (ESTIMATE) | Zero — fully managed | **Tailscale** |
| Failure modes | Reachability SPOF; DNS-failover to a clone in minutes; no data loss | Tailscale outage/policy change; no self-heal path you control | **Hetzner** (failure is *recoverable by you*) |
| GDPR / Law 124/2024 | Cleanest: EU processor, self-controlled | Workable: DPA + SCCs, metadata crosses Atlantic | **Hetzner** |
| Lock-in | None (any VPS, portable config) | ts.net names, proprietary control plane (Headscale partial exit) | **Hetzner** |
| Dev/ephemeral convenience | Needs the box + DNS first | `tailscale funnel 443` in one command | **Tailscale** |

### 1.3 Recommendation (hybrid, with sharp roles)

**Primary: Hetzner CX23 (Falkenstein or Nuremberg), nginx `stream` SNI-passthrough, your own
domain, TLS terminating on the vendor node. ~€6/mo all-in.** It wins every sovereignty,
production, naming, and jurisdiction axis; the only axes Tailscale wins (price, zero-ops) are
worth less than €6/mo and 2 hours.

**Tailscale's three legitimate, non-primary roles:**
1. **Dev/demo convenience** — Funnel is the fastest way to show a vendor node to someone before
   the domain exists. Use freely pre-launch.
2. **Ops/admin plane** — a free 6-user tailnet gives the operator NAT-free SSH/admin into vendor
   boxes with zero port exposure. This is Tailscale's actual product and it is excellent at it.
   (Headscale later if sovereignty of the admin plane starts to matter.)
3. **Break-glass fallback — for the courier/admin legs only.** Critical caveat the "Funnel as
   fallback" idea misses: **Funnel cannot back up the customer leg**, because the customer leg's
   URL is baked into printed QR codes on your domain — a fallback that changes the URL to
   `ts.net` is not a fallback. Customer-leg HA = a **second €6 VPS + low-TTL DNS failover**
   (defer until real volume; the vendor node keeps working offline regardless).

Reject (re-confirming the architecture lens with fresh checks): **Cloudflare Tunnel** (terminates
TLS at edge — violates the red-line; VERIFIED-in-repo-doc §4.4), **ngrok free** (interstitial
kills one-shot conversion), **DDNS/port-forward** (dead under CGNAT).

---

## 2. The mesh / P2P-overlay assessment ("mesh/ic2p", each candidate honestly)

The frame for every verdict: **this system has no discovery problem and no anonymity
requirement.** The vendor knows its couriers by name (vendor-employed, enrolled in person); the
customer arrives by scanning a QR that *is* the address; fiscalization (Law 87/2019, NIVF codes)
makes the money trail legally attributable **by design**. Overlays earn a place only by solving
reachability or sync — never discovery, never anonymity.

### (a) iroh — vendor↔courier transport. VERDICT: KEEP, confirmed; self-host the relay on the same Hetzner box.

- 2026 state re-confirmed: **v1.0 shipped 2026-06-15** ("Dial keys, not IPs"), v1.0.2 current;
  wire + API stability commitment; ~200M endpoints created in 30 days (VERIFIED-in-repo-doc
  §4.2, re-checked against iroh.computer + release coverage this session). Relays are stateless
  E2E-encrypted forwarders — the dumb-relay property native (VERIFIED — docs.iroh.computer/
  concepts/relays). Ed25519 NodeId dialing aligns 1:1 with bebop's self-cert identity
  (VERIFIED-in-repo-doc, C-lens §2.2/§3.2). Official Swift/Kotlin bindings for the courier app.
- **n0's public relays are dev/test only ("no uptime guarantees"); hosted n0des is $0 dev /
  $19/mo Pro** (VERIFIED — iroh.computer/pricing). $19/mo for a managed relay vs €0 marginal on
  the Hetzner box you already run: **self-host `iroh-relay`** (open-source crate/binary, TOML
  config, allowlist/denylist + bearer-token access control, LE built in — VERIFIED —
  docs.iroh.computer + crates.io/iroh-relay) **on the same CX23**. One €6 box = SNI passthrough
  (customer leg) + iroh relay (courier leg). Token-gate it so it serves only your endpoints.
- Honest limits: browser iroh is relay-only-over-WebSocket (so still wrong for the customer
  leg — plain WSS is lighter for one order); hole-punch success has no official number (~70%
  ±7 is the best measured DCUtR analogue — VERIFIED-in-repo-doc), which is exactly why the relay
  fallback exists. Same-premises vendor↔courier on shop Wi-Fi gets **direct QUIC on the LAN for
  free** — no extra mesh layer needed for the same-premises case.

### (b) libp2p / IPFS — VERDICT: REJECT for MVP; the "poetry" call was right.

- **libp2p for presence/discovery:** rust-libp2p is 0.56.0 with **no stable release in >12
  months** (VERIFIED-in-repo-doc §4.3, re-checked); browser story broken on Safari (no
  `serverCertificateHashes` — WebKit stated it does not intend to implement, VERIFIED-in-repo-doc
  §4.1); `libp2p-webrtc` server still alpha; and running it means operating bootstrap +
  rendezvous + circuit-relay infra — **strictly more servers than the dumb relay it would
  replace**. Gossipsub is engineered for Filecoin/Ethereum-scale meshes of hundreds-to-thousands
  of always-on nodes with amplification-control machinery (VERIFIED — libp2p gossipsub v1.1
  spec); dowiz's "presence" set is **one vendor node + a handful of vendor-employed couriers on
  push-woken phones** — a roster, not a network. A DHT on a phone fights Doze/OEM killers and
  drains battery for zero benefit when every peer is already known and reachable via
  node/relay/push. Presence = a heartbeat row over the existing iroh/WSS connection. Done.
- **IPFS content-addressing for menus:** real technology, fantasy fit. Public gateways are
  documented by IPFS's own material as unreliable for production and slow when providers are
  NAT'd or absent; content vanishes unless pinned; the practical advice is "run your own gateway
  or pay a pinning service" (VERIFIED — blog.ipfs.tech gateway explainer, ipfs/kubo#6383,
  Cloudflare web3 troubleshooting docs) — i.e. **a server with extra steps**, plus IPNS's
  notoriously slow mutable-pointer problem for data that changes daily (UNVERIFIED — widely
  reported, not benchmarked this session). A menu is a few KB of **single-writer vendor data**
  already served CA-TLS from the node through the relay, cache-friendly by ETag. The legitimate
  kernel of the idea — content-hash addressing so tampered data is rejected — **already exists
  inside bebop2's canonical-bytes + content-hash envelopes** (VERIFIED-in-repo-doc) without any
  global network. Keep the hash, skip the planet-scale DHT.

### (c) I2P (reading "ic2p" as the anonymity overlay) — VERDICT: REJECT categorically; anonymity is an anti-requirement here.

- Measured reality: typical request RTT **1-3 seconds**; per-tunnel throughput capped around
  **20-50 KB/s**; performance varies with volunteer-router quality by construction (VERIFIED —
  i2p.net official performance docs, quoted via search; ~55k volunteer routers). Garlic routing
  buys sender/receiver unlinkability by paying in latency, jitter, and third-party reliability —
  the exact three currencies a live dispatch loop cannot spend.
- It doesn't just fail on performance — it fails on **purpose**. dowiz's trust model is built on
  *attribution*: signed capabilities, counter-signed custody hand-offs, courier PoDs bound to
  `(order, courier, ts, loc)`, vendor-employed couriers, and a legally mandated fiscal trail
  (NIVF). An anonymity overlay would actively degrade dispute resolution and fiscal compliance
  while making every order slower. There is no leg — customer, courier, or vendor — where
  anonymous routing helps. If the concern behind "i2p" is *privacy from the relay*, that is
  already solved: the relay sees ciphertext + SNI only, by construction, on both shortlisted
  candidates.

### (d) The other meshes — one paragraph each

- **Hyperswarm / Holepunch (Pear):** alive and actively developed into 2026 (repos updated
  May 2026 — VERIFIED-secondary — github.com/holepunchto), DHT + hole-punching with Noise E2E
  streams. But: JS-centric runtime (conflicts with the Rust kernel direction), no published
  hole-punch success rates, no story for the browser customer or for waking locked phones — it
  solves precisely the problem iroh already solves, with worse language fit and weaker relay
  self-hosting ergonomics. **REJECT.**
- **Nebula (Slack-lineage, Defined Networking):** solid self-hosted L3 overlay — cheap lighthouse
  node, cert-based identity, mobile apps exist (VERIFIED — github.com/slackhq/nebula +
  nebula.defined.net). But it is a **fleet VPN for machines you administer**, not an app
  transport: couriers' personal phones would need an always-on VPN profile (battery, the single
  Android VPN slot, ops overhead), and it does nothing for the one-shot browser customer.
  Its only plausible role — operator admin plane — is already covered better by the free
  Tailscale tailnet (or Headscale later). **REJECT for product traffic.**
- **Netmaker / WireGuard-mesh:** same category as Nebula — self-hosted WireGuard automation,
  freemium with a `pro/`-directory proprietary license split (VERIFIED — github.com/gravitl/
  netmaker). Admin-plane tooling, not a product transport. **REJECT for product traffic.**
- **Tailscale-as-mesh (the tailnet proper, not Funnel):** the one overlay that earns a (non-product)
  place — free 6-user ops plane for the operator to reach vendor boxes NAT-free. **ACCEPT as ops
  tooling only.** Product traffic stays on relay+iroh where identity is dowiz's own keys, not a
  tailnet ACL.
- **Zenoh (same-premises LAN multicast):** already nominated by lens C for the vendor+courier
  in-shop case; 1.x stable, but no NAT story and no official iOS binding (VERIFIED-in-repo-doc
  §2.2). Since iroh already gives direct LAN QUIC between co-located peers, Zenoh adds a second
  transport for a case that is already covered. **DEFER indefinitely** — revisit only if
  multi-device vendor kiosks need sub-ms local pub/sub.

### Where mesh genuinely helps vs over-engineering (the crisp line)

- **Genuinely helps:** vendor↔courier when both apps are installed and either co-located (LAN
  direct QUIC) or cross-city (hole-punch + owned relay fallback) — that is iroh, already chosen,
  now confirmed. Multi-device vendor (tablet + kitchen screen) is the same LAN case.
- **Over-engineering:** anything DHT-shaped, gossip-at-scale, content-routed, or anonymous for a
  three-party roster where every peer is known, employed, or QR-directed. **The customer leg
  especially: one shot, one browser, one signed intent — plain WSS through the dumb relay wins
  on every axis** (weight, latency, Safari compat, battery, and auditability).

---

## 3. The honest synthesis — minimal, cheapest, most-sovereign stack for all three legs

### 3.1 The stack (one box, one domain, three legs)

| Leg | Transport | Trust/TLS | Infra it touches | Marginal cost |
|---|---|---|---|---|
| **Customer (one-shot mobile web)** | HTTPS/WSS → `<venue>.order.<domain>.al` → DNS A → **Hetzner CX23 nginx `stream` SNI-passthrough** → vendor node | CA cert (LE DNS-01) held **on the vendor node**; relay forwards ciphertext | The €6 box | €0 beyond the box |
| **Courier (installed app)** | **iroh** — direct QUIC (LAN/hole-punch), fallback **self-hosted `iroh-relay`, token-gated, on the same CX23**; FCM (or self-hosted ntfy) wake-nudge for locked phones | E2E to node keys (Ed25519 NodeId = bebop self-cert); relay stateless | Same box + push gateway (the one unavoidable Big-Tech door) | €0 beyond the box |
| **Vendor (sovereign node)** | terminates TLS; single writer; `kernel::decide`; fiscal queue node-only | holds its own keys, log, and cert | none (its own device) | €0 |
| **Ops plane (operator only)** | free Tailscale tailnet (≤6 users) for NAT-free admin; Funnel for pre-launch demos | E2E WireGuard; metadata to Tailscale (accepted for admin only) | none | €0 |

**Total recurring infra: ≈ €5.99-6.49/mo** (CX23 + IPv4; VERIFIED-secondary pricing) — one box
carrying both relays, versus $19/mo for n0des Pro alone or €0-but-your-front-door for Funnel.
The COD posture makes this stack complete: **no payment gateway, no webhook receiver, no PCI
surface exists anywhere** — the relay only ever carries ciphertext of signed *bookkeeping*
(obligations settled by cash at the door), so the cheapest box on the menu is genuinely
sufficient, and the "serverless money" claim stays literally true.

Sovereignty summary: the only parties who can interrupt service are Hetzner (your contract,
German/EU law, 99.9% SLA) and — for courier wake-ups only — Google/Apple push. Nobody outside
the vendor's device can read an order. The naming authority (domain) is dowiz's own. That is the
maximum sovereignty available at any price without violating the reachability physics ratified
in `02-local-first-architecture.md §4`.

### 3.2 Build first (ordered), and what to defer

**Build now:**
1. **Domain + Hetzner CX23 + nginx `stream` SNI-passthrough + LE DNS-01 on the vendor node.**
   ~1 session. Zero dependency on bebop2 maturity; makes the customer leg real end-to-end and
   is prerequisite plumbing for venue #1's QR. Falsifier (VbM): packet capture on the relay
   shows any plaintext ⇒ RED (matches the architecture lens's falsification (c)).
2. **Courier push-wake (FCM via thin native wrapper, or ntfy).** The hub review's #1 product gap;
   independent of transport choice; without it no P2P layer matters because the phone is asleep.
3. **iroh behind `MeshTransport` + self-hosted token-gated `iroh-relay` on the same box** for
   vendor↔courier. Falsifier: kill-the-relay drill — in-flight order completes on LAN ⇒ GREEN.

**Defer (cheap to add later, wrong to build now):**
- Second relay + low-TTL DNS failover (when real order volume exists — until then the €6 SPOF
  is proportionate; the vendor node is not data-dependent on it).
- Headscale (only if the admin plane's Tailscale dependence starts to bind).
- iroh-gossip rendezvous for multi-vendor federation (gated behind the ratified
  one-real-order/G11 trigger, per SYNTHESIS §5-6).
- Zenoh LAN pub/sub (only if multi-device vendor kiosks demand it).

**Never (on current requirements):** libp2p/IPFS for menus or presence; I2P or any anonymity
overlay; Nebula/Netmaker as product transport; Cloudflare Tunnel; printing a `ts.net` URL on
anything a customer can scan.

---

## Sources (load-bearing, fetched/confirmed 2026-07-11 unless noted)

- Tailscale Funnel TLS/limits/beta: https://tailscale.com/docs/features/tailscale-funnel (fetched)
- Tailscale DERP no-decrypt + regions: https://tailscale.com/docs/reference/derp-servers (fetched); https://tailscale.com/kb/1504/encryption; https://tailscale.com/security
- Tailscale pricing v4 (Apr 2026, free 6-user/Standard $8/Premium $18): https://tailscale.com/pricing; https://tailscale.com/blog/pricing-v4
- Tailscale DPA (SCCs, EU→US): https://tailscale.com/dpa; open-source boundaries: https://tailscale.com/opensource; Headscale: https://github.com/juanfont/headscale
- Tailscale Inc., Toronto, ~$275M raised: https://www.cbinsights.com/company/tailscale; https://pitchbook.com/profiles/company/268781-05; https://craft.co/tailscale/locations
- Hetzner price adjustment 15-06-2026 (CAX11 €4.49→€5.99; CPX22 €7.99→€19.49; CCX13 €15.99→€42.99): https://docs.hetzner.com/general/infrastructure-and-availability/price-adjustment/ (fetched)
- Hetzner July-2026 plan snapshot (CX23 €5.49, 20 TB, €1/TB overage, IPv4 €0.50): https://costgoat.com/pricing/hetzner (fetched); corroborating: https://comparedge.com/tools/hetzner/pricing; https://bestusavps.com/reviews/hetzner/
- Hetzner Cloud SLA 99.9% + credits (updated 2026-03-16): https://docs.hetzner.com/general/company-and-policy/slas-cloud/ (fetched); DDoS included: https://www.hetzner.com/unternehmen/ddos-schutz
- iroh relays/self-hosting/pricing: https://docs.iroh.computer/concepts/relays; https://www.iroh.computer/pricing; https://crates.io/crates/iroh-relay; v1.0: https://www.iroh.computer/blog (per C-lens, re-checked)
- IPFS gateway reliability: https://blog.ipfs.tech/2022-06-30-practical-explainer-ipfs-gateways-2/; https://github.com/ipfs/kubo/issues/6383; https://developers.cloudflare.com/distributed-web/ipfs-gateway/troubleshooting/
- I2P performance (1-3 s RTT, 20-50 KB/s per tunnel): https://i2p.net/en/docs/overview/performance/
- libp2p gossipsub spec/scale posture: https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.1.md
- Hyperswarm/Holepunch activity: https://github.com/holepunchto/hyperswarm; https://docs.pears.com
- Nebula: https://github.com/slackhq/nebula; https://nebula.defined.net/docs/; Netmaker: https://github.com/gravitl/netmaker
- Albania TikTok ban + Constitutional Court 2026-03-11: https://balkaninsight.com/2026/03/11/albanias-tiktok-ban-violated-free-expression-court-rules/bi/; https://www.france24.com/en/live-news/20260311-albania-tiktok-ban-violated-free-speech-court-rules; https://www.balcanicaucaso.org/en/cp_article/tiktok-in-albania-ban-ends/
- Albania Law 124/2024 (GDPR-aligned, in force Feb 2025): https://iapp.org/news/a/albania-s-personal-data-protection-law-a-legal-framework-harmonized-with-the-gdpr; https://kpmg.com/al/en/insights/2025/02/new-law-on--personal-data-protection-.html; https://www.dlapiperdataprotection.com/?t=law&c=AL
- Repo (read-only): `docs/design/local-first-hub-2026-07-11/{SYNTHESIS.md,02-local-first-architecture.md,C-runtime-transport-identity.md}`

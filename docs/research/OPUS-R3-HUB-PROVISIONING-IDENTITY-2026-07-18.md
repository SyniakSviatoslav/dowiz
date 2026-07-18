# OPUS-R3 — Hub Provisioning, Identity & Crypto-Agility Research

> **Scope:** Wave-0 technical grounding for the claim-mechanic + capability-cert + tunnel/host
> provisioning surface of dowiz/DeliveryOS. Feeds the P39/P48 Tier-3 blueprint work.
> **Provenance:** binding decisions in `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`
> §16.1–16.2, §16.12, §16.32, §16.45, §16.48, §16.54, §16.57, §17.2, §17.3, §17.7, §17.8–17.9.
> **Method:** real-source research (Cloudflare/Hetzner/IETF/SPIFFE docs + Rust crate docs), not
> memory. Every load-bearing claim is cited inline. Date: 2026-07-18.

---

## 0. What is already decided (do not re-litigate)

From the roadmap dialogue passes, binding for this research:

- **Claim mechanic, not live provisioning** (§16.32): a *pool of pre-generated, fixture-populated
  demo hubs* (§16.54) exists ahead of time; a vendor *claims* one (ownership assignment) and uses
  it instantly. No boot-from-scratch wait on the critical path. Abandoned claims are never
  reclaimed (§16.57) → the pool *depletes*, so the supply pipeline must be a background refill, not
  a recycle loop.
- **One dowiz CF account fronts all hubs for Wave-0** (§16.45), dowiz owns tenant-isolation between
  tunnels/routes/credentials on that account.
- **Owner multi-hub credential = a root/delegating capability-cert** (§16.48) the owner holds
  themselves; it can add/modify/revoke child hub certs. Built on the existing **ML-DSA-65⊕Ed25519
  hybrid** signer, extended one delegation level.
- **Crypto-agility from Wave-0** (§17.2): versioned capability-certs, algorithm-migration path with
  no hard fork of the mesh.
- **Four forever-dependency escape hatches** are *ports with Wave-0 defaults, not hardcoded*:
  dowiz-CF-account tunnel → vendor's own CF/other provider (§17.3); dowiz-signed cert root → hub is
  its own self-signed root, dowiz co-sign optional (§17.7); Cloudflare-the-company → swappable
  tunnel port, WireGuard/other relay substitutable (§17.8); Hetzner → swappable VPS port (§17.9).
- **Licensing/open-source boundary** (§16.54): hub software is AGPLv3+TM+DCO open; dowiz's *own*
  claim-mechanic / CF-isolation / landing infra stays closed.

The research below turns these shapes into concrete, sourced mechanisms.

---

## 1. Cloudflare Tunnel automation & multi-tenant isolation

### 1.1 Remotely-managed tunnels are the right primitive

Cloudflare distinguishes *locally-managed* (config lives in a YAML file on the host) from
*remotely-managed* tunnels. **A remotely-managed tunnel requires only a tunnel token to run** — an
opaque `eyJ…` JWT string — and all ingress/routing config lives in Cloudflare's control plane,
editable via API. This is exactly the shape dowiz needs: the hub binary ships `cloudflared`, is
handed a token at claim-time, and dowiz configures routing centrally without ever touching the
host. ([Cloudflare — remote tunnel permissions](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/remote-tunnel-permissions/), [Create a tunnel (API)](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel-api/))

### 1.2 The concrete API flow (per hub)

Fully automatable, unattended, no dashboard step:

1. **Create tunnel** — `POST /accounts/{account_id}/cfd_tunnel` with
   `{ "name": "hub-<id>", "config_src": "cloudflare" }` (`config_src: cloudflare` = remotely
   managed). Returns `tunnel_id`.
2. **Fetch token** — `GET /accounts/{account_id}/cfd_tunnel/{tunnel_id}/token` → the `eyJ…` string
   handed to the hub.
3. **Configure ingress remotely** — `PUT /accounts/{account_id}/cfd_tunnel/{tunnel_id}/configurations`
   with an `ingress` array mapping `hostname → service` (e.g. `hub-<id>.hubs.dowiz.org →
   http://localhost:8080`, terminal catch-all `http_status:404`).
4. **DNS route** — `POST /zones/{zone_id}/dns_records` with `type: CNAME`, `content:
   <tunnel_id>.cfargotunnel.com`, `proxied: true`.
5. **Host runs** — `cloudflared service install <token>` (or a systemd/container unit). No local
   config.

([Create a tunnel (API)](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel-api/), [Cloudflare API — Tunnels](https://developers.cloudflare.com/api/resources/zero_trust/subresources/tunnels/))

### 1.3 Terraform provider (v5) — the declarative path

The Cloudflare Terraform provider exposes the same surface as first-class resources — useful for the
*pool-refill* pipeline (not the per-claim hot path):
- `cloudflare_zero_trust_tunnel_cloudflared` (the tunnel; renamed from `cloudflare_tunnel` in the v5
  provider),
- `cloudflare_zero_trust_tunnel_cloudflared_config` (ingress rules),
- `cloudflare_zero_trust_tunnel_cloudflared_token` (data source to read the token into the host's
  provisioning),
- `cloudflare_dns_record` (the CNAME).

([Deploy Tunnels with Terraform](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/deployment-guides/terraform/), [Automating Cloudflare Tunnel with Terraform — CF blog](https://blog.cloudflare.com/automating-cloudflare-tunnel-with-terraform/), [registry: zero_trust_tunnel_cloudflared](https://registry.terraform.io/providers/cloudflare/cloudflare/latest/docs/resources/zero_trust_tunnel_cloudflared))

### 1.4 Hard account limits (the §16.45 "revisit if hub count grows" number, quantified)

Per Cloudflare One account limits ([source](https://developers.cloudflare.com/cloudflare-one/account-limits/)):

| Limit | Value | Consequence for dowiz |
|---|---|---|
| cloudflared **tunnels / account** | **1,000** | **Hard ceiling on hubs per CF account at Wave-0.** One tunnel per hub → ≤1,000 hubs before a second account or Enterprise raise is mandatory. This is *the* scaling wall §16.45 flagged. |
| **Routes / account** | 1,000 (shared w/ CF Mesh) | If each hub needs one hostname route, this co-caps at ~1,000. |
| Replicas / tunnel | 25 | Ample for HA per hub. |
| Virtual networks / account | 1,000 | Not a near-term constraint. |

**Planning takeaway:** the single-account Wave-0 design (§16.45) is validated but has a *concrete
1,000-hub cliff*. Design the CF-account handle as itself a swappable/shardable parameter (account
pool) from day one so hitting the cliff is a config change, not a re-architecture.

### 1.5 What "tenant isolation on one account" actually is — and is NOT

This is the sharpest caveat for the blueprint. On a single CF account:
- **Data-plane isolation is real and per-tunnel:** each hub gets its own tunnel credential (token),
  its own ingress config, and its own DNS hostname. A hub's token authorizes only *its* tunnel; a
  leaked hub token cannot impersonate another hub's tunnel. Cloudflare recommends **rotating tunnel
  tokens on a cadence**, and rotation is graceful with ≥2 replicas. ([remote tunnel permissions](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/remote-tunnel-permissions/))
- **Control-plane isolation is NOT real:** the *dowiz operator API token* that creates/edits tunnels
  can see and modify **every** tunnel on the account. There is no per-tenant CF sub-account boundary
  in this model. So "tenant isolation" here means **dowiz's own provisioning service is the trust
  boundary** — a compromise of that service is a compromise of all hubs' tunnel config (not their
  application data or capability-cert keys, which never touch CF). The blueprint must treat the
  provisioning service's API-token custody as a top-tier secret and log all tunnel-config mutations.
- The §17.3/§17.8 escape hatch (switch tunnel target to the vendor's own CF account or a WireGuard
  relay) is what removes even *this* residual dowiz dependency for a vendor who wants it — see §5.

---

## 2. Certificate hierarchy: delegating/root capability-cert (§16.48)

The question the blueprint must answer: **adopt SPIFFE/SPIRE, wrap `rcgen`, or build on the existing
hybrid signer directly?** Research says: **borrow SPIFFE's *model*, reject SPIRE's *runtime*, and
implement the delegation semantics with a biscuit-style signed-block capability token over the
existing ML-DSA-65⊕Ed25519 signer — not X.509.**

### 2.1 SPIFFE/SPIRE — right concepts, wrong runtime for dowiz

SPIFFE (the spec) / SPIRE (the Go implementation) is the mainstream prior art for *exactly* this
problem — workload identity with a flexible root of trust and cross-org federation:
- A **trust domain** is the root of trust; every identity (`spiffe://…`) is verifiable against the
  domain's root keys. This maps 1:1 to §17.7's "each hub is its own self-signed root." ([SPIFFE concepts](https://spiffe.io/docs/latest/spiffe-about/spiffe-concepts/))
- The **SPIRE server is the CA**: self-signed by default, *or* it delegates signing to an external
  CA via an **UpstreamAuthority plugin** (e.g. `aws_pca`). This is the "dowiz optionally co-signs the
  root" pattern (§17.7) in existing-product form. ([SPIRE concepts](https://spiffe.io/docs/latest/spire-about/spire-concepts/), [Configuring SPIRE](https://spiffe.io/docs/latest/deploying/configuring/))
- **Federation = exchange trust bundles** (sets of root public keys) between domains, so IDs issued
  by one hub can be verified by another *without* a shared central authority. This is precisely the
  primitive dowiz's optional future federation phase (§17.4) would need, and the survivability model
  (§17.3) — trust survives because bundles are self-contained. ([SPIFFE — federation](https://spiffe.io/docs/latest/spire-about/spire-concepts/))

**Why NOT adopt SPIRE the runtime:**
1. **No ML-DSA / no PQC.** SPIRE's `ca_key_type` supports only `ec-p256` (default), `ec-p384`,
   `rsa-2048`, `rsa-4096` — classical only. PQC is an *open, unshipped* effort ([SPIRE issue #6975
   "SPIRE support for Post-Quantum Cryptography"](https://github.com/spiffe/spire/issues/6975)),
   documented as "not production-ready for certificates yet." dowiz's §17.2 mandate is PQC *from
   Wave-0*; SPIRE cannot express dowiz's mandated hybrid today. ([SPIRE server config](https://spiffe.io/docs/latest/deploying/spire_server/))
2. **Go, node-attestation, and K8s-centric.** SPIRE is a heavyweight Go daemon oriented to
   datacenter workload attestation — antithetical to dowiz's Rust-native, no-daemon-sprawl,
   isolated-hub ethos.
3. **X.509-SVID is an X.509 profile.** X.509-SVIDs put the identity in a URI SAN but inherit X.509's
   signature-algorithm negotiation, which has no clean slot for a *capability/attenuation* graph —
   dowiz's §16.48 need is delegation-with-narrowing, not just identity. ([X509-SVID spec](https://spiffe.io/docs/latest/spiffe-specs/x509-svid/))

**Verdict:** adopt SPIFFE's *vocabulary and topology* — trust domain = hub root, SVID-style stable
IDs, trust-bundle federation, optional upstream co-sign — as the conceptual frame in the blueprint.
Do **not** take a SPIRE dependency. This is a documented-decision, not a hand-wave: the roadmap's own
Rust-native + PQC-from-day-one constraints are individually disqualifying.

### 2.2 `rcgen` — usable for X.509 plumbing, wrong layer for the capability graph

`rcgen` (rustls project) is the production Rust crate for generating X.509 certs/CSRs and **can sign
child certs**: an `Issuer` signs a subject cert, populating the issuer field and Authority Key
Identifier from the issuer's key. ([rcgen docs](https://docs.rs/rcgen/latest/rcgen/), [rcgen Issuer](https://docs.rs/rcgen/latest/rcgen/struct.Issuer.html)) Two cautions from real history:
- AKI/chain bugs were live and only fixed in **0.13.1** (correct issuer key-identifier computation);
  hierarchy handling has been "an area of active bug-fixing." ([rcgen CHANGELOG](https://docs.rs/crate/rcgen/latest/source/CHANGELOG.md), [issue #261](https://github.com/rustls/rcgen/issues/261))
- **`rcgen` performs no validation that the issuer is a CA / has signing key-usage** — it will
  cheerfully sign with a non-CA key. Any CA-constraint enforcement is dowiz's responsibility.
- **No ML-DSA.** `rcgen` signs with classical algorithms; it cannot natively emit dowiz's hybrid.

So `rcgen` is a fine tool *if* dowiz decides the wire format is X.509 (e.g. for TLS termination
interop), but it does not solve the §16.48 *delegation/attenuation* requirement and does not carry
the PQ half. It is plumbing, not the design.

### 2.3 biscuit-auth — the closest match to §16.48's "root that mints/attenuates children"

`biscuit-auth` (Eclipse project, Rust) is a **public-key capability token** with the exact semantic
§16.48 asks for:
- **Offline delegation by attenuation:** "a new, valid token can be created from another one by
  attenuating its rights, by its holder, without communicating with anyone." The owner's root
  credential can mint child hub-credentials *offline* — no dowiz round-trip — directly satisfying
  §16.48 + §17.3 survivability. ([biscuit-auth docs](https://docs.rs/biscuit-auth/latest/biscuit_auth/), [eclipse-biscuit/biscuit-rust](https://github.com/biscuit-auth/biscuit-rust))
- **Public-key, not shared-secret:** "any application holding the **root public key** can verify a
  token." Verifiers need only the root's *public* key — a hub can verify an owner-minted child cert
  knowing only the owner root's public key. This is strictly better than **macaroons**, whose
  HMAC-caveat model requires the verifier to hold the **root *secret***. ([biscuit vs macaroons](https://lib.rs/crates/biscuit-auth))
- **Signed-block chain + Datalog checks:** each attenuation appends a signed block carrying facts,
  rules, and checks — so "this child cert may operate hub-X only, may not re-delegate, expires T" is
  expressible *in the token* and verified offline. ([biscuit-auth](https://docs.rs/biscuit-auth/latest/biscuit_auth/))

Adjacent prior art worth citing for the "works with zero central infra" property: **Vouchsafe** — "A
Zero-Infrastructure Capability Graph Model for Offline Identity and Trust" ([arXiv 2601.02254](https://arxiv.org/pdf/2601.02254)) — confirms the offline-capability-graph pattern is a live research direction, not a dowiz idiosyncrasy.

**Caveat that keeps this bespoke:** stock biscuit-auth signs blocks with Ed25519 (and now some
secp256r1), **not ML-DSA-65**. dowiz cannot take biscuit-auth as-is under §17.2. But biscuit's
*construction* — an append-only chain of independently-signed blocks, each narrowing authority,
verified against a root public key — is the correct architecture to **re-implement over dowiz's
existing `HybridSigner` (ML-DSA-65⊕Ed25519)**. dowiz already owns the hybrid sign/verify primitive
(the P06 `key_V` HybridSigner closed 2026-07-18); the delegation layer is a thin, well-precedented
structure on top of it.

### 2.4 Recommended cert-hierarchy design (§16.48 + §17.7)

- **Identity object = a hybrid-signed capability certificate**, not X.509, structured as a
  biscuit-style append-only signed-block chain:
  - **Block 0 (root):** the hub's (or owner's) self-signed root — signed by its own ML-DSA-65⊕Ed25519
    keypair. This *is* the §17.7 self-sufficient root. No dowiz needed.
  - **Optional dowiz co-sign:** a *detached* dowiz signature over the root's public key, distributed
    as a convenience attestation (like a SPIFFE upstream-authority bundle entry). Its absence never
    invalidates the root — it only adds a second verifiable voucher for relying parties that trust
    dowiz. This is the §17.7 "dowiz may optionally sign, hub never needs it" property, made concrete.
  - **Child blocks (delegation):** the owner root appends a signed block per child hub, carrying
    Datalog-style scope facts (`hub_id`, `capabilities`, `may_delegate: false`, `expiry`). Adding a
    hub = append a block; modifying = append a superseding block; **revoking** = see §2.5.
- **Verification** is offline against the relevant root public key — a hub, a courier device, or the
  customer wallet can all verify an owner-minted credential with no server.

### 2.5 Revocation — the genuinely hard part (flagged, not hand-waved)

Capability tokens/certs are notoriously bad at *revocation* because they're designed to verify
offline without a central authority — the same property that gives §17.3 survivability fights against
timely revocation. Options, in preference order for dowiz:
1. **Short expiry + re-mint (SPIFFE's own answer):** SVIDs are deliberately short-lived; "revocation"
   is just non-renewal. Cheap, offline-friendly, no CRL. Best default; sets an upper bound on
   compromise window equal to the TTL.
2. **Owner-published revocation list**, gossiped hub-to-hub (a small signed "revoked child IDs" blob
   the owner root signs and pushes to their own hubs). Works within one owner's fleet without dowiz.
3. **dowiz optional revocation-transparency feed** (like the CVE-advisory stance in §17.7) — a
   convenience, never load-bearing.
   Recommend **(1) as the mechanism**, **(2) for immediate owner-driven revoke**, **(3) optional**.

---

## 3. Crypto-agility: the versioned capability-cert scheme (§17.2)

### 3.1 The single most useful finding: dowiz's exact hybrid is already a standardized OID

The IETF LAMPS **Composite ML-DSA** draft (`draft-ietf-lamps-pq-composite-sigs`, v19, active through
Oct 2026) standardizes composite signatures where an ML-DSA variant is paired with a classical
algorithm, **each combination getting a distinct OID**. Critically:

- **`id-MLDSA65-Ed25519-SHA512` — OID `1.3.6.1.5.5.7.6.48`** — is **dowiz's exact ML-DSA-65⊕Ed25519
  hybrid, already assigned a standard identifier.** Signature label `COMPSIG-MLDSA65-Ed25519-SHA512`,
  SHA-512 pre-hash. ([draft-ietf-lamps-pq-composite-sigs](https://datatracker.ietf.org/doc/draft-ietf-lamps-pq-composite-sigs/))
- **Verification semantics are AND:** "Valid signature … if and only if **all** component signatures
  were successfully validated." This matches the safe posture already adopted in dowiz's own crypto
  work (the B4 batch-verify walk-back: every accept must pass full single-verify). Both halves must
  verify — a break of *either* algorithm does not silently pass. ([same draft](https://datatracker.ietf.org/doc/draft-ietf-lamps-pq-composite-sigs/))
- **The versioning mechanism IS the OID/label:** "Each Composite ML-DSA algorithm has a unique
  signature label"; agility is achieved by protocols distinguishing composites via OID matching. So
  "add a new algorithm suite" = "register a new suite ID," not "fork the format."

**Consequence for §17.2:** dowiz should not invent a bespoke hybrid-versioning scheme. **Adopt an
algorithm-suite-identifier field** in the capability-cert (mirroring the composite-sigs OID/label
model — dowiz can use the actual OID `1.3.6.1.5.5.7.6.48` for its current suite, or a compact private
enum that maps to it). Suite v1 = `MLDSA65-Ed25519`. A future suite v2 = e.g. `MLDSA87-Ed448` or a
pure-PQ `MLDSA65-SLHDSA` is just a new suite ID; old and new certs coexist by ID. This is the
"versioned capability-cert, no hard fork" of §17.2 made concrete and standards-aligned.

### 3.2 How real fleets rotate algorithms without a hard fork (borrowable patterns)

- **TLS 1.3 negotiation:** client and server advertise supported algorithm lists (cipher suites /
  `supported_groups`), agree on the strongest mutually supported option; hybrid PQ integrates
  directly into `supported_groups`. → dowiz's inter-hub/agent handshake should carry a
  **suite-list**, negotiate the strongest common suite. ([PQC migration 2026](https://appscale.blog/en/blog/post-quantum-cryptography-migration-ml-kem-ml-dsa-hybrid-tls-2026), [PQ crypto-agility — Thales](https://cpl.thalesgroup.com/encryption/post-quantum-crypto-agility))
- **SSH graceful host-key rotation (the best fleet-rotation prior art):** OpenSSH 6.8+ ships
  `hostkeys@openssh.com` / `UpdateHostKeys` — a server **publishes all its host keys (including new
  algorithms)**; clients learn the new keys *while still trusting the old*; after an overlap window
  the deprecated key is removed and the new one becomes primary. There is deliberately an **overlap
  period** so no client is stranded. This is being formalized in `draft-ietf-sshm-hostkey-update`.
  ([djm — key rotation in OpenSSH 6.8+](https://blog.djm.net.au/2015/02/key-rotation-in-openssh-68.html), [draft-ietf-sshm-hostkey-update](https://datatracker.ietf.org/doc/draft-ietf-sshm-hostkey-update/)) → dowiz should copy the **overlap-rotation** shape: when migrating suites, a hub publishes a *new-suite* credential *alongside* its old-suite one for a defined window; verifiers learn the new root/suite; only then is the old suite retired. No flag-day, no mesh fork.
- **Dual-algorithm certificates for backward-compat transition** is an established academic/industry
  pattern ([Springer — Dual Algorithm Certificates in TLS](https://link.springer.com/chapter/10.1007/978-3-032-16089-8_30)), reinforcing that the "run two, phase one out" approach dowiz already uses for its hybrid is the mainstream migration primitive.

### 3.3 Recommended versioning scheme (concrete)

1. **Suite ID field** in every capability-cert block: `alg_suite: u16` (or the composite OID),
   `v1 = MLDSA65-Ed25519 (1.3.6.1.5.5.7.6.48)`.
2. **AND-verify** both components of the current suite (matches composite-sigs + dowiz B4 precedent).
3. **Suite negotiation** at every handshake: advertise supported suites, use the strongest common.
4. **Overlap rotation** (SSH-style) for fleet migration: dual-publish new+old suite credentials for a
   window `W`; retire old after `W`. `W` is a hub-local policy value (survives dowiz disappearing).
5. **Downgrade protection:** the highest suite a peer advertised must be bound into the transcript so
   an attacker can't force both sides down to a broken suite (the classic negotiation pitfall).

---

## 4. Hetzner provisioning: "claim triggers a ready instance" (§16.1 default, §16.9 port)

### 4.1 The two viable fast-assignment models

The roadmap's claim mechanic (§16.32) wants *no boot-from-scratch wait*. Two Hetzner-supported
approaches, both real:

- **(A) Golden-snapshot + provision-on-claim.** Build one fully-configured hub image with **Packer**,
  store it as an `hcloud` snapshot, and on claim create a server *from that snapshot* rather than a
  base OS. Terraform: a `data "hcloud_image"` selects the newest snapshot, `resource "hcloud_server"`
  sets `image = data.hcloud_image.<snap>.id`. Boot-from-snapshot is dramatically faster than
  install-from-scratch because all software/config is baked in. ([hcloud_server resource](https://registry.terraform.io/providers/hetznercloud/hcloud/latest/docs/resources/server), [hcloud_snapshot](https://registry.terraform.io/providers/hetznercloud/hcloud/latest/docs/resources/snapshot), [Using the Hetzner Cloud Terraform Provider — D. Hamann](https://davidhamann.de/2026/01/21/hetzner-cloud-terraform/))
- **(B) Warm pool.** Pre-create N running servers from the golden snapshot *ahead of demand* (the
  literal §16.32 "pre-generated demo hubs"); on claim, do **ownership assignment only** — flip the
  hub's owner record, hand over the capability-cert root and tunnel token, no API call to Hetzner on
  the hot path at all. This gives true zero-wait claims and matches §16.54's "pre-populated with
  fixtures." Cost: idle servers burn money, and §16.57's no-reclaim rule means the warm pool *only
  depletes* → the refill pipeline (option A, run in the background) must top it up.

**Recommended: (B) as the claim hot-path, (A) as the background refill.** Warm pool = instant claim;
snapshot-provision = how the pool gets refilled. This directly realizes §16.32 ("assignment, not
provisioning") + §16.57 (net-consumption pool).

### 4.2 Constraints to bake into the blueprint

- **`server_type` and `location` must match the snapshot's** build parameters. ([D. Hamann](https://davidhamann.de/2026/01/21/hetzner-cloud-terraform/)) Standardize one server_type + one primary location per pool; multi-region = multiple pools.
- **API token is per-project**; the pool-manager service holds it (same top-tier-secret custody note
  as the CF API token in §1.5).
- Community Terraform modules exist for exactly this (`zoro16/terraform-hcloud-server`,
  `tgunawandev/hetzner-vps-provision` — Terraform+Ansible) — reference implementations, not
  dependencies. ([zoro16 module](https://github.com/zoro16/terraform-hcloud-server), [tgunawandev](https://github.com/tgunawandev/hetzner-vps-provision))

### 4.3 The §17.9 swappable-VPS-port implication

Because Hetzner is a *port* (§17.9), the pool-manager must talk to an **abstract `VpsProvider`
trait** (`create_from_image`, `assign`, `destroy`) with a Hetzner adapter as the Wave-0 default. The
warm-pool/snapshot logic lives *above* the trait so a self-hosting vendor (or a future non-Hetzner
default) drops in a different adapter without touching claim logic. See §5 for how mature
self-hostable projects structure exactly this.

---

## 5. Swappable-infrastructure-port prior art (§17.3/§17.8/§17.9)

The roadmap's "every infra dependency is a port with a Wave-0 default" (§17.9 synthesis) is a
well-trodden pattern in self-hostable software. Concrete borrowable shapes:

- **Matrix/Synapse — storage as a pluggable module, DB as a cautionary counter-example.** Synapse
  stores media on the local filesystem by default but offloads to **S3 via a media-storage-provider
  module**; operators "choose their own bridges, bots, SSO provider, and storage backend, with no
  artificial feature gates, no vendor lock-in." *But* Synapse hard-requires **PostgreSQL** — the DB
  is *not* swappable. ([Synapse](https://github.com/matrix-org/synapse), [self-host guide 2026](https://danubedata.ro/blog/self-host-mastodon-matrix-server-vps-2026)) **Lesson for dowiz:** a port is only a port if the *default* was built behind the interface from day one. Synapse's storage was; its DB wasn't — and the DB is now un-swappable. §17.2/§17.9 must be enforced by putting the CF-tunnel, VPS, and signer behind traits *before* Wave-0 ships, not retrofitted.
- **Mastodon — adapter-by-config storage.** Mastodon selects local vs S3 vs GCS vs Swift object
  storage purely by environment variable, the canonical "one interface, config-selected adapter, one
  default" shape. → dowiz's tunnel/VPS/signer ports should likewise be **config/trait-selected with a
  Wave-0 default**, adapters community-maintainable (AGPL, §16.57).
- **SPIFFE UpstreamAuthority (again)** is the direct prior art for the *cert-root* port (§17.7): the
  signer is a plugin — self-signed default, external-CA adapter optional. dowiz's "self-root default,
  dowiz-cosign optional" is the same port with the same two adapters. ([SPIRE concepts](https://spiffe.io/docs/latest/spire-about/spire-concepts/))

**Synthesized port map for the blueprint (all trait-behind from day one):**

| Port (trait) | Wave-0 default adapter | Escape-hatch adapter | Roadmap ref |
|---|---|---|---|
| `TunnelProvider` | dowiz CF account (`cloudflared`) | vendor's own CF account / WireGuard relay | §17.3, §17.8 |
| `VpsProvider` | Hetzner (warm pool + snapshot) | any VPS / self-host bare metal | §17.9 |
| `CertRoot` (signer/upstream) | hub self-signed ML-DSA-65⊕Ed25519 | dowiz optional co-sign | §17.7 |
| `PaymentProvider` | (multi-provider adapter) | vendor's chosen PSP | §16.13 |
| `MediaStore` | in-stack default | vendor S3/GCS | §16.29 |
| `AiModel` | managed default | local/self-host model | §16.52 |

---

## 6. Wave-0 concrete recommendation

### 6.1 Claim-mechanic implementation

1. **Pool = warm Hetzner servers built from a Packer golden snapshot** (§4.1-B), each pre-populated
   with fixtures (§16.54), each already running `cloudflared` against a **pre-created remotely-managed
   tunnel** (§1.2) on the dowiz CF account, reachable at `hub-<id>.hubs.dowiz.org`.
2. **Each pooled hub is pre-minted its own self-signed ML-DSA-65⊕Ed25519 root capability-cert**
   (§2.4) at snapshot/provision time — the hub is trust-self-sufficient *before* anyone claims it.
3. **Claim = ownership assignment, zero infra work on the hot path** (§16.32): dowiz's (closed,
   §16.54) claim service (a) binds the hub's owner record to the claimant, (b) hands the claimant the
   **owner root capability-cert** (or, if the owner already holds a multi-hub root, appends a signed
   **child block** delegating this hub to that root — §2.4/§6.2), (c) the hub is already online, so
   the vendor sees a working, fixture-populated hub instantly.
4. **Background refill pipeline** (Terraform + Packer, §4.1-A) tops the warm pool back up, since
   §16.57 forbids reclaim → the pool is net-consumed. Alert/scale the CF account before the **1,000-
   tunnel cliff** (§1.4).
5. **Non-pool path** (§16.32): the `dowiz.org` interest form notifies the operator for manual
   follow-up — no automation required at Wave-0.

### 6.2 Cert-hierarchy design

- **Do not adopt SPIRE** (no ML-DSA, Go/K8s-heavy — §2.1). **Do borrow SPIFFE's model:** trust
  domain = hub root, stable SVID-style IDs, trust-bundle federation, optional upstream co-sign.
- **Implement a biscuit-style signed-block capability chain over the existing `HybridSigner`**
  (§2.3): root block (self-signed hub/owner root) → optional detached dowiz co-sign → child blocks
  (owner delegates each hub, Datalog scope facts, `may_delegate` flag, expiry). Offline
  mint/attenuate/verify against the root *public* key — satisfies §16.48 delegation + §17.3/§17.7
  survivability in one structure.
- **Revocation = short TTL + re-mint (primary) + owner-signed revocation blob gossiped within the
  owner's fleet (immediate) + optional dowiz transparency feed** (§2.5).
- `rcgen` only if/where an X.509 wire format is separately needed for TLS interop — it is plumbing,
  not the identity design, and carries no PQ half (§2.2).

### 6.3 Crypto-agility versioning scheme

- **`alg_suite` identifier field** per cert, `v1 = MLDSA65-Ed25519`, aligned to the standardized
  composite OID **`1.3.6.1.5.5.7.6.48`** (§3.1) — do not invent a bespoke scheme.
- **AND-verify both halves** (§3.1, matches dowiz B4 precedent).
- **Suite negotiation** at handshakes (TLS-style) + **downgrade protection** (§3.2-3.3).
- **SSH-style overlap rotation** for fleet migration: dual-publish new+old suite for a hub-local
  window, then retire old — no mesh-wide flag-day (§3.2).

---

## 7. Riskiest open unknowns for the Tier-3 blueprint

Ranked by how badly a wrong Wave-0 guess hurts:

1. **Control-plane blast radius of the single dowiz CF API token (§1.5).** The dowiz provisioning
   service can rewrite *every* hub's tunnel routing. There is no CF-native per-tenant sub-account
   boundary in the §16.45 model. If that token/service is compromised, an attacker re-points hubs'
   public hostnames. *Unknown:* what compensating control (HSM-held token, append-only mutation log,
   per-hub route-change signing) is acceptable for Wave-0, and whether §16.45's "one account" should
   already be an *account pool* to shrink the blast radius. **Highest-severity open item.**
2. **Capability-cert revocation latency vs. offline-survivability tension (§2.5).** Short-TTL re-mint
   needs *something* to re-mint against; if the owner root is offline/lost, either hubs expire
   (availability loss) or TTLs are long (compromise window). The re-mint authority, TTL value, and
   owner-root backup/recovery story are unresolved and safety-critical (this is money/auth red-line
   territory).
3. **Bespoke hybrid capability-token security review (§2.3).** Re-implementing biscuit's signed-block
   attenuation over ML-DSA-65⊕Ed25519 is *new crypto-adjacent code*. dowiz's own history (the B4
   batch-verify forgery caught only by an independent reviewer) says: this needs genuine adversarial
   review — canonicalization of blocks, block-reordering/truncation attacks, cross-suite confusion —
   before it holds owner delegation authority. Do not treat "it's just biscuit-shaped" as safety.
4. **Warm-pool economics under §16.57 no-reclaim (§4.1).** Idle warm servers + a monotonically
   depleting, never-recycled pool is an unbounded cost/refill-rate question. *Unknown:* target pool
   depth, refill cadence, and per-region pool count — and whether abandoned-but-never-reclaimed hubs
   should at least be *powered down* (still "theirs," but not burning a warm slot).
5. **`config_src: cloudflare` lock-in vs. the §17.3 escape hatch.** Remotely-managed tunnels keep
   ingress config in Cloudflare's control plane. Switching a hub to the vendor's own CF account or a
   WireGuard relay (§17.3/§17.8) means the hub must be able to *re-materialize* its routing config
   locally. *Unknown:* whether Wave-0 hubs should store a shadow local ingress config from day one so
   the escape hatch is a real switch and not a rebuild.
6. **Composite-sigs draft is not yet an RFC.** `draft-ietf-lamps-pq-composite-sigs` is at v19 and the
   OID `…6.48` could still shift before RFC publication. Pin dowiz's suite to an *internal* enum that
   *maps* to the OID, so an OID change is a one-line remap, not a cert-format migration.
7. **Federation trust-bundle mechanics deferred but design-constraining (§17.4).** If future
   federation uses SPIFFE-style bundle exchange, the Wave-0 cert format must already be
   bundle-expressible. Not building it, but not foreclosing it, is a real Wave-0 design constraint.

---

## 8. Sources

**Cloudflare Tunnel**
- Remote tunnel permissions / token: https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/remote-tunnel-permissions/
- Create a tunnel (API): https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/create-remote-tunnel-api/
- Account limits: https://developers.cloudflare.com/cloudflare-one/account-limits/
- Deploy Tunnels with Terraform: https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/deployment-guides/terraform/
- Automating Cloudflare Tunnel with Terraform (CF blog): https://blog.cloudflare.com/automating-cloudflare-tunnel-with-terraform/
- Terraform resource `zero_trust_tunnel_cloudflared`: https://registry.terraform.io/providers/cloudflare/cloudflare/latest/docs/resources/zero_trust_tunnel_cloudflared
- Cloudflare API — Tunnels: https://developers.cloudflare.com/api/resources/zero_trust/subresources/tunnels/

**Certificate hierarchy / capability tokens**
- SPIFFE concepts: https://spiffe.io/docs/latest/spiffe-about/spiffe-concepts/
- SPIRE concepts (CA, UpstreamAuthority, federation): https://spiffe.io/docs/latest/spire-about/spire-concepts/
- SPIRE server config (ca_key_type): https://spiffe.io/docs/latest/deploying/spire_server/
- SPIRE PQC support (open issue #6975): https://github.com/spiffe/spire/issues/6975
- X.509-SVID spec: https://spiffe.io/docs/latest/spiffe-specs/x509-svid/
- rcgen docs: https://docs.rs/rcgen/latest/rcgen/ · Issuer: https://docs.rs/rcgen/latest/rcgen/struct.Issuer.html · CHANGELOG: https://docs.rs/crate/rcgen/latest/source/CHANGELOG.md · AKI issue #261: https://github.com/rustls/rcgen/issues/261
- biscuit-auth: https://docs.rs/biscuit-auth/latest/biscuit_auth/ · repo: https://github.com/biscuit-auth/biscuit-rust · vs macaroons: https://lib.rs/crates/biscuit-auth
- Vouchsafe (offline capability graph): https://arxiv.org/pdf/2601.02254

**Crypto-agility**
- Composite ML-DSA (draft-ietf-lamps-pq-composite-sigs): https://datatracker.ietf.org/doc/draft-ietf-lamps-pq-composite-sigs/
- SSH key rotation (OpenSSH 6.8+): https://blog.djm.net.au/2015/02/key-rotation-in-openssh-68.html
- SSH host-key update draft: https://datatracker.ietf.org/doc/draft-ietf-sshm-hostkey-update/
- Dual-algorithm certs in TLS: https://link.springer.com/chapter/10.1007/978-3-032-16089-8_30
- PQC migration 2026 (hybrid TLS): https://appscale.blog/en/blog/post-quantum-cryptography-migration-ml-kem-ml-dsa-hybrid-tls-2026
- Post-quantum crypto-agility (Thales): https://cpl.thalesgroup.com/encryption/post-quantum-crypto-agility

**Hetzner provisioning**
- hcloud_server (Terraform): https://registry.terraform.io/providers/hetznercloud/hcloud/latest/docs/resources/server
- hcloud_snapshot: https://registry.terraform.io/providers/hetznercloud/hcloud/latest/docs/resources/snapshot
- Using the Hetzner Cloud Terraform Provider: https://davidhamann.de/2026/01/21/hetzner-cloud-terraform/
- terraform-hcloud-server module: https://github.com/zoro16/terraform-hcloud-server
- hetzner-vps-provision (Terraform+Ansible): https://github.com/tgunawandev/hetzner-vps-provision

**Swappable-port prior art**
- Synapse (S3 media module, Postgres-required): https://github.com/matrix-org/synapse
- Self-host Mastodon/Matrix 2026: https://danubedata.ro/blog/self-host-mastodon-matrix-server-vps-2026

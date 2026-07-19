# Trust-Boundary Scan — Is there a genuinely-closed 2-trusted-actor channel in dowiz/bebop2 that spends heavyweight crypto it doesn't need?

> Research-only. Zero code written, no branches touched. Every claim below is
> grounded in a file:line read of the live tree at HEAD (`main`, 2026-07-18) or a
> cited external reference. The operator's distributed-systems premise is stated
> precisely and tested against the real architecture — not assumed.

---

## 0. The question, stated precisely

Two mutually-trusted parties on an already-authenticated channel with **no adversary
in the threat model** do not need per-message asymmetric signing; a monotonic
sequence number over an append-only queue (Lamport-style) buys the ordering /
freshness / replay-prevention that a lot of the heavyweight machinery is spent on.
This is **true** — but only for a channel where *all three* of the following hold:

- **(a)** both endpoints are fully operator-controlled (not an end-user device, not a third party);
- **(b)** the channel does not cross a perimeter a compromised/external actor can realistically reach;
- **(c)** the design already spends heavyweight crypto/signing there that ordering could replace **without loss of security**.

The finding: **no channel in dowiz/bebop2 today satisfies all three.** Every place
that spends heavyweight crypto fails (a) or (b) or (c); every genuinely-closed
2-trusted-actor path either has **no channel at all** (same-process) or **already uses
cheap integrity** (loopback + RLS + content-addressed sequence numbers) with no
crypto to remove. The principle is sound; it has no applicable target here. Notably
the codebase *already applies the operator's principle correctly* (see §5).

---

## 1. The literature boundary (so the report is precise, not hand-wavy)

Grounding what sequence/ordering integrity does and does **not** give you, versus a
MAC or a signature (confirmed against canonical references — WebSearch budget was
exhausted session-wide, so these were fetched directly):

| Mechanism | Ordering | Freshness / replay-block | Integrity (content untampered) | Authenticity (who sent it) | Non-repudiation (prove *exactly* who, to a third party) |
|---|---|---|---|---|---|
| Monotonic seq # / Lamport counter, alone | ✅ | ✅ (in an already-authenticated channel) | ❌ | ❌ | ❌ |
| MAC (symmetric secret) | — | — | ✅ | ✅ (holder of shared key) | ❌ (either party could forge) |
| Digital signature (Ed25519 / ML-DSA, asymmetric) | — | — | ✅ | ✅ | ✅ (only private-key holder) |

Key consequence: **a sequence number only substitutes for signing when the channel's
integrity/authenticity is *already* provided by something else** (a session MAC / TLS
record layer / a physically closed bus) **and non-repudiation against a *compromised
endpoint* is not in the threat model.** A monotonic counter "proves neither integrity
nor authenticity — anyone can increment it without possessing secret information"
([Replay attack, Wikipedia](https://en.wikipedia.org/wiki/Replay_attack);
[Message authentication code, Wikipedia](https://en.wikipedia.org/wiki/Message_authentication_code)).
This matters directly for candidate #5 below: dowiz spends signing precisely where the
*insider itself* is the threat (P06), which ordering fundamentally cannot cover.

---

## 2. Candidate #1 — kernel ↔ engine: MOOT (no channel exists)

There is **no network hop, no IPC, no wire** between the kernel and the engine. The
engine is a Rust crate that consumes the kernel as an **in-process local path
dependency**:

- `engine/Cargo.toml:21` — `dowiz-kernel = { path = "../kernel", version = "0.1.0" }`, explicitly *"the SAME local-crate relationship agent-governance-wasm uses; no external/wgpu/serde crate is introduced."*
- Real in-process calls, not a boundary: `engine/src/bridge.rs:16` `use dowiz_kernel::csr::{Csr, LaplacianKind};`; `engine/src/field_energy.rs:21-23` pulls `csr`, `incidence`, `noether`; `engine/src/field_frame.rs:62` reads `dowiz_kernel::DT_STABLE`.
- `engine/src/lib.rs:2-6` — *"Authoritative compute is CPU-side; GPU/wasm is a display surface."*

There is nothing to sign and nothing to replace: it is a monomorphized Rust function
call inside one address space. The question is moot for this seam. (The only crypto
anywhere near the engine is the E1 energy-gate *test oracle* at `engine/src/lib.rs:19-23`,
compiled `#[cfg(test)]` — not a runtime contract.)

---

## 3. Candidate #2 — mesh node ↔ node: REAL channel, but FAILS (a)/(b) — wire adversary is explicitly in the threat model

This is the one place heavyweight hybrid crypto rides a genuine channel — and it is
**not** a closed 2-trusted-actor path.

- The carrier is real QUIC / WSS over the public internet: `bebop2/proto-wire/src/iroh_transport.rs:1-5` — *"QUIC transport — real node-to-node carrier (pure-Rust quinn/rustls)… signed on send and verified on recv through the RequireBoth hybrid gate."* Sibling transports: `wss_transport.rs`, `stdio_transport.rs`, `sync_pull.rs`.
- The gate requires BOTH legs: `bebop2/proto-cap/src/hybrid_gate.rs:1-12` — Ed25519 (`bebop2-core::sign`) AND ML-DSA-65 (`signed_frame::{sign_pq,verify_pq}`), `RequireBoth`.
- **The design explicitly assumes the relay is NOT trusted.** `docs/design/mesh-real/MESH-REAL-PLAN.md:91` and `BLUEPRINTS-MESH-REAL.md:121` both call the encryption layer *"defense-in-depth **past-semi-trusted-relay**"* with `ML-KEM-768→XChaCha20-Poly1305` payload encryption. A semi-trusted relay is, by definition, an adversary-in-scope on the wire — premise (b) is deliberately false here.
- **The red-team already demonstrated live wire attacks** on exactly this path: expired-capability-accepted-on-the-wire (`bebop2/docs/red-team/2026-07-13/B2-protocol-authz.md:34,55,80`, PoC accepts at `now=0`), and a PQ-forgery gap where a single Ed25519 sig is accepted (`B2-protocol-authz.md:86`). You cannot drop signing on a path with a proven forgery/replay surface.
- **Authorization is rooted in a delegation chain precisely because nodes are not all one trusted party.** `hybrid_gate.rs` (module doc) rejects a self-signed frame as `UnknownIssuer`; every frame must carry a UCAN-subset `Delegation` chain rooted in an enrolled `AnchorRoster`. The whole roster/anchor/genesis apparatus exists *because* a node's peer is not axiomatically trusted.

Even in the single-operator deployment (`docs/design/mesh-real/MESH-12-RESOLVED-2026-07-14.md:17-23`, "operator-signed-root … single-operator mesh (dowiz today)"), the *endpoints* may be operator-owned but the **channel crosses the public internet between physically separated devices** (hubs, courier phones). Operator-owned endpoints ≠ closed channel: (a) can hold while (b) fails, and here (b) fails by design. And the roster is built to *also* admit third-party-run nodes later (WoT/QR deferred, not excluded), plus the agentic-mesh B1 AgentBridge *"admits third-party agent code (LangGraph, …)"* (`docs/design/agentic-mesh-protocol-2026-07-17/COUNSEL-ethics-strategy-review.md:41`). Not closed now; less closed later.

**Verdict: not a candidate. The mesh is the textbook case *for* keeping the crypto.**

---

## 4. Candidate #3 — event_log durable-insert-then-`set_tip`: genuinely closed, but there is NO channel and NO signing to remove

`kernel/src/event_log.rs` (MESH-06) is a **per-node, single-writer, in-process,
local-first** structure:

- `event_log.rs:1,19` — *"per-node… single-node local-first only"*; `MemEventStore` is *"non-durable and not shared across processes."*
- The write path is a local durable-insert-then-`set_tip`: `append()` (`:302`) computes the content-id then `self.store.set_tip(id)` (`:319`); `set_tip` (`:204,246`) is a plain local store mutation. No peer, no socket.
- **Integrity here is already ordering-based, not signature-based** — exactly the operator's model: the event-id is `SHA3-256(prev ‖ actor_pubkey ‖ actor_seq ‖ payload)` (`:146,152`) with a per-actor **monotonic counter `actor_seq`** (`:130-140`). Duplicates are a *structural* no-op (`:6-8` "no TTL dedup — a duplicate is a structural no-op"). The local append path signs nothing.

So this seam already embodies "cheap ordering, no crypto." The signing that *does*
exist (`kernel/src/mesh.rs` `SignedEntry`, ML-DSA-65, `#![cfg(feature = "pq")]`) is
applied **only at the cross-node sync/gossip layer** — i.e., candidate #2's network
boundary, not this local structure. Nothing to remove; the principle is already applied.

---

## 5. Candidate #4 — app-server ↔ database (pgrust): genuinely closed, but ALREADY cheap — fails (c) (no heavyweight crypto is there to remove)

This is the *closest* real match to the operator's premise, and it shows why the
principle is already honored:

- The runtime today is `native-spa-server` (a **read-only static file server**, `deploy/native-spa-server.service`, *"never writes… no secrets"*) plus `pgrust` on **loopback only**: `deploy/pgrust.toml` `listen = "127.0.0.1:5432"`, `deploy/pgrust.env` `PGRUST_LISTEN=127.0.0.1:5432`, fronted by a native PgBouncer sidecar.
- The isolation model is explicitly **not** crypto: `deploy/README.md` — *"isolation comes from the host process model + the app-level Row-Level-Security cross-tenant gate, not from a container boundary"*; `pgrust.toml` `rls = { cross_tenant = "deny" }`. `pgrust.service` header: *"a trusted first-party component."*

So the app↔DB path **is** a genuinely-closed 2-trusted-actor channel (a ✅, b ✅ —
loopback, same host). But it **already uses the cheap option** — Postgres wire +
password over `127.0.0.1` + RLS — and spends **zero** hybrid signing. There is no
heavyweight crypto here to swap for sequence numbers; (c) is false. (There is also no
separate TS `apps/api` service in the live tree — the old `apps/api/src/server.ts` the
Repowise index references is the retired stack; today the kernel is consumed as
**in-process wasm in the browser** or a **native rlib**, `kernel/src/json_api.rs:1-16`.)

---

## 6. Candidate #5 — P06 key_K/key_V (capability-cert / HybridSigner): NOT a channel, and the threat IS the trusted insider — ordering is fundamentally insufficient

The `capability_cert` chain (`kernel/src/capability_cert.rs:1-27`, hybrid Ed25519⊕ML-DSA-65
over the `SignatureVerifier` seam) and the P06 split-identity verifier are worth naming
explicitly because they look like "internal" crypto but are the exact opposite of a
replaceable case:

- The capability-cert chain authorizes actors whose scope terminates at **untrusted edge devices** — `kernel/src/ports/agent/scope.rs` resources include `Customer` (0x08), `Menu`/courier catalog, `Order`, and the RED-LINE `Ledger` (0x02). These are courier/customer-facing; premise (a) fails.
- **P06 signs git diffs and verdicts, not a network message.** `docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P06-v1-split-identity-verifier.md:§0` — the problem is *"the same agent both claims and checks… a claim-shaped artifact replaces a check-shaped artifact"* (a 52-second GREEN on a 1,610-line diff). `key_K` signs the author's diff, `key_V` signs the verifier's verdict, under **distinct** anchor keys (`§2`, `K ≠ V` enforced at load).

Here the crypto delivers **non-repudiation / authorship-binding against a trusted-but-
self-certifying insider** — precisely the property §1's table shows a sequence number
*cannot* provide (a counter "proves neither integrity nor authenticity; anyone can
increment it"). The whole point is that the two identities are the *same operator's*
agents, yet must be **cryptographically un-confusable**. Sequence numbers would be a
strict security regression. (c) is false in the strongest possible sense.

---

## 7. Honest conclusion

**No genuinely-closed 2-trusted-actor channel in dowiz/bebop2 today spends heavyweight
crypto that a monotonic sequence number could replace without loss of security.** The
premise and the crypto never coincide:

| Seam | (a) both endpoints operator-owned | (b) no reachable adversary perimeter | (c) spends heavyweight crypto | Replaceable by ordering? |
|---|---|---|---|---|
| kernel ↔ engine | n/a — same process | n/a — no channel | ❌ none | **Moot** (no channel) |
| mesh node ↔ node | endpoints yes, but channel crosses internet | ❌ "semi-trusted relay" by design; live PoCs | ✅ hybrid Ed25519⊕ML-DSA | **No** — adversary in scope |
| event_log `set_tip` | ✅ | ✅ single-writer, in-process | ❌ none (already seq#+SHA3) | **Already done** — no crypto to remove |
| app ↔ pgrust (loopback) | ✅ | ✅ `127.0.0.1` + RLS | ❌ none (password + RLS) | **N/A** — already cheap |
| capability-cert / P06 K/V | ❌ edge devices / n/a — not a channel | ❌ insider is the threat | ✅ hybrid signing | **No** — needs non-repudiation |

The operator's distributed-systems point is **technically correct** and, importantly,
**dowiz already applies it**: it uses cheap ordering (monotonic `actor_seq` +
content-addressed hash chain) on the closed local path (event_log) and loopback+RLS on
the closed DB path, and reserves hybrid signing for exactly the two places ordering
cannot cover — the internet-crossing mesh wire (adversary in scope) and the
self-certifying-insider governance gate (non-repudiation required). There is no
mis-placed heavyweight crypto on a closed channel to harvest. Do **not** stretch to
manufacture one; the app↔pgrust loopback is the only path that even matches the
"closed" half, and it carries no signing to begin with.

### If the operator still wants a target
The only future scenario where the premise could become live: a **same-host,
multi-process** split of the backend (e.g. `native-spa-server`/API process ↔ a kernel
worker process over a **Unix-domain socket**) where the OS peer-credential
(`SO_PEERCRED`) already authenticates both ends. *If* such an internal split were ever
introduced *and* someone proposed putting capability-cert signing on that loopback IPC,
that would be the genuine candidate for a monotonic-sequence substitute. It does not
exist today (there is no second backend process — the kernel is in-process wasm/rlib),
so this is a watch-item, not a change.

---

*Sources: live tree reads cited inline (file:line). Literature:
[Replay attack, Wikipedia](https://en.wikipedia.org/wiki/Replay_attack);
[Message authentication code, Wikipedia](https://en.wikipedia.org/wiki/Message_authentication_code).*

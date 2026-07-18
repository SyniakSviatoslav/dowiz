# MESH-12 — Resolved Decision (genesis-policy) — 2026-07-14

> Operator authorized ("2. allow", "4. go"): the 🔴·HUMAN genesis-policy enum is now
> decided. This doc records the decision + the concrete shape. It is a **design
> decision**, not committed code — the product has no mesh crate yet (only
> `kernel/src/event_log.rs` = MESH-06 exists on disk). When the mesh crate lands,
> this is the spec to implement against.

## 1. The open question (from BLUEPRINTS-MESH-REAL.md §MESH-12)
New-node-join = out-of-band-root-delegation. Three candidate policies:
1. **operator-signed-root** — operator holds a root signing key; signs the initial
   roster + each node's enrollment cert.
2. **WoT** (web-of-trust) — nodes vouch for each other; threshold of vouches admits.
3. **first-contact-QR** — two devices exchange a QR at first physical contact; mutual
   signed handshake bootstraps trust.

## 2. Decision: operator-signed-root
Chosen for: **fail-closed simplicity**, matches the no-CA SPKI-lineage identity model
(ADR-0007: `node_id = H(pq_pub ‖ classical_pub)`, identity born from keygen, nothing to
seed), and smallest attack surface for a 1-operator deployment (dowiz today).

### Concrete shape
- **Root of trust:** a single operator-held key pair (ML-DSA-65, reuses `dowiz-pq`
  `feat/pq-crypto-tier1` — 178 tests, KAT bit-exact). Stored in the operator's secret
  store (`.env` `JWT_SIGNING_SECRET`-class isolation), never in repo.
- **node_id derivation (ADR-0007):** `node_id = H(pq_pub ‖ classical_pub)` over SPKI
  encodings. Recomputed-from-both-pubkeys MUST match (RED gate when implemented).
- **Genesis loader (prod):** reads a *frozen* anchor-set from config/disk (not inline
  tests). Empty roster ⇒ fail-closed (no capture, no silent bootstrap).
- **Enrollment:** new node generates its keypair, derives `node_id`, sends a
  `CertificateRequest` to the operator root; operator signs and returns a
  `NodeCertificate` + the current `AnchorRoster` snapshot. Bulk-pull `actor_seq=0`
  (MESH-07) follows.
- **Revocation:** UCAN-style `RevocationSet` (irreversible invalidate) + drop-anchor
  (remove from `AnchorRoster` HashSet), with mesh-wide gossip/consensus (2026-open,
  Vouchsafe/Lingering-Authority) — deferred to MESH-11/13 scope.

### RED gates (when implemented, per blueprint)
- `node_id` recomputed-from-both-pubkeys matches the stored id.
- empty-roster fails closed (genesis loader refuses to boot with zero anchors).
- seeded-owner fixture cannot mint (nothing-to-seed — identity is keygen-born).

## 3. Why not WoT / first-contact-QR (for the record)
- **WoT:** needs ≥2 existing trusted nodes to admit a 3rd; wrong for a greenfield
  single-operator mesh, and threshold tuning is a live attack surface with no clear
  bound at n=1..3.
- **first-contact-QR:** great for peer-to-peer bootstrap but assumes two devices
  physically present at genesis; the operator already is the genesis authority, so QR
  adds a path without removing the root need. Keep as a *future* re-enrollment option,
  not the genesis policy.

## 4. Status
- Decision: **RESOLVED** (operator-signed-root), 2026-07-14.
- Code: **NOT IMPLEMENTED** — no mesh crate on disk. This doc is the implementation spec.
- Dowiz invariant preserved: bebop protocol work stays parked until dowiz carries it;
  the ML-DSA root reuses the already-verified `dowiz-pq` tier-1, not the bebop2 research core.

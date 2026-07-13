# ADR-0007 — Self-certifying node identity; no directory, no phone-home

- Status: PROPOSED (design gate — ports red-team D1-F1 / legacy C2 class into the NEW architecture)
- Date: 2026-07-13
- Red-line: AUTH / CRYPTO. Forward-only. Reversible (drop the verifier if a different scheme is chosen).
- Supersedes/relates: MANIFESTO §2 (no directory, no central identity provider, no phone-home), DECISIONS D1 (drop the centralized server), DECISIONS D2 (PQ + classical dual key), red-team `D1-appsec-authz.md` F1 (seeded-credential foothold).
- Relates: `RESONATOR-DESIGN.md` (same "port the lesson, don't patch dead code" principle).

## Context
The legacy stack shipped a **seeded weak owner credential** (`test@dowiz.com` / `test123456`) live on prod, which minted production-key-signed `role:owner` JWTs — an immediate, zero-effort authenticated foothold (red-team D1-F1, CRITICAL, confirmed against prod). Root cause was structural: a **central identity issuer** the operator had to provision, protect, and keep free of fixtured accounts. Every central-issuer design reproduces that failure mode — compromise or mis-provisioning of the issuer yields owner sessions.

In the decentralized protocol (bebop2 mesh) there is **no central auth issuer and no directory service** (MANIFESTO §2). Node identity must therefore be **self-certifying** and **verifiable without a phone-home**.

## Decision
- **`node_id = H(pq_pub ‖ classical_pub)`** — dual-key binding (ML-KEM-768 + ML-DSA-65, per DECISIONS D2). No central CA, no directory, no provisioned accounts, no shared symmetric password.
- A peer proves identity by signing a challenge with **both** its classical and PQ keys; the verifier recomputes `node_id` from the presented public keys and checks the bind. AuthN is **possession of the key pair + the PQ envelope**, not a shared secret.
- No `test@*` / fixture account can ever exist, because there is nothing to "seed" — identity is born from a key generation, not a provisioned row.

This closes the D1-F1 class **at the architecture level**: there is no provisioned credential to ship, no password to guess, no central issuer whose breach yields owner sessions. The "seeded weak account" attack surface is eliminated by construction.

## Alternatives considered
- **A — central IdP / JWT issuer (legacy model):** REJECTED. Reproduces D1-F1 (central compromise = full foothold) and contradicts MANIFESTO §2.
- **B — pre-shared symmetric keys:** REJECTED. Reinvents the seeded-credential failure mode (a shared secret that can be weak/shipped).
- **C — self-certifying identity (chosen):** possession-based, no central trust anchor to breach.

## Consequences
- **+** No provisioned credential, no phone-home, no directory to breach.
- **+** Red-team D1-F1 root cause removed by design, not by a CI guard that can regress.
- **−** Key loss = identity loss (mitigation: operator-held out-of-band encrypted backup of the key pair).
- **−** Initial trust bootstrap needs an explicit out-of-band key exchange (first-contact QR / operator-signed root).

## Open items / human decisions
- **HUMAN — bootstrap trust anchor:** operator-signed root vs Web-of-Trust vs first-contact QR. Owner: operator.
- **Proof (Mandatory Proof Rule):** a falsifiable test that a peer presenting only a forged `node_id` (key mismatch) is rejected, and that no provisioned/shared secret exists in the handshake path.

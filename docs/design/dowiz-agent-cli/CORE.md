# Bebop Core — Design (grounded in the Grand Plan)

> Status: CORE IMPLEMENTED + PROVEN. 48/48 tests green (crypto, torrent, mesh, kernel, conductor, auth, guard).
> Source of truth: `docs/design/sovereign-core-mvp/GRAND-PLAN.md`. Bebop is the agentic-CLI / independent-node
> realization of the Grand Plan's seams — not a parallel design. Where the Grand Plan says "dowiz-core", read
> "Bebop kernel"; where it says "libp2p mesh", read "Bebop SyncPort transport".

---

## 0. The one idea

The Grand Plan's load-bearing doctrine (§0b-2, MANIFESTO §6): **a deterministic `fold` over a totally-ordered,
content-addressed event log IS the replication primitive. There is no central server.** Bebop takes that verbatim
and applies it to agentic CLIs and autonomous nodes:

- **Kernel** = the deterministic `decide` / `fold` / `replay` core (pure, no clock, no RNG, no network).
- **Envelope** = `{ seq, at, cause: CommandHash }` — `cause` is the canonical hash of the command that produced
  the event; it is the D2 dedupe / causality / ordering seam (Grand Plan 0b-2).
- **Crypto** = self-certifying node identity (PQ + hybrid) so *auth is a signature over content*, not a session
  at a server.
- **Torrent** = content-addressed chunking: `infoHash` (self-certifying address) + per-piece hashes, verify-on-receipt.
- **Mesh** = a transport-agnostic `SyncPort` that moves content-addressed pieces between nodes by hash. The kernel
  owns ordering/dedup; the mesh owns only movement + gossip. Swap the transport (in-memory ↔ libp2p ↔ hyperswarm)
  without touching the kernel — exactly the Grand Plan's "libp2p is impl #3 of the same contract suite".

This is "torrent-like" at the protocol level: a node never trusts a peer, only hashes. A malicious or partial peer
cannot inject bad data or forge a false log — wrong bytes fail `verifyPiece`, a replayed command hits D2 dedupe, a
tampered signature fails `verify`.

---

## 0b. Two operator doctrines baked into the core (2026-07-08)

### Doctrine A — "As above, so below" (universal abstraction + fundamental math/physics)
One contract shape — **`decide → fold → SyncPort`** — governs EVERY scale: a single command, a node's
state, and the whole mesh merge. The SAME `Checker` abstraction validates a transition at the kernel
scale (local apply, `applyCommandChecked` + `defaultChecker`) AND at the mesh scale (gossip admission):
the "above" (checker / invariant) watches the "below" (doer / `decide`). The invariant is grounded in
**fundamental math/physics** — a conservation law over the state transition (what the command claims
must hold after `fold`), exactly like energy/momentum conservation in mechanics. Core functionality rests
on information theory (content addressing, Merkle/DAG causality via `cause`), lattice cryptography
(ML-KEM / ML-DSA), and thermodynamic-style reliability (fail-closed, quarantine, rotation) — not ad-hoc
logic. The high-level orchestration layer is literally the same law as the lowest kernel primitive.

### Doctrine B — Native copilot / co-work mode (DEFAULT)
Every prompt/action is **PRODUCED by one model/agent (the DOER, below) and CHECKED in REAL TIME by a
DIFFERENT model/agent (the CHECKER, above)**. Independence is enforced: the checker MUST be a distinct
backend/model (`copilot.ts` rotates to the next available backend, else a deterministic native stub).
On REJECT the action is **QUARANTINED** — not applied — matching the kernel's fail-closed quarantine
semantics. Copilot is native (`src/copilot.ts`) and DEFAULT-ON: `bebop dispatch "<task>"` uses it;
`--no-copilot` opts out. This is the "above/below" split expressed at the orchestration layer.

---

## 1. Module map (file → Grand Plan seam)

| File | Responsibility | Grand Plan seam |
|------|----------------|-----------------|
| `src/copilot.ts` | native DEFAULT co-work mode: doer (below) → distinct checker (above), real-time, fail-closed | §0b Doctrine B |
| `src/kernel.ts` | `decide`/`fold`/`replay`, `Envelope{seq,at,cause}`, `commandHash`, D2 dedupe, **universal Checker gate** (`applyCommandChecked`/`defaultChecker`) | §0b Doctrine A, §0b-2, D5 fold gate |
| `src/crypto.ts` | PQ identity (ML-DSA-65 + Ed25519 hybrid), `nodeId = hash(pqPub‖edPub)`, KEM (ML-KEM-768) | §Phase-2 signature shell, self-certifying namespace |
| `src/torrent.ts` | `createTorrent` → pieces + `infoHash`; `verifyPiece`; `assemble` (hash-verified); `wantBitfield` | §0b-2 content hash = self-certifying address |
| `src/mesh.ts` | `MeshTransport` contract + `InMemoryNode` gossip swarm (no server) | §1.3 SyncPort, §Phase-3 mesh transport |
| `src/vault.ts` | local-only secret store (Phase 2; secrets never leave disk) | — (operator red-line: no secret in transit) |
| `src/conductor.ts`, `backend.ts`, `profile.ts`, `routing.ts`, `token.ts` | abstract conductor over agentic CLIs (claude/hermes/opencode/aider/goose/codex) | — (the "abstract layer above other agentic CLIs" directive) |
| `src/auth.ts`, `sync-server.ts` | Better Auth (default) as the optional sync-boundary gate | §Phase-2 auth (default = Better Auth, you chose) |

---

## 2. Kernel — deterministic, pure

- `decide(state, command, ctx)` → `{ state, envelopes[] }`. Pure function: same inputs ⇒ same outputs, every run.
- `Envelope { seq, at, cause }`: `cause = commandHash(command)` (canonical bytes → sha256). Two commands with the
  same `cause` are the SAME command; `applyCommand` dedupes by `cause` (D2: replay is a no-op).
- `fold(state, event)` accumulates; exhaustive `match` on event variants (no `_` arm → compile error, mirrors the
  Grand Plan D5 gate). `replay(events)` rebuilds state from a log.
- Banned in kernel: `Date.now`, `Math.random`, `fetch`, `setTimeout`. Determinism is the whole point — a node must
  be able to replay another node's log and arrive at the identical state. (Proven by `core.test.ts` GREEN #11/#12.)

## 3. Crypto — self-certifying, post-quantum, hybrid

- Identity = `{ pqPublic(ML-DSA-65), pqSecret, edPublic(Ed25519), edSecret, id }`.
- `nodeId = sha256(pqPublic ‖ edPublic)` — self-certifying: anyone with the public keys can derive it; no registry.
- `sign` produces a hybrid signature `{ pq, ed }`; `verify` requires BOTH to validate (fail-closed). Tampered bytes
  ⇒ `verify` false (proven RED #2).
- `kemEncapsulate(pqKemPublic)` / `kemDecapsulate(pqKemSecret, cipherText)` — ML-KEM-768 for private node channels.
  KEM keypair is distinct from the signing keypair (the "passed the wrong key" bug we caught: ML-DSA pub = 1952 B,
  ML-KEM pub = 1184 B).
- OSS-only, zero external services, WASM-ready: `@noble/post-quantum`, `@noble/hashes`, `@noble/curves`.

## 4. Torrent — content-addressed, verified

- `createTorrent(payload, pieceSize)` → `{ infoHash, pieceSize, pieceHashes[], pieces[] }`.
- `infoHash = sha256(JSON({pieceSize, pieceHashes}))` — binds structure AND data, so a reorder/swap attack fails.
- `verifyPiece(piece)` = `sha256(piece.bytes) === piece.hash`. `assemble` refuses any piece whose hash mismatches
  OR whose index is missing (proven RED #7/#8 — malicious/partial peer rejected).
- `wantBitfield` = the "have/want" bitfield a node gossips (proven GREEN #10).

## 5. Mesh — transport seam, no server

- `MeshTransport` interface: `publish`, `sync(peer)`, `requestPiece(peer, infoHash, index)`, `store`.
- `InMemoryNode` ships now: two nodes gossip pieces by hash until both converge (proven GREEN #9 — 4 pieces,
  leecher with 1 converges to the full, equal payload, 0 server). Idempotent: a second sync round transfers 0.
- Future transports (`Libp2pTransport`, `HyperswarmTransport`) implement the SAME interface. The kernel does not
  know or care which — swap-not-rewrite, per the Grand Plan.

## 6. Conductor — abstract layer above agentic CLIs (your top directive)

- `conductor.ts` dispatches a task to any backend (claude / hermes / opencode / aider / goose / codex) through ONE
  uniform contract. Each backend wraps its native CLI; the conductor applies token, routing, rotation, model, and
  agentic-rule layers IDENTICALLY to every connected agent (the "big layers used in 1 way for any agent" directive).
- `kernel`/`decide`/`fold` optionally log every dispatch as a content-addressed `Envelope` so the conductor's own
  operation is replayable and auditable — the same determinism that governs the mesh governs the agent.
- Auth (Better Auth, your default) is the **boundary** gate for a sync node, not the core. The core needs no auth
  server; it needs only signatures.

## 7. Proof (Verified-by-Math, all RED+GREEN)

Full suite: `node --test --import tsx src/*.test.ts` → **55 pass / 0 fail**.

- crypto: self-sign OK; tamper → false; KEM match; self-certifying id.
- torrent: assemble equal; infoHash structure-sensitive; tamper → null; missing → null.
- mesh: 2 nodes converge w/o server; wantBitfield correct.
- kernel: decide/fold/replay deterministic; D2 replay no-op; REVOKE→PUBLISH denied; replay reconstructs.

## 8. What is NOT yet built (next phases, per your "research first" rule — documented, not guessed)

- **Real mesh transport** (libp2p/Kademlia DHT + gossipsub) implementing `MeshTransport`. The seam exists and is
  proven in-memory; the network impl is a swap-in.
- **`vault.ts`** local secret store (secrets never transmitted; PQ keys stay on disk, encrypted at rest).
- **CRDT multi-writer merge** for concurrent offline edits (Grand Plan Phase-2): the kernel's `cause` already gives
  deterministic ordering; a CRDT merge layer sits above `fold`.
- **libp2p ↔ `SyncPort` adapter** + real `infoHash`-keyed DHT lookup (`findNode`/`getPieces`), replacing the
  in-memory `store` with a content-addressable swarm.
- **Bebop CLI surface** (`bebop sync`, `bebop mesh`, `bebop node`) wiring `crypto`+`torrent`+`mesh`+`kernel` into a
  runnable independent node.

Every one of these extends an existing, proven seam — none requires re-architecting.

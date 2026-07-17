# R2 — Web3 Good Patterns: Critical Evidence Review

> Research stream 2 of 6 for the agentic-mesh synthesis. 2026-07-17.
> Discipline: every "it worked" claim carries a falsifiable mechanism, not popularity.
> Numbers are either (a) cited from a named source, or (b) explicitly flagged as
> order-of-magnitude estimates. Convergence with already-built dowiz/bebop2 machinery
> is noted honestly — several of these patterns are things the kernel already does.

---

## 1. Account abstraction (ERC-4337 → EIP-7702)

**What actually happened.** ERC-4337 shipped March 2023 with no Ethereum consensus
change: a parallel `UserOperation` mempool plus a singleton `EntryPoint` contract. By
2024 roughly 40M smart accounts existed and 100M+ UserOperations had been processed,
with the large majority of UserOps paid by paymasters (app-sponsored gas)
([Turnkey](https://www.turnkey.com/blog/account-abstraction-erc-4337-eip-7702),
[EIP-4337 two years later](https://samuelokediji.medium.com/eip-4337-two-years-later-dc638a86b49d)).
EIP-7702 then landed in-protocol with Pectra (May 7, 2025): an EOA signs a delegation
that gives its address contract code, keeping its key and address; ~14M EOAs had signed
at least one delegation per Dune-aggregated dashboards
([eco.com comparison](https://eco.com/support/en/articles/14797813-eip-7702-vs-erc-4337-two-smart-wallet-paths)).
MetaMask's late-2025 adoption pushed it to the mainstream EOA base.

**Unsentimental caveat.** "Majority of UserOps are paymaster-sponsored" cuts both ways:
it proves gas sponsorship was the killer feature, and it also means much of the volume
is subsidy-driven (campaigns, airdrop farming), so raw UserOp counts overstate organic
demand. The durable signal is architectural, not volumetric: every serious wallet stack
in 2026 supports pluggable validation.

**Why it worked (falsifiable mechanism).** ERC-4337 decoupled *who may authorize an
action* from *one fixed signature scheme*. An account is valid if its own `validateUserOp`
predicate accepts the message — which is how passkeys/WebAuthn signers, session keys,
spending limits, and social recovery shipped without a hard fork. The falsifiable test:
if the value were "smart contract wallets" generically, Gnosis Safe (2018) would have
already saturated the niche; it didn't, because Safe still required someone to pay gas
from an EOA and used a fixed multisig policy. 4337's two specific additions — arbitrary
per-account validation logic and third-party gas sponsorship inside the same envelope —
are exactly what the adoption clustered around. EIP-7702's rapid uptake confirms the
second half: the market wanted the *policy layer*, not the new account type (it retrofits
policy onto existing keys).

**Transferable lesson.** Define mesh messages as `(envelope, validation-policy-of-actor)`,
not `(envelope, fixed-sig-scheme)`. A node's identity record should declare its own
acceptance predicate — single Ed25519, hybrid ML-DSA⊕Ed25519, t-of-n threshold, session
key with expiry — and verifiers evaluate the actor's declared policy. Add the paymaster
analog: the party who *pays* for execution (relay, compute, storage) can be a different
actor from the one who *authorizes* it, bound in one signed envelope.

**dowiz/bebop2 convergence.** Partial. bebop2 already has per-actor PQ identity and
hybrid ML-DSA⊕Ed25519 (UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT v3), which is one hard-coded
point in the policy space. The *pluggable per-actor validation predicate* (policy as data,
verifier evaluates the actor's declared scheme, upgradeable per-actor without protocol
version bumps) is genuinely new capability, and it is the mechanism that would let mesh
operators bring their own agent-auth schemes.

---

## 2. ZK validity proofs vs fraud proofs

**Structural difference.** A validity proof (SNARK/STARK) is checked once, cheaply, and
finality is immediate on verification; there is no challenge window and no liveness
assumption that at least one honest watcher exists and can get a challenge included.
Optimistic systems import both: a ~7-day window and a censorship-resistance assumption
about challengers. The cost is moved to the prover.

**Real 2025–2026 numbers.** Succinct's SP1 Hypercube demonstrated proving Ethereum
block 22309250 (143 txs, 32M gas) in 10.8s, and ~90% of blocks under 12s on a cluster
of 100+ GPUs (May 2025)
([Succinct](https://blog.succinct.xyz/sp1-hypercube/), [The Block](https://www.theblock.co/post/355013/succinct-introduces-zkvm-sp1-hypercube-claims-real-time-ethereum-proving));
by early 2026 the same pipeline reports 99.7% of L1 blocks proven <12s on 16 RTX 5090s
([Succinct](https://blog.succinct.xyz/real-time-proving-16-gpus/)). RISC Zero's R0VM 2.0
cut Ethereum-block proving from ~35min to ~44s (April 2025)
([ChainCatcher](https://www.chaincatcher.com/en/article/2200417)). Reported costs fell
from $0.5–2 per block (April 2025) to an average of ~3.5¢ per proof by December 2025
([Aligned year-review](https://blog.alignedlayer.com/the-year-of-zkvm-real-time-proving-milestones-present-and-future/)).
Treat all of these as vendor-reported; the order of magnitude — **seconds-to-tens-of-seconds
proving latency on dedicated GPU fleets, cents-to-dollars per block, milliseconds to verify** —
is corroborated across independent vendors.

**Verdict for a mesh-agent hot path.** Per-message validity proofs are not realistic:
proving is 4–6 orders of magnitude more expensive than signing, needs GPU infrastructure
a mesh node won't have, and 10s latency is not "real-time" for messaging/trading.
What *is* realistic: (a) mesh nodes **verifying** externally produced proofs in
milliseconds — verification is constant-cost and CPU-only; (b) batched settlement /
periodic state checkpoints, where one proof covers thousands of events and amortized
cost is sub-cent. The transferable design principle is **verification asymmetry**: make
the party asserting a claim bear the cost; keep every consumer's check O(ms) with no
watchtower/liveness assumption. Fraud-proof-style designs ("someone will challenge bad
state") should be rejected for the mesh — they smuggle in an honest-watcher availability
assumption that a sparse mesh cannot guarantee.

**dowiz/bebop2 convergence.** The kernel's fold/decide replay is a *re-execution* checker
(verifier cost = prover cost). Signed claims + periodic proof-of-replay checkpoints
would be new; per-message ZK is explicitly not recommended.

---

## 3. Threshold signatures and DKG

**The evidence from failures.** The two canonical bridge disasters were key-custody
failures, not cryptography failures. Ronin (March 2022, ~$624M): 5-of-9 validators,
attacker phished into Sky Mavis and obtained four Sky Mavis keys plus the Axie DAO key
via a whitelisted gas-free RPC backdoor — one organization's compromise yielded a
supermajority ([Merkle Science / SoK 2023 review](https://arxiv.org/html/2501.03423v1)).
Harmony Horizon (June 2022, ~$100M): a 2-of-5 multisig; two decrypted keys sufficed
([Merkle Science](https://www.merklescience.com/blog/hack-track-analysis-of-harmonys-horizon-bridge-exploit)).
The falsifiable pattern: low threshold × correlated custody (many keys under one org,
one infra, one phishing blast radius) = single point of failure with extra steps.

**What threshold+DKG actually changes.** With distributed key generation the full private
key never exists on any machine at any time; signing is an interactive protocol among t
parties. This removes the "steal the key file" attack class entirely — but it moves the
attack surface into the MPC protocol implementation. That surface is real: TSSHOCK
(Verichains, 2023) demonstrated full key extraction against deployed GG18/GG20/CGGMP21
implementations ([verichains.io/tsshock](https://verichains.io/tsshock/)), and
CVE-2023-33241/BitForge (Fireblocks) exploited a missing Paillier-modulus ZK check —
key extraction after as few as 16 signatures
([Fireblocks](https://www.fireblocks.com/blog/gg18-and-gg20-paillier-key-vulnerability-technical-report)).
Lesson inside the lesson: prefer the *simple* audited scheme. FROST (RFC 9591), with the
Zcash Foundation's Rust implementation (`frost-ed25519` et al.), passed an NCC Group
assessment with no high/critical findings and is stable
([ZcashFoundation/frost](https://github.com/ZcashFoundation/frost),
[NCC report](https://www.nccgroup.com/media/m1yjijzn/_ncc_group_zcashfoundation_e008263_report_2023-10-20_v11-1.pdf)).

**Post-quantum threshold: honest status — immature.** Threshold ML-DSA exists only as
research: "Efficient Threshold ML-DSA" presented at NIST's 6th PQC conference supports
**up to 6 parties** ([NIST CSRC](https://csrc.nist.gov/csrc/media/events/2025/sixth-pqc-standardization-conference/efficient%20threshold%20ml-dsa%20up%20to%206%20parties.pdf));
TALUS (2026 preprint) gets one-round online signing
([arXiv](https://arxiv.org/pdf/2603.22109)). No standard, no audited production
implementation, small party counts. NIST's separate threshold-cryptography effort has
not produced a PQ threshold standard. **Do not plan on PQ threshold signing today.**

**Transferable lesson + dowiz mapping.** Three concrete mechanisms: (1) DKG so no key
ever exists whole; (2) threshold strictly above any single organization's custody
footprint (decorrelated holders — different operators, different infra); (3) scheme
simplicity as a security control (FROST over GG20). For dowiz's hybrid stance, the
practical composition is **threshold-FROST-Ed25519 ⊕ single-party ML-DSA**: the classical
half gets t-of-n distribution now; the PQ half stays single-signer until threshold ML-DSA
matures, with the hybrid AND-composition ensuring the PQ signature still gates validity.
bebop2's per-actor PQ identity already supports this shape; the threshold layer itself
is new capability.

---

## 4. L2 sequencer decentralization — what actually shipped

**Honest census.** As of early 2026, every major L2 — Arbitrum, Base, OP Mainnet,
zkSync Era, Linea, Scroll — still runs a centralized sequencer
([eco.com](https://eco.com/support/en/articles/14798711-ethereum-l2-sequencers-centralized-today-decentralized-tomorrow)).
The genuine exceptions: **Metis** runs a rotating decentralized sequencer pool
(Tendermint-style consensus + threshold signing of batches) and accepted measurably
lower TPS from the group-signature round
([Four Pillars analysis](https://4pillars.io/en/articles/metis-the-first-ever-decentralized-layer2)).
**Espresso** runs Mainnet 0: ~100 permissioned geo-distributed nodes on HotShot
consensus, 20M+ transactions processed, but full permissionless PoS still pending and
major-rollup adoption thin ([Espresso](https://hackmd.io/@EspressoSystems/EspressoSequencer)).
**Astria**, a dedicated shared-sequencer company, shut down in December 2025 — weak
demand made explicit ([Aligned year-review](https://blog.alignedlayer.com/the-year-of-zkvm-real-time-proving-milestones-present-and-future/)).

**Why decentralization didn't happen (the falsifiable reason).** Not laziness: the
centralized sequencer is tolerable because rollups ship an **L1 escape hatch** (forced
inclusion): a censoring or dead sequencer delays you, it cannot steal from or permanently
censor you. Sequencer revenue (fees + MEV) then makes operators economically unwilling
to distribute a role users don't demand distributing. Prediction this makes: projects
decentralize the sequencer only when it is their founding differentiator (Metis) —
which is what the census shows.

**Transferable lesson.** Don't decentralize the fast path; **guarantee the slow path**.
The proven pattern is: efficient, even single-operator, fast path + a censorship-resistant
fallback (forced inclusion) that bounds the harm of fast-path failure to *latency*, never
*safety*. For the mesh: a hub/relay node may order messages for throughput, as long as any
node can always commit to its own local log and sync peer-to-peer — the mesh-wide gossip
path is the escape hatch. Second lesson: where ordering *is* distributed, Metis paid for
it with threshold-signing latency — budget that cost consciously.

**dowiz convergence.** Strong and already built: the kernel is local-first — each node
appends to its own content-addressed event log and syncs (MESH-06,
`kernel/src/event_log.rs`). dowiz never had a central sequencer to decentralize; the
architecture starts from the escape-hatch side. The lesson is confirmation, plus a
warning against ever introducing a mandatory ordering service.

---

## 5. Content addressing and data availability (IPFS / Filecoin / Arweave)

**What's proven durable.** Hash-as-identity itself: CIDs give integrity verification,
deduplication, and location-independence, and this layer has not been the failure mode
anywhere. What breaks at scale is **discovery and availability**, which are separate
layers. The SIGCOMM 2022 measurement study (Trautwein et al.) found IPFS retrievals take
≥4× the equivalent HTTPS request across all tested AWS regions (DHT content routing is
the bottleneck), while actual content transfer is fine — >99% of 0.5MB exchanges complete
in <1.26s once a provider is found
([paper](https://research.protocol.ai/publications/design-and-evaluation-of-ipfs-a-storage-layer-for-the-decentralized-web/trautwein2022.pdf)).

**The availability lesson.** IPFS gives **no** persistence guarantee: unpinned content is
garbage-collected, and nothing repairs or re-replicates data
([IPFS docs](https://docs.ipfs.tech/concepts/persistence/)). Availability is always an
explicit economic/policy layer bolted on top: pinning services, Filecoin storage deals
(paid persistence, but retrieval slow/inconsistent), or Arweave's pay-once endowment.
Arweave's model — ~200-year storage endowment, claimed ≥20 replicas, endowment untouched
in ~7 years of operation — has held so far, but the sources are largely
Arweave-affiliated and the model is a bet that storage cost/GB keeps declining; if that
assumption breaks, the guarantee breaks
([ArDrive](https://ardrive.io/can-data-really-be-stored-forever),
[endowment simulation](https://hackmd.io/@YTnQkIXiSgyoU2Gnfq6BGg/ry5zORx7j)).

**Transferable lessons, sharpened for dowiz.** dowiz already runs this pattern:
`sha3_256` content-addressed event log (`kernel/src/event_log.rs`), `BlockStore`
(`kernel/src/backup.rs`), and a demote-never-delete attic — which is precisely pinning
semantics (explicit root set; nothing implicit keeps data alive). Three sharpening points
from the field evidence: (1) **never conflate integrity with availability** — write the
replication policy (dowiz's 3-2-1-1-0 backup doctrine) into the protocol layer as
explicit per-object pin/replica counts, not as ops convention; (2) **skip the global
DHT** — the measured 4× latency penalty is the price of open-world discovery; a mesh
with known/introduced peers should use direct provider hints and gossip, which removes
IPFS's worst layer entirely; (3) **GC needs a root-set contract** — the attic's
tier/TTL-demote design is correct; the failure mode to guard is an object reachable
from a capability but absent from every node's pin set.

---

## 6. Capability-based security outside crypto

**Track record (the non-blockchain lineage).** Object-capability security predates
blockchains by decades (Dennis & Van Horn 1966 → KeyKOS → E language/CapTP → seL4 →
Cap'n Proto). Two hard pieces of evidence:

- **seL4**: the first OS kernel with machine-checked functional-correctness proof
  (Isabelle/HOL, refinement from abstract spec to code — Klein et al., SOSP 2009 /
  [CACM](https://cacm.acm.org/research/sel4-formal-verification-of-an-operating-system-kernel/)),
  in which *all* authority is mediated by unforgeable capabilities. In DARPA's HACMS
  program, a professional red team was given full access to a non-critical partition
  (camera subsystem) of Boeing's Unmanned Little Bird helicopter **in flight** and could
  not escalate to the flight-critical partition
  ([DARPA HACMS](https://www.darpa.mil/news/resources/case-studies/hacms),
  [Quanta](https://www.quantamagazine.org/formal-verification-creates-hacker-proof-code-20160920/)).
  That is a live adversarial test of "possession of capability = authority; absence =
  mathematically stuck."
- **Cloudflare Workers**: the sandbox↔supervisor boundary of one of the largest edge
  compute platforms runs on Cap'n Proto RPC, whose security model is CapTP from the E
  language; Workers RPC exposes only the object references explicitly handed across the
  boundary ([Workers security model](https://developers.cloudflare.com/workers/reference/security-model/),
  [Workers RPC visibility](https://developers.cloudflare.com/workers/runtime-apis/rpc/visibility/)).
  Same lineage now extended to browsers via capnweb
  ([Cloudflare blog](https://blog.cloudflare.com/capnweb-javascript-rpc-library/)).

**What capabilities structurally prevent.** Ambient authority and the confused deputy:
you cannot exercise access you were not handed, and you can hand out *attenuated*
capabilities (narrower rights, revocable) rather than identity + ACL lookups. Contrast
with reputation systems: scores are sybil-inflatable, gameable, and centralizing (someone
maintains the score). A capability has no global state to game.

**Transferable lesson + convergence.** This is direct, independent, non-crypto-industry
validation of dowiz's already-decided M12 stance and bebop2's "trust = signed capability,
NEVER reputation" (SOVEREIGN-EVENT-EXCHANGE blueprint) — fully converged on the
principle. The genuinely new, concrete piece to lift is **CapTP-style capability passing
over the mesh's bridges**: agents exchange attenuated, revocable object references
(promise pipelining included) rather than "node X has trust score Y." Cap'n Proto's
serialization + RPC is Rust-implementable and its model is exactly the bridge semantics
an agentic mesh needs.

---

## 7. Real-time verification: signatures, not SNARKs

**Ed25519.** The original ed25519 paper reports 71,000 verifications/second with batch
verification on a quad-core 2.4GHz Westmere (~134k cycles/signature in batches of 64),
with max latency <4ms ([Bernstein et al.](https://ed25519.cr.yp.to/ed25519-20110926.pdf)).
On 2026 server cores, order of magnitude: **tens of microseconds per verify, ~10⁴–10⁵
verifies/sec/core**, roughly 2× more with batching.

**ML-DSA (what bebop2 uses).** Verification is matrix-vector multiplication + hashing —
no rejection sampling (that cost is signing-side). Secondary sources put ML-DSA-65
verification at roughly 100–200µs and Level-5 around ~75µs on modern hardware
([DEV/EncryptionConsulting summaries](https://dev.to/abraham_arellanotavara_7/choosing-between-ml-kem-and-ml-dsa-for-your-post-quantum-migration-part-2-4dip)); this
is consistent with published Dilithium cycle counts (~150–250k cycles ≈ 50–100µs at
3GHz). I could not find a single authoritative cross-platform benchmark to cite for one
exact number — treat it as **order of magnitude 0.1ms, i.e. thousands of verifies per
second per core**. Conclusion: **ML-DSA verification is unambiguously fast enough for a
real-time hot path.** The real PQ tax is wire size, not CPU: ML-DSA-65 signatures are
3,309 bytes and public keys 1,952 bytes (FIPS 204) versus Ed25519's 64/32 — a hybrid
ML-DSA⊕Ed25519 envelope costs ~3.4KB of signature per message. For a chatty agent mesh,
bandwidth and message framing dominate, not verify latency.

**Aggregation.** Ethereum's beacon chain scales to hundreds of thousands of validators
because BLS signatures aggregate — many attestations collapse to one verification
([eth2book](https://eth2book.info/latest/part2/building_blocks/signatures/)). But BLS
pairings are ~ms-scale and BLS is not post-quantum; **ML-DSA has no practical
aggregation scheme**. So the mesh's scaling tool is *batch verification* (amortize
Ed25519) and *envelope design* (one hybrid signature over a batch/checkpoint of events,
not per-tiny-event), not signature aggregation.

> **Correction (2026-07-17, post-F1 batch-verify fix):** the general amortization property above
> is real, but as implemented in bebop2 (`sign.rs::verify_batch`, hardened against SSR-2020
> mixed-order forgeries by confirming every batch-accept with per-item single verifies) batch
> verification amortizes nothing — batch-accept costs ≥ N singles (measured 3.26× for N=64, bebop
> `docs/ledger/crypto-bench.jsonl`). Of the two scaling tools named here, only *envelope design*
> currently delivers; batch verification survives as a sound fast-reject. See B4 §2.3's correction
> for the full accounting.

**Transferable lesson.** Real-time verifiability for the mesh is a signature-systems
problem: signed claims verified in ~0.1ms are the hot path; validity proofs (Section 2)
are the periodic checkpoint layer; nothing in between is currently buildable. dowiz/bebop2's
hybrid choice is already the right primitive — the open engineering is batching and
size-budgeting, both measurable.

---

## Synthesis: ranked transferable patterns

| # | Pattern (mechanism, not slogan) | dowiz/bebop2 status |
|---|---|---|
| 1 | **Per-actor pluggable validation policy + sponsor/authorizer split** (ERC-4337's actual innovation) | New capability — hybrid signing exists, but as one fixed policy |
| 2 | **Verification asymmetry**: prover pays, any consumer checks in O(ms), no honest-watcher liveness assumption | Partially new — signatures yes; proof-carrying checkpoints new; reject fraud-proof designs |
| 3 | **Capability passing over bridges (CapTP lineage)**: attenuated, revocable, unforgeable references; no reputation state | Principle fully converged (M12); the RPC-level mechanism is the new build |
| 4 | **DKG + decorrelated custody + simple audited scheme** (FROST); PQ threshold not ready — hybridize: threshold classical ⊕ single-party PQ | New capability; composes with existing hybrid signing |
| 5 | **Fast path centralizable iff slow escape hatch guaranteed** (rollup forced-inclusion lesson) | Already built — local-first log *is* the escape hatch |
| 6 | **Integrity ≠ availability**: content addressing needs an explicit pin/replication contract; skip global DHT discovery | Already built (sha3_256 log, BlockStore, attic pinning); sharpen replication policy into protocol |

**Anti-recommendations (evidence-backed):** no per-message ZK proofs (10s/GPU-fleet
proving vs 0.1ms signature verify); no fraud-proof/challenge-window constructions
(liveness assumption a sparse mesh can't meet); no reputation-based trust (sybil-gameable,
contradicts the proven capability lineage); no GG18/GG20-class complex MPC (TSSHOCK);
no dependence on PQ threshold signatures before a standard exists.

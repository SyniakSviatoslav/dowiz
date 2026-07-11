# Integration-Ready Research: 3 Resources

## 1) Princess Pi — Pseudonymous Attribution Proofs
*Source: codex.churchofmalware.org/researchers/princess_pi/ (Gitea: PrincessPi/Encrypt-Share-Attribution).*

**(a) What it is.** A per-round scheme to distribute files pseudonymously with *two independent* attribution proofs:
- Generate a fresh **ED25519 SSH key** (`ssh-ed25519`) each round.
- Prompt an **attribution passphrase/message**; compute `SHA512(<attribution_pass> ‖ <inner.7z>)` and store the tag in the **outer** 7Zip layer.
- Sign the **inner** 7Zip archive with the SSH private key; store the signature in the outer layer.
- Store `SHA512` checksums of every file in the outer layer.
- Outer archive optionally **AES-CBC-256** (`-mem=AES256`) + PBKDF2/SHA256, with filenames encrypted; optional 32B random padding to break signatures.

*Proof A (signature):* any node verifies `ssh-keygen -Y verify` with the published pubkey → proves possession of the private key (authorship).
*Proof B (passphrase):* originator may later reveal the passphrase; verifier recomputes `SHA512(pass ‖ inner.7z)` → proves the *attribute* without exposing the key.

**(b) Integration point in bebop — pseudonymous Proof-of-Delivery.** Courier's pubkey *is* their pseudonymous ID; real PII never enters the archive. Receipt = inner.7z containing canonical `{order_id=X, timestamp=T, geohash, photo_hash}`. Courier signs it with an ephemeral `ssh-ed25519` key; nodes verify signature (proves the key-holder completed delivery). The passphrase = a *stable reputation secret*: courier reveals it only selectively to link deliveries to one pseudonym without doxxing. PII binding stays sealed server-side under dispute.

**(c) RED FLAGS / avoid.**
- 7Zip default is weak ZipCrypto — MUST use `-mem=AES256`; otherwise the "outer layer" is trivially readable.
- `SHA512(pass ‖ archive)` means passphrase reuse across rounds links deliveries — rotate per round or accept the linkage deliberately.
- Signature proves key possession, NOT legal identity. For chargebacks, bind the pubkey to PII in a sealed envelope, not in the archive.
- HIBP/entropy/cracklib checks need network at keygen — disable for offline couriers or pre-seed a wordlist.
- `ssh-ed25519` (OpenSSH format) ≠ raw `ed25519` — pick one and document it for cross-node verify.

## 2) arXiv 2104.03902 — "The Autodidactic Universe" (Alexander, Cunningham, Lanier, Smolin et al.)
*Note: the paper the task cites is titled "The Autodidactic Universe," not "Self-Taught Learning Machine" — same PDF.*

**(a) What it is.** Core claim: the Universe can *learn its own laws* — there is no external supervisor ("autodidactic"). Matrix models of gauge/gravity are placed in correspondence with neural-net learning machines (RBM/cyclic RNN). "Learning" = a system alters internal processes to better exploit flows through it. The objective is **internal** (survival, maximizing *variety*, precedence), not an externally supplied label — that is the self-supervised/self-taught part. A "consequencer" accumulates past signal that is disproportionately influential on the future.

**(b) Integration point in bebop — L5 self-learning layer.** bebop's governor already has PID/ICIR/resonance/thermo. Borrow 3 mechanisms from Ch.4:
1. **Variety maximization (§4.4):** reward structural diversity of candidate routes so L5 doesn't collapse to one corridor (anti-overfit exploration).
2. **Renormalization-group learning (§4.1):** coarse-grain fine per-node delivery telemetry into protocol-level weight deltas (aggregation, not raw replay).
3. **Precedence (§4.2):** weight parameter updates by recency — exponential-decay on delivery outcomes (reward = on-time margin − cost). No external oracle; autodidactic.

**(c) RED FLAGS / avoid.**
- It is a *correspondence*, not an equivalence — do not claim physical-law optimality; treat it as an analogy/objective shape only.
- "Maximize variety" unbounded ⇒ routing chaos; bound it with the existing resonance/thermo guards.
- Don't import the cosmological scale — keep it as a local weight-update rule on live flow.

## 3) Agentic Design Patterns (Gulli, 21 ch / 7 apps)
*Source: github.com/evoiz/Agentic-Design-Patterns (book PDF + notebooks).*

**(a) What it is.** Canonical catalog: Core (Prompt Chaining, Routing, Parallelization, Reflection, Tool Use, Planning, Multi-Agent); Advanced (Memory, Learning/Adaptation, MCP, Goal Setting); Production (Exception Handling/Recovery, HITL, RAG); Enterprise (A2A, Resource-Aware, Reasoning, Guardrails, Eval/Monitoring, Prioritization, Exploration/Discovery).

**(b) Integration point in bebop.** **Adopt** at dispatch layer: Planning-lite, Reflection (after-action on failed deliveries), Exception/Recovery, Guardrails, Prioritization, Resource-Aware (maps to L5 governor). A2A is *relevant* but should ride bebop's existing field/L5/stabilizer runtime, not a new stack.

**(c) RED FLAGS / avoid reimplementing.**
- **Multi-Agent collaborative swarms:** coordination overhead, deadlock risk, overkill for dispatch — already handled by field runtime.
- **HITL in hot path:** keep only at dispute/arbitration.
- **RAG / Coding-agents (App G):** irrelevant to deterministic dispatch DB.

**Synthesis:** bebop should *adopt* Planning-lite + Reflection (post-hoc) + Exception/Recovery + Guardrails + Prioritization as dispatch behaviors, and *delegate* routing, resource control, and inter-node messaging to its existing field/L5/stabilizer runtime — explicitly NOT reimplementing Multi-Agent swarms, HITL-loop, or RAG.

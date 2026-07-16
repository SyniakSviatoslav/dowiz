# BLUEPRINT H3 — INDEPENDENT PEER-DRIVEN INTEGRITY PROBE

> **Anchor:** Gender **V-3** (`PRINCIPLE-7-GENDER.md` §2) — part of **RC-2** "verification organs
> without independent teeth" (`HERMETIC-ARCHITECTURE-PRINCIPLES.md` §RC-2, ranked-findings **row 15**,
> MED). The audited party still originates its own breach evidence; the active *detection* half is not
> independent.
> **Depends on / builds ON TOP OF (does not duplicate):**
> - **G9 breach-witness primitives in `kernel/src/hydra.rs`** — `BreachAlert` (+ `to_bytes`/`witness_event_id`),
>   `ingest_peer_breach`, `boot_verify`, `integrity_check`, `OrganismState`, `BREACH_WITNESS_ACTOR`,
>   `EventLog::append_raw`. Consumed as evidence *content* and *sink*; no new WORM machinery invented.
> - **P06 key_V** — the probe *verdict* is signed by the probing peer's key_V-role anchor (row 15:
>   "routed through key_V (#2)"). **P03** — canonical ML-DSA-65 signs challenge/response/verdict on the wire.
> - **P09 §2/§4** — wire + MST gossip carries probe frames and gossips breach-witness rows, so a peer can
>   cross-check a claimed log-tip against independently-received rows. **P10 §3** — the operator kill-switch
>   an operator may invoke after a failed probe.
> **Sequencing (hard):** `hydra.rs` had heavy G9 activity **this session** (`1701eabd1`, `5403a3eff`,
> `fab17275a`, `b5b583e49`, `d0e71cec9`) — the **same file**. Build **after** all in-flight G9 work lands,
> **not in parallel**; do not open a concurrent edit on `hydra.rs`.
> **Planning artifact only. It writes no code and edits no `.rs` file.**

---

## 0. The problem (scoped to what live re-verification shows is genuinely still open)

The audit's baseline framing ("a peer can only ingest a breach the compromised node *chose* to
broadcast") is now **partially stale** — the G9 work landed the receptive and forge-proof halves. The
genuine remainder, re-verified line-by-line against the current `hydra.rs`, is narrower and precise:

**There is no mechanism by which a peer proactively demands a proof of integrity from a silent node, and
treats the absence of that proof as evidence.** Every breach path in the organism is **self-triggered**:
`raise_breach_alarm` returns `None` unless `self.state == Locked` (`:291-293`), and `state` only flips to
`Locked` when the node runs `integrity_check()` **on itself** (`:180-195`). `boot_verify` is `&self` over
the node's own `base_edges` (`:252-264`). `ingest_peer_breach` is purely receptive — it records an alert
that *arrives* (`:329-343`). A core that is compromised, stays reachable at the transport layer (so P09's
DSU partition detection sees it as healthy), and simply **never calls `integrity_check`/`raise_breach_alarm`**
produces zero evidence for any peer to ingest. This is the "silence-before-witnessing" gap named verbatim
in the finding (`hydra.rs:283` anti-silent-heal covers *denial after* witnessing, not *silence before*).

---

## 1. Current-state evidence — what G9 already provides (cited, not redesigned) and the exact gap

**Already solved by G9 (do not rebuild):**
- **(a) Un-forgeable alert.** `BreachAlert::witness_event_id` (`:135-147`) is a content-addressed digest
  fully determined by `node_id + group_size`; a receiver re-derives it *without trusting the sender*
  (`hydra_breach_alert_receiver_verifiable`, `:617-669`). ML-DSA (transport) signs the 40 canonical bytes.
- **(b) Un-suppressible once raised.** `raise_breach_alarm` self-witnesses an immutable row into the WORM
  log (`:297-310`) — a tampered core "can NEVER silently heal or deny it was compromised" (anti-silent-heal);
  no per-event consent, unbounded fan-out (`hydra_breach_alarm_unbounded_on_tamper`, `:532-570`).
- **(c) Hub convergence.** `ingest_peer_breach` durably records a verified peer breach, idempotent on replay
  (`hydra_ingest_peer_breach_converges_hub`, `:577-611`).

**The exact remaining gap:** (a)-(c) are all the **receptive** half — they process a signal the audited node
emitted first. The **active detection half is not independent**: it fires only from the audited node's own
`integrity_check`. There is no `IntegrityProbe` a peer sends, no on-demand `attest_integrity` the queried
node must answer, and no timeout turning silence into a recorded witness. RC-2's prescription (row 15):
*"Independent peer-driven integrity probe/attestation, routed through key_V."*

---

## 2. Target-state design — a peer-initiated integrity probe

A three-message exchange that moves the *initiative* to the auditor while reusing G9 as the evidence sink.
The kernel stays network/RNG/serde-free (same discipline as `BreachAlert`): it produces/consumes canonical
byte payloads; the transport layer (P03/P09) does ML-DSA signing, nonces, and deadlines.

**(1) Challenge — `IntegrityProbe` (peer A → node B).** Canonical bytes `prober_id(32) ‖ nonce(32) ‖
deadline_unix(8)`. A generates a fresh `nonce`, signs with A's ML-DSA anchor, sends over the P09 wire. This
is the **independent active half V-3 says is missing** — A probes on *A's* schedule, not B's.

**(2) Response — `IntegrityAttestation` (node B → peer A).** A new pure kernel method (planning-named)
`attest_integrity(nonce) -> IntegrityAttestation` packages, in canonical fixed layout:
`echoed_nonce(32) ‖ node_id(32) ‖ state_tag(1: Live/Locked) ‖ rho_bits(8, `boot_verify` reading) ‖
worm_tip(32, `log().tip()`)`. B signs these bytes with **B's own anchor** (transport). The `state_tag`/`rho`
come straight from the existing `integrity_check`/`boot_verify` (consumed, not replaced); the `worm_tip`
binds the attestation to B's *actual current log*, so B cannot present a healthy ρ while hiding
breach-witness rows it already wrote.

**(3) Verdict at A — three outcomes, each producing durable evidence:**
- **Attests-Live.** Valid signature, echoed nonce matches, `state==Live`, `rho<1` → A records "B attested
  Live at T" (a positive-liveness witness). *Attested*, not *proven clean* — see §5.
- **Attests-Locked.** `state==Locked` or `rho>=1` → B is self-reporting compromise. A feeds it straight into
  the **existing** `ingest_peer_breach` path — the probe and the broadcast-alarm converge on one WORM sink.
- **Silence / malformed / bad-nonce / bad-signature (the RED case).** No well-formed, in-deadline, correctly
  signed response → A appends a **probe-timeout witness row** via `append_raw` under `BREACH_WITNESS_ACTOR`
  with a distinct payload tag (e.g. `0x01 ‖ target_id ‖ nonce`), so it is distinguishable from a breach
  row yet lives in the same immutable log. **Silence is now evidence, not a false-negative "clean."**

**Independence routing (row 15).** B's attestation is unavoidably a self-report signed by B (the hard
limit, §5). A's *verdict* — including the timeout witness — is signed by **A's key_V-role anchor** (P06), a
forge-proof independent record that "A probed B and B failed/attested-Locked" that B cannot suppress.
**Cross-check teeth:** since P09 gossips breach-witness rows, A compares B's attested `worm_tip` against rows
A already received — a B that self-witnessed a breach then attests `Live` must omit that row, and the
omission is detectable against the gossiped content-id (same trick as `witness_event_id`).

**Composition:** reuses `integrity_check`/`boot_verify` as attestation *content*; `ingest_peer_breach` as
the Locked-path *sink*; `BREACH_WITNESS_ACTOR`/`append_raw` for the timeout row; leaves signing/nonce/
deadline at the ML-DSA transport. It **adds an active initiator**, not a new evidence store.

---

## 3. Migration steps

1. **[GATE]** Confirm all in-flight G9 commits have landed and `hydra.rs` is quiescent; confirm P03 canonical
   ML-DSA and P06 key_V role exist (or stub the key_V-role anchor behind the P06 seam).
2. Add the pure kernel primitives to `hydra.rs` (after G9): `IntegrityProbe`/`IntegrityAttestation` structs
   with fixed-layout `to_bytes`/`from_bytes` (fail-closed on bad length, mirroring `BreachAlert`), and
   `Hydra::attest_integrity(nonce)` (pure, reads `state`/`boot_verify`/`log().tip()`).
3. Add receiver-side `verify_attestation(&self, probe, attestation) -> ProbeVerdict` (pure: nonce echo,
   layout, `rho<1`, `state` tag; re-derives content-addressed pieces without trusting the sender).
4. Add `record_probe_timeout(target_id, nonce)` — appends the `0x01`-tagged witness via `append_raw`
   (idempotent, content-addressed like `ingest_peer_breach`).
5. Wire the Locked verdict to the **existing** `ingest_peer_breach` (no new sink).
6. Transport/mesh layer (P09): send `IntegrityProbe`, enforce the deadline, ML-DSA-sign each leg, sign A's
   verdict with the key_V-role anchor; gossip timeout/Locked witnesses over the MST overlay.
7. Tests + the §4 acceptance suite; keep the kernel std-only/egress-free.

---

## 4. Acceptance criteria (numbered, falsifiable)

1. **GREEN (attests-Live).** A probes a healthy B; B returns a signed, in-deadline `IntegrityAttestation`
   with echoed nonce, `state==Live`, `rho<1`; A's verdict = `Live`, signed by A's key_V-role anchor.
2. **GREEN (attests-Locked → converges).** A probes a B whose `integrity_check` returned `Locked`; the
   attestation's `state_tag==Locked`; A routes it through `ingest_peer_breach`, and B is now durably "burnt"
   in A's WORM log (same row a broadcast alarm would have produced).
3. **RED (silence → evidence, not false-clean).** A probes a B that never responds by `deadline`. A records a
   `0x01`-tagged probe-timeout witness under `BREACH_WITNESS_ACTOR`; the verdict is **NOT** `Live`. Asserted:
   no code path lets an absent response resolve to a clean result.
4. **RED (stale-nonce replay).** A replayed *healthy* attestation carrying a **prior** nonce fails the echo
   check and is treated as silence (criterion 3), not accepted as fresh liveness.
5. **RED (forged Live over hidden breach).** A B that self-witnessed a breach row but attests `Live` with a
   `worm_tip` omitting it is caught: A's gossiped copy of the row yields a content-id absent from the attested
   tip → mismatch → treated as failed probe.
6. **Forge-proof verdict.** A 1-bit flip in any signed leg (probe, attestation, or verdict) fails ML-DSA
   verification fail-closed; `from_bytes` rejects any non-fixed-length payload.
7. **No-rebuild / composition.** The Locked path calls the existing `ingest_peer_breach` (grep proves no
   second breach sink); the kernel additions remain network/RNG/serde-free.

---

## 5. Residual limitation (honest) + what this unblocks

This probe converts the audited node's **silence into recorded evidence** and moves the **detection
initiative** to an independent party — but it cannot make a self-report *true*. B's `IntegrityAttestation`
is signed by **B's own anchor**; a node compromised in a way that leaves its signing key, its `base_edges`
spectrum, and its WORM log intact and controllable can sign a truthful-*looking* `Live, rho<1` attestation.
This is exactly the V1 **"identity ≠ person"** residual (`BLUEPRINT-P06 §8`): the response is bound to the
node's *identity*, not to an unforgeable ground truth of its integrity. Binding `rho` + `worm_tip` raises the
forgery cost — a lie must also preserve the spectral baseline **and** survive cross-check against gossiped
rows — but a compromise that shifts nothing observable is undetectable by **any** self-report probe, and we
do not claim otherwise. What it genuinely closes, falsifiably: (1) silence-before-witnessing — a dark or
refusing node is now a recorded failed-probe, not a false clean (criterion 3); (2) the *initiative*
dependence — detection no longer waits on the audited party's schedule; (3) the self-witnessed-then-denied
case, via the gossiped-tip cross-check (criterion 5). **Unblocks:** the operator kill-switch (P10 §3) gains
an independent, forge-proof trigger — a repeated failed probe is a signed verdict an operator can act on —
and RC-2's V-3 independence defect is closed to the exact boundary only P06's true second party
(human/decorrelated model taking the key_V role) can push further.

---

*Blueprint H3 complete. Scope: the genuine remainder of Gender V-3 after live G9 re-verification — an
independent peer-driven integrity probe that turns silence into evidence, composing on top of the G9
BreachAlert/`ingest_peer_breach`/`append_raw` primitives (not duplicating them) and routing its verdict
through P06's key_V. Sequenced strictly after in-flight G9 work on the shared `hydra.rs`. No code written by
this document.*

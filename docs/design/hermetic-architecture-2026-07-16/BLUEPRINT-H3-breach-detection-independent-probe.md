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

---

## 6. Planning-protocol completion appendix (2026-07-17, decorrelated pass)

### (i) Citation verification + new grounding (priority pass — this blueprint had one citation in 166 lines)

**Headline finding: H3's hard sequencing gate is now met.** §0's header said "build after all in-flight
G9 work lands, not in parallel." `git log --oneline 4dec04218..HEAD -- kernel/src/hydra.rs` is **empty**
— H1 landed (`4dec04218`, 2026-07-16T22:21:42Z) and `hydra.rs` has been quiescent ever since (confirmed
against current HEAD `cc3d5c916`, 2026-07-17T02:26Z). H3's own step 1 gate ("confirm all in-flight G9
commits have landed and `hydra.rs` is quiescent") is satisfied for real, not just asserted — and, as an
extra load-bearing fact this blueprint's own text anticipated but could not yet confirm: **H3 must build
against the *post-H1* API**, because H1 changed exactly the two functions H3's design calls
(`raise_breach_alarm`, `ingest_peer_breach`) from `Option<BreachAlert>`/`()` to
`Result<Option<BreachAlert>, StoreError>`/`Result<(), StoreError>`. This blueprint's §3.5 design
("Wire the Locked verdict to the existing `ingest_peer_breach`") must therefore `?`/handle a
`StoreError`, not treat the call as infallible — a concrete, previously-unstated correction this pass
supplies.

**Every citation re-verified and corrected against live `hydra.rs` (all shifted ~+130 lines from the
pre-H1 baseline this blueprint was written against):**

| Blueprint claim | Original cite | **Live, corrected cite (this pass)** |
|---|---|---|
| `raise_breach_alarm` self-triggered only when `Locked` | `:291-293` | **`hydra.rs:287-294`** (fn header `:287-291`; the guard `if self.state != OrganismState::Locked { return Ok(None); }` at `:292-294`) — content confirmed identical, only the gate now returns `Result` |
| `integrity_check` is `&mut self` over own baseline | `:180-195` | **`hydra.rs:180-195`** — coincidentally unchanged (H1's edits landed below this fn) |
| `boot_verify` is `&self` over own `base_edges` | `:252-264` | **`hydra.rs:253-265`** |
| `ingest_peer_breach` purely receptive | `:329-343` | **`hydra.rs:332-348`** — now returns `Result<(), StoreError>` (H1 §2.4), confirmed |
| `BreachAlert::witness_event_id` content-addressed digest | `:135-147` | **`hydra.rs:128-148`** |
| receiver-verifiable test | `hydra_breach_alert_receiver_verifiable, :617-669` | **`hydra.rs:625-677`** |
| `raise_breach_alarm`/`:286-315` | `:286-315` | **`hydra.rs:272-318`** (doc comment `:272-286`, body `:287-318`) |
| unbounded-fan-out test | `:532-570` | **`hydra.rs:538-578`** (`hydra_breach_alarm_unbounded_on_tamper`) |
| hub-convergence test | `:577-611` | **`hydra.rs:585-619`** (`hydra_ingest_peer_breach_converges_hub`) |
| `hydra.rs:283` anti-silent-heal prose | `:283` | **`hydra.rs:282-284`** ("tampered core can NEVER silently 'heal' or deny it was compromised (anti-silent-heal)") — content confirmed intact at the new location |

**New grounding this pass adds (none of this was cited before — the "missing evidence research" this
pass was tasked with):**
- `OrganismState` enum: **`hydra.rs:75-79`** (`Live`/`Locked`, exactly as §2's design assumes).
- `BreachAlert` struct + wire codec: **`hydra.rs:87-120`** (`to_bytes`/`from_bytes`, 40-byte fixed
  layout, fails closed on bad length — the exact pattern H3 §3 step 2 says to mirror for its own
  `IntegrityProbe`/`IntegrityAttestation` structs).
- `BREACH_WITNESS_ACTOR`: **`hydra.rs:126`** — the sentinel actor key H3 §3 step 4 reuses for its
  `0x01`-tagged probe-timeout witness row.
- `Hydra<S: EventStore>` struct: **`hydra.rs:153-158`**; `commit`: **`hydra.rs:214-245`**; `log()`:
  **`hydra.rs:268-270`**.
- `EventLog::append_raw` signature (post-H1): **`kernel/src/event_log.rs:321`** —
  `Result<AppendOutcome, StoreError>`, confirming the `?` H3's own `attest_integrity`/
  `record_probe_timeout` (not yet written) will need to propagate.
- Confirmed via `grep -rn "attest\|IntegrityProbe\|IntegrityAttestation\|record_probe_timeout\|verify_attestation" kernel/src/ tools/`
  (excluding `node_modules`): **zero hits**. None of H3's proposed primitives exist yet anywhere in the
  repo — the design is still 100% prospective, unlike H1/H2 (see their appendices).

**The instructed check on the native Rust telemetry ports (`4519bd7ff`, `cc3d5c916`) — result: NOT
relevant, verified rather than assumed.** I read both commits' diffs and the new crates
(`tools/skillspector-rs` — a static source-pattern security scanner ported from Python;
`tools/telemetry/native-trackers`, `native-ser`, `hetzner-exporter`, `swarm-proof` — host
metrics/wire-format/economic-calc tooling; `tools/telemetry/topics` — a Telegram aggregator). None of
them contain `breach`, `integrity_check`, `attest`, or `Probe` (grep-confirmed above); they are an
**ops/observability/security-linting surface**, entirely disjoint from H3's kernel-internal
peer-to-peer mesh breach-detection design. H3's own text never actually cited these files — it was
already correctly scoped to `hydra.rs`/G9/P03/P06/P09 before this pass. **Conclusion: the "H3 claims may
be stale due to the telemetry ports" hypothesis does not hold** — verified negative, not silently
dropped, per the same discipline as the rest of this pass.
- Confirmed via `kernel/Cargo.toml` (full file read): **no crypto/ML-DSA crate dependency exists in
  dowiz's kernel** — P03's actual PQ crypto work lives in a **different repo**
  (`/root/bebop-repo/bebop2/`, crates `core`/`proto-cap`/`proto-crypto`/`proto-wire`, per
  `BLUEPRINT-P03-pq-trust-root-hardening.md` header, re-read this pass) — not dowiz kernel at all. This
  sharpens H3's own gate: **the pure-kernel primitives (§3 steps 2-5: `IntegrityProbe`/
  `IntegrityAttestation` structs, `attest_integrity`, `verify_attestation`, `record_probe_timeout`) have
  NO live dependency on P03/P06/P09 and are buildable today** (H1's gate is clear); only **§3 step 6**
  (ML-DSA signing, key_V-role anchor, P09 gossip transport) is blocked, and blocked cross-*repo*, not
  merely cross-blueprint — P06/P09 remain blueprint-only in dowiz (confirmed: only `.md` files exist
  under `docs/design/sovereign-roadmap-2026-07-16/`, no implementing code).

### (ii) DECART judgment

**No DECART owed.** H3's design is explicit that the kernel stays "network/RNG/serde-free" (§2, §3
step 7) and reuses existing G9 primitives; the only crypto dependency it needs (ML-DSA signing) is
inherited from P03's already-scoped choice in a different repo, not a new choice H3 introduces. If P03's
ML-DSA crate choice was itself DECART'd, that DECART belongs to P03's blueprint (outside my assigned set
— `BLUEPRINT-P03-pq-trust-root-hardening.md` is not one of the four files I was asked to complete); H3
neither makes nor should make that choice itself.

### (iii) Per-blueprint 2-question doubt audit

**Q1 — concrete, unresolved doubts:**
1. **The sequencing correction above (H3 must `?`-propagate `StoreError` from the now-fallible
   `raise_breach_alarm`/`ingest_peer_breach`) is my own derivation, not something this blueprint's
   original text anticipated** — I did not find any line in H3 acknowledging the post-H1 signature
   change explicitly; it is a real, load-bearing gap this pass closes, but I did not go further and
   sketch what the resulting `Result`-threading through `verify_attestation`/`ProbeVerdict` should look
   like — that is real design work for the eventual build pass, not this appendix.
2. **P09's actual wire/gossip implementation status was not independently verified beyond "the
   blueprint file exists."** I confirmed `BLUEPRINT-P09-confidential-self-healing-wire.md` exists and
   re-read its header (Wave 2, "writes no code"), but did not grep bebop-repo or dowiz for a partial P09
   implementation that might already exist outside my search scope (I searched dowiz only, per my
   assignment; P09/mesh code plausibly lives in `/root/bebop-repo`, which I did not audit).
3. **I did not verify P06's actual current build status beyond "no `key_V`/`key_K` string appears in
   dowiz kernel/src."** This is a real grep result, not an assumption, but P06 (like P03) may have
   partial implementation in `/root/bebop-repo` that a dowiz-only grep would miss — I did not cross the
   repo boundary to check, since that repo is outside this task's assigned file set.
4. **The `git log --oneline 4dec04218..HEAD -- kernel/src/hydra.rs` quiescence check is a point-in-time
   fact (as of `cc3d5c916`)** — if further G9/hydra work lands between this appendix and an actual H3
   build pass, the gate re-closes; this appendix asserts quiescence NOW, not a permanent property, and
   nothing in H3's text (nor this appendix) re-checks it automatically before a future build starts.
5. **I did not attempt to actually write or compile a throwaway `IntegrityProbe`/`attest_integrity`
   stub** to confirm the design's claimed byte layout (`prober_id(32) ‖ nonce(32) ‖ deadline_unix(8)` /
   `echoed_nonce(32) ‖ node_id(32) ‖ state_tag(1) ‖ rho_bits(8) ‖ worm_tip(32)`) actually compiles
   cleanly against `BreachAlert`'s existing `to_bytes`/`from_bytes` pattern (`hydra.rs:97-120`) — the
   design reads as consistent with that template on inspection, but this is unexecuted, unlike H1/H2's
   citations which I ran as live tests.
6. **The residual-limitation section (§5) cites `BLUEPRINT-P06 §8`** for the "identity ≠ person"
   framing — I confirmed that exact phrase and section number exist in the live P06 file (`grep -n`,
   confirmed `identity != person` at line 168 and "## 8. Residual limitation" at line 304), but did not
   read P06 §8 in full to confirm the analogy H3 draws is faithful to P06's complete argument, only that
   the citation resolves to real content (Mentalism-style "does it resolve," not "is the argument
   sound").

**Q2 — biggest blind spot:** this blueprint's own header already names its sequencing constraint
precisely and conservatively ("do not open a concurrent edit on `hydra.rs`") — the actual risk this pass
surfaces is the **opposite** of what the header worried about: the gate has been *clear* for a full day
(since `4dec04218`) and nothing in the repo's structure notified anyone that H3 became buildable. A
blueprint that gates on "wait for X" but has no mechanism to announce "X happened" reproduces, in
miniature, exactly the Rhythm/RC-2 pattern (`AGENTS.md`'s own 2Q ritual; HERMETIC-ARCHITECTURE-PRINCIPLES.md
Finding 5) this entire hermetic-architecture arc exists to fix: a condition that must be *periodically
re-checked* to stay honest, currently checked only because this decorrelated pass happened to check it.

### (iv) Anu (logic) & Ananke (organization) check

**Anu.** The blueprint's central technical claim — that G9 already solved the receptive/forge-proof
halves and the sole genuine gap is the *initiative* half — is now derivable against the live,
post-H1 `hydra.rs` exactly as it was against the pre-H1 version the blueprint audited: `raise_breach_alarm`
is still gated on `self.state` (`:292`), `integrity_check` is still `&mut self` (`:180`),
`ingest_peer_breach` is still purely receptive (`:332`), and no `attest`/`Probe` primitive exists
anywhere (grep-confirmed). The problem statement survives H1's changes untouched — a good sign the
original diagnosis was addressing the right layer, not an artifact of a since-changed API. The one
place Anu was NOT fully closed by the original document — the post-H1 `Result`-propagation implication
— is supplied in this appendix (§(i) above), not left standing.

**Ananke.** The blueprint's own gate ("build after H1 lands... do not open a concurrent edit") is a
structurally sound *constraint statement*, but it is not a structurally sound *trigger*: nothing pings
whoever owns H3 when H1 lands. This is the same class of gap the umbrella plan's §6.Q2 already names for
the whole arc ("nothing makes the audit recur") and H4 (separately) proposes a partial, operator-gated
mitigation for at the plan-artifact layer. H3 has no equivalent even at the design level — its
diligence-reliance is: *a human or agent must think to re-check `git log -- hydra.rs` before starting
the build*. Naming it here (rather than silently trusting the header) is the concrete Ananke action this
pass takes; building a re-check mechanism is out of scope for a doc-only pass.

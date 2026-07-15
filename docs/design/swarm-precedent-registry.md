# Anglo-Saxon Precedent / Jury Registry for Swarm Governance

Common-law layer over the DECART-square rule (`docs/operating-model/integration-decart-rule.md`). Reuses the `telemetry` JSONL-ledger + Telegram-topic primitives (topic 267 Hermes, 291 Planning, 294 Benchmarks). No new deps.

## (A) Precedent store — `tools/telemetry/logs/precedent.jsonl`

One JSONL row per finalized ruling. Schema (all fields mandatory, machine-checkable):

```json
{
  "id": "P-2026-0715-0042",
  "question": "rustls+ring vs aws-lc-rs for wss transport?",
  "decart_table": "criterion|rustls|aws-lc-rs|...",   // canonical decart rows
  "winner": "rustls+ring",
  "evidence": ["c837442","hardened_verifier_rejects_self_signed_cert"],
  "date": "2026-07-15T10:02:00Z",
  "overturned": null,        // null | {by:"P-...", reason:"..."}
  "argued_rounds": 3,        // # of research rebuttal passes before ruling
  "jury": ["judge-α","judge-β","judge-γ"],
  "binding": true            // false = persuasive only
}
```

Append via `log_event precedent id=.. question=.. winner=.. ...` so it mirrors every other telemetry ledger and streams to topic 267.

## (B) `precedent` command — command flow

`precedent <new_question> [--decart <table>] [--jury 3]`

1. **Retrieve (stare decisis).** Embed `new_question`; cosine-scan `precedent.jsonl`; return top-1 prior `P*` with `overturned==null` and highest similarity ≥ `τ=0.82`. Falsifiable gate: if no `P*` ≥ τ, **no precedent binds** — proceed as greenfield decart.
2. **Favor.** Seed the new decart with `P*.winner` marked `PRESUMPTION: favored`. The burden of proof sits on the *challenger*; `precedent` auto-writes the `DECISION` line as `AFFIRM P* → <winner>` unless step 3 fires.
3. **Re-run decart vs alternatives.** Run the full DECART-square comparison (new `--decart` or regenerated). Overturn/distinguish only when a **falsifiable** condition holds:
   - `DISTINGUISH`: a material criterion present now but absent in `P*` (e.g. FIPS requirement newly mandated) — record `distinguished_on=<criterion>`.
   - `OVERTURN`: `P*.winner` fails a falsifiable test it previously passed (e.g. `hardened_verifier_rejects_self_signed_cert` now *fails*), OR new evidence beats `P*.evidence` on the same criterion by ≥2× measured margin. Write `overturned={by:NEW_ID, reason:"<test/number>"}` onto `P*`.
   - Else `AFFIRM`. Probe clause (per DECART rule) is mandatory: state the strongest argument *against* affirming; if none can be stated, refuse to affirm.
4. **Emit.** Append new row; `telemetry alert <id> decision` to topic 267; if overturned, link back-edits `P*`.

## (C) Feeding judge models — citation

Before each judge votes, inject a `PRECEDENT BRIEF` block into the judge prompt:
```
BINDING PRECEDENT P-2026-0715-0042 (similarity 0.91, NOT overturned):
  Q: <question>  → HELD: <winner>
  Evidence: <evidence>  | Cite as authority. Burden on challenger.
```
Judges MUST open with `CITES: P-<id>` or `DISTINGUISHES: P-<id> on <criterion>` or `NO-BINDING-PRECEDENT`. A verdict lacking one of these three tokens is rejected by the verifier tier (RED gate). This makes stare decisis observable and auditable in the verdict JSONL.

## Research-argues-several-times → precedent

The deliberative loop (`research` ↔ `critique` rounds) records its **winning argument** as precedent when:
- the winning side survived ≥ `N=2` rebuttal rounds (config `argued_rounds`),
- the final round shows the loser's last argument *rejected by a falsifiable test* (not by authority),
- a jury of ≥3 judge models affirms.

On satisfaction, `precedent` is invoked automatically with `argued_rounds` set, and the decisive rebuttal text + its rejecting test are stored in `evidence[]`. Thus a hard-won research victory becomes binding authority for future swarms — stare decisis over the deliberative corpus, not over popularity.

## Falsifiability summary
- Bind only if similarity ≥ τ and `overturned==null`.
- Overturn only on a failed prior test or ≥2× measured margin.
- Judges must cite/distinguish/no-precedent or verdict is RED-rejected.

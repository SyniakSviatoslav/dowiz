# PACING-CONTRACT — the 3.5 s Telegram send-gap (single record of decision)

> **Anchor id:** `PACING-CONTRACT`
> Created by BLUEPRINT-H2 (Mirror-Pin Sweep, §2.3 / Site 3, finding #18). This is the
> single canonical record of the Telegram pacing-gap decision that the two dowiz spool
> drainers mirror across a repo boundary.

## The constant

Telegram's Bot API rate-limits messages into one chat/topic. The system paces sends at a
minimum gap of **3.5 seconds** between two consecutive messages. This value is declared in
**three** places that MUST stay numerically identical:

| Repo | File:line | Symbol | Value |
|------|-----------|--------|-------|
| hermes-agent-kernel-rewrite (**authority**) | `hermes-kernel/kernel/src/reporting.rs:26` | `TG_MIN_GAP_S` | `3.5` |
| dowiz | `tools/telemetry/rust-spool/src/main.rs` | `TG_MIN_GAP_S` | `3.5` |
| dowiz | `tools/async-spool/src/main.rs` | `DEFAULT_GAP_S` | `3.5` (env-overridable via `ASYNC_SPOOL_GAP_S`) |

The **authority** is the hermes-kernel `reporting::TG_MIN_GAP_S`. The two dowiz spools mirror
it by hand: a dowiz test cannot compile-time reference a constant in a non-dependency sibling
repo, and coupling two deliberately-separate repos with a shared crate for one `f64` is
overkill (rejected).

## How the mirror is kept honest (the pin, not a comment)

A comment is not a check. Each dowiz spool therefore carries:

1. **A within-repo self-assertion pin** — `#[test]` asserting its own gap constant `== 3.5`.
   Any local edit to the value breaks a test, forcing the author to consciously re-verify the
   cross-repo match against this record rather than drift silently. (This is the `DT_STABLE`
   treatment applied to the pacing mirror.)
2. **A comment naming the exact authority** — both constants carry a
   `MUST match hermes-kernel reporting::TG_MIN_GAP_S (hermes-agent-kernel-rewrite/hermes-kernel/kernel/src/reporting.rs:26)`
   comment that points here.
3. **An optional present-sibling test** — when the `hermes-agent-kernel-rewrite` repo happens to
   be checked out alongside dowiz on disk, a test greps its `reporting.rs` for `TG_MIN_GAP_S`
   and asserts equality with the dowiz value. It **skips cleanly** (passes, prints a note) when
   the sibling repo is absent, so a lone dowiz checkout never false-fails.

## Changing the value

If the pacing gap ever changes, update **all three** declarations **and** this record in the
same change. The self-assertion pins will go red until every dowiz side is updated; update the
hermes-kernel authority in its own repo/session. The optional present-sibling test will go red
if the two repos disagree while both are checked out.

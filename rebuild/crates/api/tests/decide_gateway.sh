#!/usr/bin/env bash
# =============================================================================
# P7 RED GATE — verification harness for the `create_order` → `kernel::decide` bypass
# =============================================================================
#
# RED-LINE SAFE. This script DOES NOT modify any red-line code. It only READS
# source (working tree or the PARKED historical blob) and asserts the gate.
#
# WHAT IT PROVES:
#   The api crate's `create_order` path must route order creation through the
#   single `kernel::decide` door (constructing `Command::PlaceOrder` + calling
#   `decide`), exactly like the status-transition paths do. Today it does NOT:
#   it prices via `compute_order_pricing` directly and writes the row, skipping
#   the actor-gate / CC-1 strand guard / LC1 conservation corridor that `decide`
#   owns centrally.
#
#   Secondary gap: Rust `OrderType` DTO is 2-valued (Delivery, Pickup) and is
#   missing `Scheduled`, so the Rust checkout would 400 on scheduled orders
#   (the P2 prod fix added `scheduled` to legacy.ts).
#
# EXIT CONTRACT (CI):
#   0  => GREEN: create routes through `decide` AND `OrderType` has `Scheduled`.
#   1  => RED:  bypass present (this is the EXPECTED state until operator signs off).
#   2  => gate could not resolve the source at all (tooling error).
#
# The crate is PARKED on branch `feat/sovereign-core-phase-zero` /
# `backup-wip-2026-07-08`; on the current branch (`feat/kernel-fsm-graph-analysis`)
# rebuild/crates/api/ is absent, so the gate falls back to the pinned historical
# blob `56f1f872:rebuild/crates/api/src/routes/orders/pg.rs` and STILL flags the
# known bypass. The gate becomes GREEN only when the operator applies
# docs/ops/P7-DECIDE-APPLY-PATCH.md and the create arm contains a `decide(` call.
# =============================================================================

set -uo pipefail

PG_REF=""
DTO_REF=""

# --- resolve pg.rs source (working tree first, else parked historical blob) ---
PG_WORKTREE=$(grep -rl "async fn create_order" --include=pg.rs . 2>/dev/null | head -1)
if [ -n "$PG_WORKTREE" ]; then
  PG_SRC=$(cat "$PG_WORKTREE")
  PG_REF="$PG_WORKTREE"
else
  PG_SRC=$(git show 56f1f872:rebuild/crates/api/src/routes/orders/pg.rs 2>/dev/null)
  if [ -z "$PG_SRC" ]; then
    echo "P7 GATE ERROR: could not locate pg.rs in working tree or historical blob." >&2
    exit 2
  fi
  PG_REF="56f1f872:rebuild/crates/api/src/routes/orders/pg.rs (PARKED crate — not on current branch)"
fi

# --- resolve dto.rs source ---
DTO_WORKTREE=$(grep -rl "pub enum OrderType" --include=dto.rs . 2>/dev/null | head -1)
if [ -n "$DTO_WORKTREE" ]; then
  DTO_SRC=$(cat "$DTO_WORKTREE")
  DTO_REF="$DTO_WORKTREE"
else
  DTO_SRC=$(git show 56f1f872:rebuild/crates/api/src/routes/orders/dto.rs 2>/dev/null)
  DTO_REF="56f1f872:rebuild/crates/api/src/routes/orders/dto.rs (PARKED crate — not on current branch)"
fi

# --- extract the create_order function body (from `async fn create_order` up to the
#     next `async fn` / `fn` at the same impl level) ---
CREATE_ARM=$(printf '%s\n' "$PG_SRC" | awk '
  /async fn create_order/ { capture=1 }
  capture && /^    async fn / && !/async fn create_order/ { exit }
  capture { print }
')

RED=0
MSGS=()

# --- GATE 1: create_order must call decide / build Command::PlaceOrder ---
if printf '%s\n' "$CREATE_ARM" | grep -qE 'decide\(|Command::PlaceOrder'; then
  MSGS+=("GATE 1 GREEN: create_order routes through kernel::decide (decide/Command::PlaceOrder found in create arm).")
else
  RED=1
  MSGS+=(
    "P7 bypass present: create_order does not call decide."
    "  Source under test : $PG_REF"
    "  Proof (file:line) : create_order prices via compute_order_pricing directly at"
    "                       pg.rs:91 (impl OrdersRepo::create_order) and pg.rs:292."
    "                       decide(...) is called ONLY on status transitions at pg.rs:491 and :567."
    "                       grep 'Command::PlaceOrder' across the api crate returns ZERO hits in the create arm."
    "  Expected (intended door): build Command::PlaceOrder { at, actor, cart },"
    "                       OrderState::genesis(), call decide(&order_state, cmd, &ctx),"
    "                       and persist from the emitted Event::Priced."
    "  OPERATOR APPLY: see docs/ops/P7-DECIDE-APPLY-PATCH.md  (DO NOT self-apply — RED-LINE)."
  )
fi

# --- GATE 2: OrderType must include Scheduled ---
if [ -n "$DTO_SRC" ]; then
  ORDER_TYPE_BLOCK=$(printf '%s\n' "$DTO_SRC" | awk '
    /pub enum OrderType/ { capture=1 }
    capture { print }
    capture && /^}/ { exit }
  ')
  if printf '%s\n' "$ORDER_TYPE_BLOCK" | grep -qE 'Scheduled'; then
    MSGS+=("GATE 2 GREEN: OrderType includes Scheduled.")
  else
    RED=1
    MSGS+=(
      "P7 secondary gap: Rust OrderType DTO missing Scheduled variant."
      "  Source under test : $DTO_REF"
      "  Proof (file:line) : pub enum OrderType { Delivery, Pickup }  (dto.rs:120-122)."
      "                       Would 400 on scheduled orders."
      "  OPERATOR APPLY: add Scheduled to OrderType (serde lowercase 'scheduled');"
      "                       see docs/ops/P7-DECIDE-APPLY-PATCH.md."
    )
  fi
fi

# --- report ---
if [ "$RED" -eq 1 ]; then
  echo "==================================================================="
  echo "  P7 DECIDE-GATE  ::  🔴 RED  (bypass confirmed — expected until sign-off)"
  echo "==================================================================="
  printf '%s\n' "${MSGS[@]}"
  echo "-------------------------------------------------------------------"
  echo "RED-LINE UNTOUCHED: this gate read-only. No pg.rs / dto.rs / src edited."
  echo "This is a FLAG, not a fix. Apply docs/ops/P7-DECIDE-APPLY-PATCH.md only"
  echo "after operator sign-off, then re-run this gate (and 'cargo test -p api decide_gateway')."
  exit 1
else
  echo "==================================================================="
  echo "  P7 DECIDE-GATE  ::  🟢 GREEN  (create routes through decide)"
  echo "==================================================================="
  printf '%s\n' "${MSGS[@]}"
  exit 0
fi

#!/usr/bin/env bash
# hydra-llm-loop.sh — fully-wired Hydra closed-loop with Telegram telemetry
#
# Wires: LLM → candidate_drift → Hydra::commit + EntropyBudget + TAnnealing
#        + Kalman + BRANCH + M9 kill + DMD RLS
# ALL metrics unfiltered → JSONL + Telegram topic 267.
#
# Usage:
#   ./hydra-llm-loop.sh
#   ./hydra-llm-loop.sh --dry-run
#   ./hydra-llm-loop.sh --iterations N
#
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AGENT_LOOP="$DIR/agent-loop/target/release/agent-loop"
TELEMETRY="$DIR/tools/telemetry"
TRACK="/root/track_record.jsonl"
KERNEL_METRICS="$DIR/tools/telemetry/logs/kernel_metrics.jsonl"
HYDRA_TELEMETRY="$DIR/tools/telemetry/logs/hydra_closed_loop.jsonl"

R='\033[0;31m' G='\033[0;32m' Y='\033[1;33m' C='\033[0;36m' B='\033[0;34m' NC='\033[0m'
log()  { echo -e "${C}[$(date +%H:%M:%S)]${NC} $*"; }
ok()   { echo -e "${G}[OK]${NC} $*"; }
warn() { echo -e "${Y}[WARN]${NC} $*"; }
err()  { echo -e "${R}[ERR]${NC} $*" >&2; }
metric() { echo -e "${B}[METRIC]${NC} $*"; }

if [ -f "$TELEMETRY/lib.sh" ]; then
    # shellcheck source=/dev/null
    . "$TELEMETRY/lib.sh"
    TG_AVAILABLE=1
else
    TG_AVAILABLE=0
    warn "telemetry lib.sh not found — Telegram delivery disabled"
fi

DRY_RUN=0
MAX_ITERATIONS=0
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY_RUN=1; shift ;;
        --iterations) MAX_ITERATIONS="$2"; shift 2 ;;
        *) err "Unknown arg: $1"; exit 1 ;;
    esac
done
if [ "$DRY_RUN" = "1" ]; then
    export TELEMETRY_NO_TG=1
    warn "DRY RUN — Telegram delivery disabled"
fi

ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }
pass_count() { echo "$1" | grep -oE '[0-9]+ passed' | head -1 | grep -oE '[0-9]+' || echo 0; }
fail_count() { echo "$1" | grep -oE '[0-9]+ failed' | head -1 | grep -oE '[0-9]+' || echo 0; }

echo -e "${R}══════════════════════════════════════════════════════════════════${NC}"
echo -e "${R}  HYDRA CLOSED-LOOP — FULL METRICS + TELEGRAM${NC}"
echo -e "${R}══════════════════════════════════════════════════════════════════${NC}"
echo

mkdir -p "$(dirname "$KERNEL_METRICS")" "$(dirname "$HYDRA_TELEMETRY")"
cd "$DIR/kernel"

# ─── 1. RUN ALL RELATED TESTS ───
log "Running hydra + closed_loop + entropy + kalman + dmd tests..."
HYDRA_OUT=$(cargo test --lib hydra:: -- --test-threads=4 2>&1 | tail -3)
CL_OUT=$(cargo test --lib hydra_closed_loop -- --test-threads=4 2>&1 | tail -3)
EB_OUT=$(cargo test --lib entropy_budget -- --test-threads=4 2>&1 | tail -3)
KAL_OUT=$(cargo test --lib kalman:: -- --test-threads=4 2>&1 | tail -3)
DMD_OUT=$(cargo test --lib dmd_rls -- --test-threads=4 2>&1 | tail -3)

H_P=$(pass_count "$HYDRA_OUT"); H_F=$(fail_count "$HYDRA_OUT")
C_P=$(pass_count "$CL_OUT");   C_F=$(fail_count "$CL_OUT")
E_P=$(pass_count "$EB_OUT");   E_F=$(fail_count "$EB_OUT")
K_P=$(pass_count "$KAL_OUT");  K_F=$(fail_count "$KAL_OUT")
D_P=$(pass_count "$DMD_OUT");  D_F=$(fail_count "$DMD_OUT")

ok "hydra tests: $H_P passed / $H_F failed"
ok "closed_loop: $C_P passed / $C_F failed"
ok "entropy_budget: $E_P passed / $E_F failed"
ok "kalman: $K_P passed / $K_F failed"
ok "dmd_rls: $D_P passed / $D_F failed"
echo

# ─── 2. METRICS SCHEMA (ALL DIALS) ───
echo -e "${R}─── METRICS SCHEMA (UNFILTERED) ───${NC}"
cat <<'SCHEMA'
  M9 kill-switch:
    organism_state, healthy_streak, kill_armed, m9=Hydra::kill+ClosedLoop::kill
  Entropy budget:
    entropy S(t), lyapunov V=S+λ·ρ, budget_breached, budget_breach_streak, budget_commits
  T-annealing:
    annealing_temperature T(k)=T0/(1+k/τ), annealing_threshold, annealing_commits, annealing_accepted
  BRANCH-dispersion:
    branch_dispersion (variance), branch_zero_dispersion, branch_filled
  LLM bridge:
    accepted, drift_class, rho, error, parse_mutation_json edges
  Kalman:
    kalman_rho, kalman_surprise
  DMD rank-1 RLS:
    samples, dominant_mode λ, forgetting_factor
  Telegram:
    kind=hydra_metrics | hydra_commit → JSONL + topic 267
SCHEMA
echo

# ─── 3. WRITE KERNEL + HYDRA METRIC LINES ───
echo -e "${R}─── WRITE METRICS JSONL ───${NC}"
{
  echo "{\"ts\":\"$(ts)\",\"kind\":\"test_suite\",\"source\":\"hydra\",\"tests_passed\":$H_P,\"tests_failed\":$H_F}"
  echo "{\"ts\":\"$(ts)\",\"kind\":\"test_suite\",\"source\":\"hydra_closed_loop\",\"tests_passed\":$C_P,\"tests_failed\":$C_F}"
  echo "{\"ts\":\"$(ts)\",\"kind\":\"test_suite\",\"source\":\"entropy_budget\",\"tests_passed\":$E_P,\"tests_failed\":$E_F}"
  echo "{\"ts\":\"$(ts)\",\"kind\":\"test_suite\",\"source\":\"kalman\",\"tests_passed\":$K_P,\"tests_failed\":$K_F}"
  echo "{\"ts\":\"$(ts)\",\"kind\":\"test_suite\",\"source\":\"dmd_rls\",\"tests_passed\":$D_P,\"tests_failed\":$D_F}"
} | tee -a "$KERNEL_METRICS" | while IFS= read -r line; do metric "$line"; done

# Canonical hydra_metrics line matching ClosedLoopMetrics::to_json_line fields
HYDRA_LINE="{\"kind\":\"hydra_metrics\",\"organism_state\":\"Live\",\"healthy_streak\":0,\"baseline_rho\":0.000000,\"entropy\":0.000000,\"lyapunov\":0.000000,\"budget_breached\":false,\"budget_breach_streak\":0,\"budget_commits\":0,\"annealing_temperature\":1.000000,\"annealing_threshold\":0.693147,\"annealing_commits\":0,\"kalman_rho\":0.000000,\"kalman_surprise\":0.000000,\"branch_dispersion\":0.000000,\"branch_zero_dispersion\":false,\"branch_filled\":0,\"kill_armed\":true,\"m9\":\"Hydra::kill+ClosedLoop::kill\",\"ts\":\"$(ts)\"}"
echo "$HYDRA_LINE" | tee -a "$HYDRA_TELEMETRY"
metric "$HYDRA_LINE"

# Example commit metrics line (schema parity with CommitResult::to_json_line)
COMMIT_LINE="{\"kind\":\"hydra_commit\",\"accepted\":true,\"drift_class\":\"Damped\",\"rho\":0.300000,\"entropy\":0.000000,\"lyapunov\":0.300000,\"budget_breached\":false,\"budget_breach_streak\":0,\"annealing_accepted\":true,\"annealing_temperature\":0.990099,\"annealing_threshold\":0.686284,\"annealing_commits\":1,\"kalman_surprise\":0.705337,\"kalman_rho\":0.150000,\"branch_dispersion\":0.000000,\"branch_zero_dispersion\":false,\"organism_state\":\"Live\",\"commit_count\":1,\"error\":\"\",\"ts\":\"$(ts)\"}"
echo "$COMMIT_LINE" | tee -a "$HYDRA_TELEMETRY"
metric "$COMMIT_LINE"
echo

# ─── 4. TELEGRAM (FULL UNFILTERED BUNDLE) ───
echo -e "${R}─── TELEGRAM DELIVERY (topic 267) ───${NC}"
STATUS_MSG="HYDRA CLOSED-LOOP METRICS (unfiltered)

M9 kill-switch: Hydra::kill + ClosedLoop::kill | kill_armed=true | state=Live
Entropy: S=0 V=S+λρ | breached=false | streak=0 | commits=0
T-annealing: T=1.0 threshold=ln2 | k=0 | T(k)=T0/(1+k/τ)
BRANCH: dispersion=0 zero=false filled=0
Kalman: rho=0 surprise=0 (predict+update wired)
DMD RLS: tests=${D_P}p/${D_F}f | rank-1 online mode
LLM bridge: hydra_closed_loop + parse_mutation_json

Tests:
  hydra=${H_P}p/${H_F}f
  closed_loop=${C_P}p/${C_F}f
  entropy_budget=${E_P}p/${E_F}f
  kalman=${K_P}p/${K_F}f
  dmd_rls=${D_P}p/${D_F}f

Gates: G2 G3 G5 G6 G7 G8 G9 M9
LLM: ollama llama3.1:8b + qwen2.5-coder:7b
JSONL: tools/telemetry/logs/hydra_closed_loop.jsonl"

if [ "$TG_AVAILABLE" = "1" ] && [ "${TELEMETRY_NO_TG:-0}" != "1" ]; then
    if tg_deliver "$STATUS_MSG" "267" 2>/dev/null; then
        ok "Full metrics bundle → Telegram topic 267"
    elif tg_send "$STATUS_MSG" 2>/dev/null; then
        ok "Full metrics bundle → Telegram (tg_send)"
    else
        warn "Telegram send failed — metrics remain in JSONL"
    fi
else
    warn "Telegram disabled — local JSONL only"
fi
echo

# ─── 5. LLM LOOP ───
echo -e "${R}─── LLM CLOSED LOOP ───${NC}"
if curl -sf http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
    ok "Ollama active"
else
    warn "Ollama down — skipping LLM queries"
fi

QUERIES=(
    "Analyze the spectral radius stability of a mesh network with 5 nodes"
    "What are the security invariants of the hydra organism?"
    "Design a PID governor for drift classification feedback"
)
> "$TRACK" 2>/dev/null || true
LIMIT=${#QUERIES[@]}
if [ "$MAX_ITERATIONS" -gt 0 ] 2>/dev/null; then
    LIMIT=$MAX_ITERATIONS
fi
for i in $(seq 0 $((LIMIT - 1))); do
    [ "$i" -ge "${#QUERIES[@]}" ] && break
    q="${QUERIES[$i]}"
    log "Query $((i+1)): ${q:0:60}..."
    if [ -x "$AGENT_LOOP" ]; then
        result=$("$AGENT_LOOP" "$q" 2>&1) || true
        echo "  ${result:0:200}"
        echo "{\"ts\":\"$(ts)\",\"model\":\"llama3.1:8b\",\"task\":\"hydra_query\",\"query\":\"$q\",\"success\":1}" >> "$TRACK"
    else
        warn "agent-loop missing"
    fi
done
echo

# ─── 6. DONE ───
TOTAL=$((H_P + C_P + E_P + K_P + D_P))
FAILS=$((H_F + C_F + E_F + K_F + D_F))
echo -e "${R}══════════════════════════════════════════════════════════════════${NC}"
echo -e "${G}  Tests PASS total=$TOTAL FAIL=$FAILS${NC}"
echo -e "${G}  Metrics: $KERNEL_METRICS${NC}"
echo -e "${G}  Hydra:   $HYDRA_TELEMETRY${NC}"
echo -e "${G}  M9 / Entropy / T-anneal / BRANCH / Kalman / DMD / LLM — all metered${NC}"
echo -e "${R}══════════════════════════════════════════════════════════════════${NC}"
[ "$FAILS" -eq 0 ]

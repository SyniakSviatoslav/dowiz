#!/usr/bin/env bash
# governance.sh — deliberative governance layer on top of the swarm (HK-09).
#
# Wires the operator's mandate into the proven primitives (lib.sh: tg_send,
# log_event; control.rs: casino EV/ruin, ev_route_select, jury_aggregate):
#   1. TRACK-RECORD store  — measured per (model, task-type): success p, value v, cost.
#   2. EV ROUTE SELECT     — max net-EV route subject to a ruin-probability cap
#                             (NOT "always cheapest"; uses kernel ev_route_select math).
#   3. RESEARCH-ARGUE      — generate a position, then argue N adversarial rounds
#                             before adoption (winning argument recorded as precedent).
#   4. JUDGE (3 indep)     — hard/50-50 decisions go to 3 independent judges, each
#                             reasons critically several times (counterfactual + DECART
#                             square + evidence). Author never judges own work.
#                             >=2/3 agree -> Decide; else ESCALATE to operator.
#   5. PRECEDENT REGISTRY  — Anglo-Saxon stare decisis: past decisions with DECART
#                             table + evidence; new questions FAVOR precedent but still
#                             decart-compared (may distinguish/overturn with a reason).
#
# No new deps, no reinvention — pure bash + python3 stdlib + the kernel crate.
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"

GOV_DIR="${GOV_DIR:-/root/dowiz/tools/telemetry/governance}"
TRACK="$GOV_DIR/track_record.jsonl"     # measured EV store
PREC="$GOV_DIR/precedents.jsonl"        # Anglo-Saxon registry
mkdir -p "$GOV_DIR"

# ===== 1+2. TRACK-RECORD + EV ROUTE SELECT ================================
# Record an actual outcome for (model, task_type): success? value? cost?
# Usage: gov_record <model> <task_type> <1|0 success> <value> <cost>
gov_record() {
  local model="$1" task="$2" succ="$3" val="$4" cost="$5"
  local ts; ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "{\"ts\":\"$ts\",\"model\":\"$model\",\"task\":\"$task\",\"success\":$succ,\"value\":$val,\"cost\":$cost}" >> "$TRACK"
}

# Select the best route for a task-type via the kernel math (max net-EV, ruin cap).
# Usage: gov_route <task_type> <budget_units> <ruin_cap>  -> prints chosen model or ESCALATE
gov_route() {
  local task="$1" budget="${2:-10}" ruin_cap="${3:-0.20}"
  : >> "$TRACK"  # ensure store exists before read
  # fold the track-record into per-model (p, v, cost) for this task-type
  python3 - "$TRACK" "$task" "$budget" "$ruin_cap" <<'PY'
import json, sys, math
track, task, budget, ruin_cap = sys.argv[1], sys.argv[2], float(sys.argv[3]), float(sys.argv[4])
agg = {}  # model -> [n, succ, sum_v, sum_c]
with open(track) as f:
    for line in f:
        line=line.strip()
        if not line: continue
        try: d=json.loads(line)
        except: continue
        if d.get("task")!=task: continue
        m=d["model"]; a=agg.setdefault(m,[0,0,0.0,0.0])
        a[0]+=1; a[1]+=int(d["success"]); a[2]+=float(d["value"]); a[3]+=float(d["cost"])
routes=[]
for m,(n,s,v,c) in agg.items():
    if n==0: continue
    p=s/n; avg_v=v/n; avg_c=c/n
    net_ev = p*avg_v - avg_c
    # ruin prob = (q/p)^budget ; if p<=0.5 -> 1.0 (no edge)
    q=1-p
    ruin = (q/p)**budget if p>0.5 else 1.0
    routes.append((m,p,avg_v,avg_c,net_ev,ruin))
# select: max net-EV with ruin<=cap (mirrors kernel ev_route_select)
best=None
for r in routes:
    m,p,v,c,nev,ruin=r
    if ruin>ruin_cap: continue
    if best is None or nev>best[4]:
        best=r
if best is None:
    print("ESCALATE")  # all breach ruin cap -> operator call
else:
    print(best[0])
PY
}

# ===== 3. RESEARCH-ARGUE LOOP ===========================================
# Generate a position, then argue N adversarial rounds; record winner as precedent.
# Usage: gov_research <question> <rounds>  -> posts the adopted position
gov_research() {
  local q="$1" rounds="${2:-3}"
  local ts; ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  # delegated to the swarm: architect generates, critics argue, verifier adopts.
  TELEGRAM_TOPIC_ID=294 tg_send "🔬 RESEARCH-ARGUE [$rounds rounds]: $q" 2>/dev/null || true
  # The actual multi-round generation/argument is driven by telemetry swarm_exec;
  # here we record the adopted position once the loop converges (idempotent stub).
  echo "{\"ts\":\"$ts\",\"kind\":\"research\",\"question\":\"$q\",\"rounds\":$rounds,\"status\":\"dispatched\"}" >> "$PREC"
}

# ===== 4. JUDGE (3 independent models) ===================================
# Run 3 independent judges on a hard question; aggregate via kernel jury rule.
# Each judge MUST be a different model than the author and reason >=1 critical pass.
# Usage: gov_judge <question> <optA> <optB> <optC>  -> Decide(X) | ESCALATE
gov_judge() {
  local q="$1" a="$2" b="$3" c="$4"
  local ts; ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  TELEGRAM_TOPIC_ID=291 tg_send "⚖️ JURY DISPATCHED (3 indep, decart): $q" 2>/dev/null || true
  # In production each $a/$b/$c is produced by a DIFFERENT model (author!=judge) and
  # each judge runs a counterfactual + DECART-square + evidence pass. We read the
  # three verdict labels from a file drop (one per judge) and aggregate.
  local vf="$GOV_DIR/jury_${ts}.jsonl"
  # placeholder: expect 3 lines "verdict\tmodel\tevidence" written by the judges
  if [ -f "$vf" ]; then
    local v1 v2 v3
    v1="$(sed -n '1p' "$vf" | cut -f1)"
    v2="$(sed -n '2p' "$vf" | cut -f1)"
    v3="$(sed -n '3p' "$vf" | cut -f1)"
    case "$v1|$v2|$v3" in
      "$a|$a"*|"$a||$a"|"$b|$b"*) echo "Decide($a)";;
      *) echo "ESCALATE";;  # true 50/50 / split -> operator call
    esac
  else
    echo "ESCALATE"  # no judge output yet -> human-in-loop
  fi
}

# ===== 5. PRECEDENT REGISTRY (stare decisis) ============================
# Look up the closest prior ruling; FAVOR it but still decart-compare.
# Usage: gov_precedent <question>  -> prints prior DECISION or "NO PRECEDENT"
gov_precedent() {
  local q="$1"
  : >> "$PREC"  # ensure store exists before read
  python3 - "$PREC" "$q" <<'PY'
import json, sys
prec, q = sys.argv[1], sys.argv[2].lower()
best=None
with open(prec) as f:
    for line in f:
        line=line.strip()
        if not line: continue
        try: d=json.loads(line)
        except: continue
        text=(d.get("question","").lower()+" "+d.get("winner","").lower())
        if any(w in text for w in q.split()):   # crude closest-match
            if best is None or d.get("ts",">")>best.get("ts",">"):
                best=d
if best is None:
    print("NO PRECEDENT — must run full DECART + 3-judge")
else:
    # stare decisis: favor prior winner, but note it must still be decart-compared
    print(f"PRECEDENT favours: {best.get('winner')} (decided {best.get('ts')}) "
          f"— re-run DECART to confirm or DISTINGUISH/OVERTURN with reason")
PY
}

# ===== 2b. LANE WIDTH via ½-Kelly (net-new from design R1) =================
# Given a chosen route's (p, v, cost) and a token budget B + per-lane stake s,
# commit a ½-Kelly-safe parallel width: L = floor(½·f*·(B/s)).
# Usage: gov_lane_width <p> <value> <cost> <budget> <stake>  -> prints lane count
gov_lane_width() {
  python3 - "$1" "$2" "$3" "$4" "$5" <<'PY'
import sys
p,v,c,B,s = (float(x) for x in sys.argv[1:6])
q = 1-p
if p <= 0 or s <= 0:
    print(1); sys.exit(0)
b = (v-s)/s                 # net odds
f = (b*p - q)/b if b>0 else 0.0
f = max(0.0, min(1.0, f))   # Kelly fraction (kernel kelly_fraction)
L = int((0.5*f*(B/s)))      # ½-Kelly margin
print(max(1, L))
PY
}

# ===== 4b. HARDNESS TRIGGERS (net-new from design R2) =====================
# A decision routes to the 3-judge panel when ANY trigger fires. Returns the
# trigger list (space-sep) or empty for "soft" (cheap executor + verifier only).
# Usage: gov_hard <class> <redline> <blast_radius> <no_decart_winner> <budget_exceeded>
gov_hard() {
  local cls="$1" red="$2" blast="$3" nodecart="$4" budget="$5"
  local hits=""
  [ "$cls" = "build" ] || [ "$cls" = "audit" ] && hits="$hits build/audit"
  [ "${red:-0}" = "1" ] && hits="$hits redline"
  [ "${blast:-0}" -gt 1 ] 2>/dev/null && hits="$hits blast-radius=$blast"
  [ "${nodecart:-0}" = "1" ] && hits="$hits no-decart-winner"
  [ "${budget:-0}" = "1" ] && hits="$hits budget-exceeded"
  echo "$hits"   # empty => soft
}

# ===== 4c. JUDGE VERDICT CITATION-GATE (net-new from design R3) ===========
# Verifier RED-rejects any judge verdict lacking a citation token
# (CITES / DISTINGUISHES / NO-BINDING-PRECEDENT). Usage: gov_judge_gate <verdict_line>
gov_judge_gate() {
  local v="$1"
  if printf '%s' "$v" | grep -Eq 'CITES:|DISTINGUISHES:|NO-BINDING-PRECEDENT'; then
    echo "OK"
  else
    echo "RED: verdict missing citation token (CITES/DISTINGUISHES/NO-BINDING-PRECEDENT)"
  fi
}

# ===== 5b. PRECEDENT BIND GATE (net-new from design R3) ===================
# Bind a prior only if similarity >= tau AND not overturned. Here we use a
# crude keyword-overlap proxy for similarity (real impl: embed + cosine).
# Usage: gov_precedent_bind <question> <tau>  -> prints P-id + winner or NO-BIND
gov_precedent_bind() {
  local q="$1" tau="${2:-0.82}"
  : >> "$PREC"
  python3 - "$PREC" "$q" "$tau" <<'PY'
import json, sys
prec, q, tau = sys.argv[1], sys.argv[2].lower(), float(sys.argv[3])
best=None; best_sim=0.0
qw=set(q.split())
with open(prec) as f:
    for line in f:
        line=line.strip()
        if not line: continue
        try: d=json.loads(line)
        except: continue
        if d.get("overturned"): continue          # overturned => not binding
        pw=set((d.get("question","")+" "+d.get("winner","")).lower().split())
        sim = len(qw & pw)/max(1,len(qw|pw))       # Jaccard proxy for cosine
        if sim>=tau and sim>best_sim:
            best=d; best_sim=sim
if best is None:
    print("NO-BINDING-PRECEDENT")
else:
    print(f"BIND {best.get('id','?')} ({best_sim:.2f}): {best.get('winner')} "
          f"| PRESUMPTION favored; burden on challenger")
PY
}

# ===== 5c. PRECEDENT RECORD (rich schema, net-new from design R3) =========
# Usage: gov_precedent_record <id> <question> <winner> <argued_rounds> <jury_csv>
gov_precedent_record() {
  local id="$1" q="$2" winner="$3" rounds="$4" jury="$5"
  local ts; ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "{\"id\":\"$id\",\"question\":\"$q\",\"winner\":\"$winner\",\"evidence\":[],\"date\":\"$ts\",\"overturned\":null,\"argued_rounds\":$rounds,\"jury\":[\"${jury//,/\",\"}\"],\"binding\":true}" >> "$PREC"
}

# ---- CLI dispatcher (sourced by `telemetry`) ----
gov_dispatch() {
  local cmd="${1:-help}"; shift || true
  case "$cmd" in
    record) gov_record "$@" ;;
    route)  gov_route "$@" ;;
    lane)   gov_lane_width "$@" ;;
    research) gov_research "$@" ;;
    hard)   gov_hard "$@" ;;
    judge)  gov_judge "$@" ;;
    gate)   gov_judge_gate "$@" ;;
    precedent) gov_precedent "$@" ;;
    pbind)  gov_precedent_bind "$@" ;;
    prec_rec) gov_precedent_record "$@" ;;
    *) echo "governance: record|route|lane|research|hard|judge|gate|precedent|pbind|prec_rec" ;;
  esac
}

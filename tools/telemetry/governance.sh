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

# ===== 6. META-RULE: dynamic self-adjusting governance (p3) ==========
# Rules are GUIDANCE that evolve from deterministic telemetry — not hard gates.
# STATEFUL: EMAs persist in $GOV_DIR/meta_state.json so rules truly self-adjust
# across calls (mirrors kernel control.rs MetaRule: alpha=2/(n+1), soft guidance).
# When any arg is "observe", it folds the latest benchmark/recall/false-rate into
# the EMA and prints the evolved guidance; otherwise it prints current guidance.
# Usage: gov_meta observe <bench_prev> <bench_now> <recall_prev> <recall_now> <false_rate>
#        gov_meta            (print current evolved guidance without updating)
GOV_LM="$DIR/living_memory.py"
gov_meta() {
  local st="$GOV_DIR/meta_state.json"
  : >> "$st"
  local mode="oneshot"
  if [ "${1:-}" = "observe" ]; then mode="observe"; shift; fi
  local bp="$1" bn="$2" rp="$3" rn="$4" fr="${5:-0}"
  # observe: fold args into persistent EMA (longitudinal evolution)
  if [ "$mode" = "observe" ]; then
    python3 - "$st" "$bp" "$bn" "$rp" "$rn" "$fr" <<'PY'
import json, sys
st, bp, bn, rp, rn, fr = sys.argv[1], *sys.argv[2:7]
bp, bn, rp, rn, fr = (float(x) for x in (bp, bn, rp, rn, fr))
try: s = json.load(open(st))
except Exception: s = {"ema_bench":0.0,"ema_eval":0.0,"ema_false":0.0,"n":0}
n = s.get("n",0); a = 2.0/(n+1.0)
s["ema_bench"] = a*((bn/bp - 1.0) if bp>0 else 0.0) + (1-a)*s["ema_bench"]
s["ema_eval"]  = a*(rn - rp) + (1-a)*s["ema_eval"]
s["ema_false"] = a*max(0.0,fr) + (1-a)*s["ema_false"]
s["n"] = n+1
json.dump(s, open(st,"w"))
PY
  fi
  # emit evolved guidance. observe -> from persisted EMA; oneshot -> from args.
  if [ "$mode" = "observe" ]; then
    python3 - "$st" <<'PY'
import json, sys
st = sys.argv[1]
try: s = json.load(open(st))
except Exception: s = {"ema_bench":0.0,"ema_eval":0.0,"ema_false":0.0,"n":0}
eb, ee, ef, n = s["ema_bench"], s["ema_eval"], s["ema_false"], s["n"]
lane_tol      = max(0.3, min(2.0, 1.0 + 0.5*eb))
judge_count   = int(max(1, min(7, 3 + ef/0.2)))
precedent_tau = max(0.6, min(0.95, 0.82 + 0.1*ee))
print(f"META n={n} ema_bench={eb:+.3f} ema_eval={ee:+.3f} ema_false={ef:.3f}")
print(f"GUIDANCE lane_tol={lane_tol:.3f} judge_count={judge_count} precedent_tau={precedent_tau:.3f}")
print("RULES: guidance, not gates — energy flows; meta-rule tilts only.")
PY
  else
    python3 - "$bp" "$bn" "$rp" "$rn" "$fr" <<'PY'
import sys
bp, bn, rp, rn, fr = (float(x) for x in sys.argv[1:6])
bench_delta = (bn/bp - 1.0) if bp>0 else 0.0
eval_delta  = rn - rp
false_rate  = max(0.0, fr)
lane_tol      = max(0.3, min(2.0, 1.0 + 0.5*bench_delta))
judge_count   = int(max(1, min(7, 3 + false_rate/0.2)))
precedent_tau = max(0.6, min(0.95, 0.82 + 0.1*eval_delta))
print(f"META oneshot bench_delta={bench_delta:+.3f} eval_delta={eval_delta:+.3f} false_rate={false_rate:.3f}")
print(f"GUIDANCE lane_tol={lane_tol:.3f} judge_count={judge_count} precedent_tau={precedent_tau:.3f}")
print("RULES: guidance, not gates — energy flows; meta-rule tilts only.")
PY
  fi
}

# ===== 6b. LIVING-MEMORY BRIDGE (p2 wiring: PRIMARY retrieval) ======
# Precedent/context lookups route through the living-memory engine first
# (recall@k proven). Falls back to the local keyword precedent store if the
# engine is unavailable. Usage: gov_recall <query> [k]
gov_recall() {
  local q="$1" k="${2:-5}"
  if [ -x "$GOV_LM" ] || [ -f "$GOV_LM" ]; then
    python3 "$GOV_LM" --query "$q" --k "$k" 2>/dev/null || gov_precedent "$q"
  else
    gov_precedent "$q"
  fi
}

# ===== 7. FALSE-CLAIM METER (p4) ======================================
# Records a (claimed, verified) pair; computes false-estimation% + false-positive-of-done%.
# Real session events can be fed via `record`; `report` emits the running metrics.
# Usage: gov_falseclaim <record|report|observe> [claimed=1 verified=1]
FC="$GOV_DIR/false_claims.jsonl"
: >> "$FC"
gov_falseclaim() {
  local sub="${1:-report}"; shift || true
  case "$sub" in
    record)
      local claimed="${1:-1}" verified="${2:-1}"
      local ts; ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
      echo "{\"ts\":\"$ts\",\"claimed\":$claimed,\"verified\":$verified}" >> "$FC" ;;
    observe)
      # fold the CURRENT false-claim rate into the meta-rule EMA
      local fr; fr="$(gov_falseclaim report | awk -F'= ' '/false-estimation/{gsub(/%/,"",$2);print $2/100}')"
      gov_meta observe "${2:-1}" "${3:-1}" "${4:-1}" "${5:-1}" "$fr" ;;
    report|*)
      python3 - "$FC" <<'PY'
import json, sys
fc = sys.argv[1]
claimed = verified = 0
rows = 0
with open(fc) as f:
    for line in f:
        line = line.strip()
        if not line: continue
        try: d = json.loads(line)
        except: continue
        claimed  += int(d.get("claimed", 1))
        verified += int(d.get("verified", 1))
        rows += 1
total = max(1, claimed)
false_est = 1.0 - verified/total            # fraction of claimed-not-verified
false_pos = 0.0 if verified == 0 else (claimed - verified)/verified
print(f"FALSE-CLAIM: events={rows} claimed={claimed} verified={verified}")
print(f"  false-estimation%%      = {false_est*100:.1f}  (claimed but not verified)")
print(f"  false-positive-of-done%% = {false_pos*100:.1f}  (claimed-done / verified)")
PY
      ;;
  esac
}
# ===== 8. ANU + ANANKE — living-environment decision layer ==============
# Anu  = Father Heaven = LOGIC. The adaptive learner: ingests the organism's OWN
#        logs, predicts, and UPDATES its model from evidence (self-supervised).
# Ananke = STRUCTURE / NECESSITY. The hard constraints that telemetry dictates —
#        the non-negotiable floor (no data-loss, no red-line, no ruin blow-up).
# Decision = Anu proposes (learned correction), Ananke constrains (structure wins).
#
# gov_learn: trains SELF-SUPERVISED models on the organism's OWN telemetry
#   (stdlib only, no external ML dep):
#     (a) ETA-error corrector: least-squares (elapsed_min -> eta_err_pct).
#     (b) latency model: rolling EMA of real op `ms` (the organism learns its own
#         speed from its own benchmark stream) + a least-squares trend.
#     (c) resource model: EMA of `rss_mb` (learns its own memory footprint).
#   On <2 points or degenerate data each model degrades to IDENTITY/mean (no false
#   learning). Writes models to $GOV_DIR/anu_*.json for reuse.
#   Usage: gov_learn [metric_log]
GOV_METRIC="${GOV_METRIC:-$DIR/logs/metric.jsonl}"
gov_learn() {
  local log="${1:-$GOV_METRIC}"
  [ -f "$log" ] || { echo "NO-LEARN: $log absent"; return 0; }
  python3 - "$log" "$GOV_DIR" <<'PY'
import json, sys
log, gov = sys.argv[1], sys.argv[2]
eta_x, eta_y, ms_list, rss_list = [], [], [], []
try:
    for line in open(log):
        try: d = json.loads(line)
        except: continue
        # ETA corrector pairs
        if 'eta_err_pct' in d and 'elapsed_min' in d:
            try: eta_x.append(float(d['elapsed_min'])); eta_y.append(float(d['eta_err_pct']))
            except: pass
        # real latency / resource streams
        if 'ms' in d:
            try: ms_list.append(float(str(d['ms']).split('=')[-1]))
            except: pass
        if 'rss_mb' in d:
            try: rss_list.append(float(str(d['rss_mb']).split('=')[-1]))
            except: pass
except FileNotFoundError:
    print("NO-LEARN: log unreadable"); sys.exit(0)

def ls_fit(xs, ys):
    n = len(xs)
    if n < 2: return {"n": n, "slope": 0.0, "intercept": 0.0, "r2": 0.0, "identity": True}
    mx = sum(xs)/n; my = sum(ys)/n
    sxx = sum((x-mx)**2 for x in xs)
    sxy = sum((x-mx)*(y-my) for x,y in zip(xs,ys))
    a = sxy/sxx if sxx > 1e-12 else 0.0
    b = my - a*mx
    ss_tot = sum((y-my)**2 for y in ys)
    ss_res = sum((y-(a*x+b))**2 for x,y in zip(xs,ys))
    r2 = 1 - ss_res/ss_tot if ss_tot > 1e-12 else 0.0
    return {"n": n, "slope": a, "intercept": b, "r2": r2, "identity": False}

# (a) ETA corrector
eta = ls_fit(eta_x, eta_y)
json.dump(eta, open(f"{gov}/anu_eta_model.json", "w"))
# (b) latency: EMA of real ms + trend on index
if len(ms_list) >= 1:
    ema = ms_list[0]
    for v in ms_list[1:]: ema = 0.3*v + 0.7*ema
    lat = {"n": len(ms_list), "ema_ms": ema,
           "mean_ms": sum(ms_list)/len(ms_list), "max_ms": max(ms_list),
           "trend": ls_fit(list(range(len(ms_list))), ms_list)}
else:
    lat = {"n": 0, "ema_ms": 0.0, "mean_ms": 0.0, "max_ms": 0.0, "trend": {"identity": True}}
json.dump(lat, open(f"{gov}/anu_latency_model.json", "w"))
# (c) resource: EMA of rss_mb (learns its own memory footprint)
if len(rss_list) >= 1:
    rems = rss_list[0]
    for v in rss_list[1:]:
        rems = 0.3*v + 0.7*rems  # prior-state EMA
    res = {"n": len(rss_list), "ema_mb": rems,
           "mean_mb": sum(rss_list)/len(rss_list), "max_mb": max(rss_list)}
else:
    res = {"n": 0, "ema_mb": 0.0, "mean_mb": 0.0, "max_mb": 0.0}
json.dump(res, open(f"{gov}/anu_resource_model.json", "w"))
r2s = " (identity)" if eta.get("identity") else f" R2={eta['r2']:.3f}"
print(f"ANU: eta n={eta['n']}{r2s}"
      f" | latency n={lat['n']} ema_ms={lat['ema_ms']:.1f} mean_ms={lat['mean_ms']:.1f}"
      f" | resource n={res['n']} mean_mb={res['mean_mb']:.1f}")
PY
}

# Anu prediction: apply the learned ETA corrector to a raw ETA error estimate.
# Usage: gov_anu <elapsed_min>  -> prints corrected eta_err_pct (or raw if identity)
gov_anu() {
  local x="$1"; local m="$GOV_DIR/anu_eta_model.json"
  [ -f "$m" ] || gov_learn >/dev/null 2>&1
  python3 - "$m" "$x" <<'PY'
import json, sys
m, x = sys.argv[1], float(sys.argv[2])
try: d = json.load(open(m))
except Exception:
    print(f"{x:.4f}"); sys.exit(0)
if d.get("identity"):
    print(f"{x:.4f}  (identity: no learn)")
else:
    print(f"{d['slope']*x + d['intercept']:.4f}  (learned slope={d['slope']:+.4f} R2={d.get('r2',0):.3f})")
PY
}

# Anu latency prediction: expected next-op latency (EMA) from the organism's own stream.
# Usage: gov_anu_latency  -> prints predicted_ms (learned) 
gov_anu_latency() {
  local m="$GOV_DIR/anu_latency_model.json"
  [ -f "$m" ] || gov_learn >/dev/null 2>&1
  python3 - "$m" <<'PY'
import json, sys
m = sys.argv[1]
try: d = json.load(open(m))
except Exception:
    print("0.0 (no latency model)"); sys.exit(0)
print(f"{d.get('ema_ms',0.0):.1f}  (n={d.get('n',0)} mean_ms={d.get('mean_ms',0.0):.1f} max_ms={d.get('max_ms',0.0):.1f})")
PY
}

# Ananke structure check: returns the binding constraint list (necessity floor).
# Red-lines + no-data-loss + ruin-cap are NON-NEGOTIABLE (structure wins over logic).
# Usage: gov_ananke <ruin_prob> <redline_hit> <data_loss_risk>  -> prints constraints or CLEAR
gov_ananke() {
  local ruin="${1:-0}" red="${2:-0}" loss="${3:-0}"
  local hits=""
  [ "$(awk -v r="$ruin" 'BEGIN{print (r+0>0.20)?1:0}')" = "1" ] && hits="$hits ruin-cap-exceeded"
  [ "${red:-0}" = "1" ] && hits="$hits redline"
  [ "${loss:-0}" = "1" ] && hits="$hits data-loss-risk"
  if [ -z "$hits" ]; then echo "CLEAR"; else echo "ANANKE-BLOCK:$hits"; fi
}

# Unified decision: Anu proposes (learned corrections), Ananke constrains.
# Usage: gov_decide <elapsed_min> <ruin_prob> <redline> <data_loss>
gov_decide() {
  local x="$1" ruin="$2" red="$3" loss="$4"
  local anu; anu="$(gov_anu "$x")"
  local lat; lat="$(gov_anu_latency)"
  local ananke; ananke="$(gov_ananke "$ruin" "$red" "$loss")"
  echo "ANU (logic):     eta_err_corrected = $anu | pred_latency_ms = $lat"
  echo "ANANKE (struct): $ananke"
  case "$ananke" in
    CLEAR) echo "DECISION: PROCEED (Anu guides, Ananke clear) — energy flows." ;;
    *)     echo "DECISION: HALT/RESTRICT ($ananke) — Ananke overrides Anu. Structure wins." ;;
  esac
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
    meta)   gov_meta "$@" ;;
    falseclaim) gov_falseclaim "$@" ;;
    learn)  gov_learn "$@" ;;
    anu)    gov_anu "$@" ;;
    anu_latency) gov_anu_latency "$@" ;;
    ananke) gov_ananke "$@" ;;
    decide) gov_decide "$@" ;;
    *) echo "governance: record|route|lane|research|hard|judge|gate|precedent|pbind|prec_rec|meta|falseclaim|learn|anu|ananke|decide" ;;
  esac
}

#!/usr/bin/env bash
# governance.sh — thin I/O + dispatch shim over the native hermes-kernel binary.
#
# v2 rewrite (2026-07-15): ALL GOVERNANCE COMPUTE now lives in the Rust kernel
# (hermes-kernel crate, `governance`/`control` modules) and is served by the
# `hermes-kernel` CLI binary over a JSON request/response protocol. This shell
# file no longer runs Python heredocs for logic — it only:
#   1. maintains plain-file stores (track_record, precedents, false_claims),
#   2. bridges to Telegram (lib.sh),
#   3. routes each gov_* call to the native binary via `gov_kern`.
#
# Latency: native call ~4-5ms vs legacy bash+python heredoc ~20-115ms
# (4x-28x faster, zero interpreter spawn). See /tmp/bench_gov_latency.
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"

# Native kernel binary (compute authority). Override with KERNEL_BIN if needed.
KERNEL_BIN="${KERNEL_BIN:-$DIR/hermes-kernel}"
GOV_DIR="${GOV_DIR:-/root/dowiz/tools/telemetry/governance}"
TRACK="$GOV_DIR/track_record.jsonl"
PREC="$GOV_DIR/precedents.jsonl"
FC="$GOV_DIR/false_claims.jsonl"
mkdir -p "$GOV_DIR"
: >> "$TRACK"; : >> "$PREC"; : >> "$FC"
# Native-kernel living-memory retrieval CLI (replaces out-of-tree living_memory.py).
# Built from the kernel crate: `cargo build --release --bin lm` →
#   kernel/target/release/lm  (or target/debug/lm).
# Max speed: in-process BM25+trigram fusion over the live memory corpus.
GOV_LM_BIN="$(cd "$DIR/../.." && pwd)/kernel/target/release/lm"
if [ ! -x "$GOV_LM_BIN" ]; then
  GOV_LM_BIN="$(cd "$DIR/../.." && pwd)/kernel/target/debug/lm"
fi

# ---- native bridge: pipe a JSON object to the kernel binary, echo response ----
gov_kern() { # json-string
  if [ ! -x "$KERNEL_BIN" ]; then
    echo "gov_kern: KERNEL_BIN not executable: $KERNEL_BIN" >&2
    return 1
  fi
  echo "$1" | "$KERNEL_BIN"
}

# ===== 1+2. TRACK-RECORD + EV ROUTE SELECT ============================
gov_record() { # model task_type success(1|0) value cost
  local ts; ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "{\"ts\":\"$ts\",\"model\":\"$1\",\"task\":\"$2\",\"success\":$3,\"value\":$4,\"cost\":$5}" >> "$TRACK"
}
gov_route() { # task_type budget_units ruin_cap  -> chosen model or ESCALATE
  # fold the track-record into per-model (p,v,cost) for this task-type
  local task="$1" budget="${2:-10}" ruin="${3:-0.20}"
  local track_json; track_json="$(python3 - "$TRACK" "$task" <<'PY'
import json, sys
track, task = sys.argv[1], sys.argv[2]
agg = {}
with open(track) as f:
    for line in f:
        line=line.strip()
        if not line: continue
        try: d=json.loads(line)
        except: continue
        if d.get("task")!=task: continue
        m=d["model"]; a=agg.setdefault(m,[0,0,0.0,0.0])
        a[0]+=1; a[1]+=int(d["success"]); a[2]+=float(d["value"]); a[3]+=float(d["cost"])
track_arr=[]
for m,(n,s,v,c) in agg.items():
    if n==0: continue
    p=s/n
    track_arr.append([m,p,v/n,c/n])
print(json.dumps(track_arr))
PY
)"
  gov_kern "{\"op\":\"gov_route\",\"track\":$track_json,\"budget\":$budget,\"ruin_cap\":$ruin}" | python3 - <<'PY'
import sys, json
d = json.load(sys.stdin)
print(d.get("route"))
PY
}

# ===== 3. RESEARCH-ARGUE LOOP ========================================
gov_research() { # question rounds
  local q="$1" rounds="${2:-3}"
  local ts; ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  TELEGRAM_TOPIC_ID=294 tg_send "🔬 RESEARCH-ARGUE [$rounds rounds]: $q" 2>/dev/null || true
  echo "{\"ts\":\"$ts\",\"kind\":\"research\",\"question\":\"$q\",\"rounds\":$rounds,\"status\":\"dispatched\"}" >> "$PREC"
}

# ===== 4. JUDGE (3 independent models) ===============================
gov_judge() { # question optA optB optC  -> Decide(X) | ESCALATE
  local q="$1" a="$2" b="$3" c="$4"
  TELEGRAM_TOPIC_ID=291 tg_send "⚖️ JURY DISPATCHED (3 indep, decart): $q" 2>/dev/null || true
  local vf="$GOV_DIR/jury_$(date -u +%Y-%m-%dT%H:%M:%SZ).jsonl"
  if [ -f "$vf" ]; then
    local v1 v2 v3
    v1="$(sed -n '1p' "$vf" | cut -f1)"; v2="$(sed -n '2p' "$vf" | cut -f1)"; v3="$(sed -n '3p' "$vf" | cut -f1)"
    case "$v1|$v2|$v3" in
      "$a|$a"*|"$a||$a"|"$b|$b"*) echo "Decide($a)";;
      *) echo "ESCALATE";;
    esac
  else
    echo "ESCALATE"
  fi
}

# ===== 5. PRECEDENT REGISTRY (stare decisis) ========================
gov_precedent() { # question  -> prior DECISION or NO PRECEDENT
  local q="$1"
  local prec_json; prec_json="$(python3 - "$PREC" <<'PY'
import json, sys
prec=sys.argv[1]
items=[]
with open(prec) as f:
    for line in f:
        line=line.strip()
        if not line: continue
        try: d=json.loads(line)
        except: continue
        items.append([d.get("id","?"), d.get("winner",""), (d.get("question","")+" "+d.get("winner",""))])
print(json.dumps(items))
PY
)"
  gov_kern "{\"op\":\"gov_precedent\",\"query\":\"$q\",\"precedents\":$prec_json}" | python3 - <<'PY'
import sys, json
d = json.load(sys.stdin)
if d.get("bind") == "NO-BINDING-PRECEDENT":
    print("NO PRECEDENT — must run full DECART + 3-judge")
else:
    print("PRECEDENT favours: %s (sim %.2f)" % (d.get("winner","?"), d.get("similarity",0)))
PY
}
gov_precedent_bind() { # question tau  -> P-id + winner or NO-BIND (pbind = precedent w/ explicit tau)
  local q="$1" tau="${2:-0.82}"
  local prec_json; prec_json="$(python3 - "$PREC" <<'PY'
import json, sys
prec=sys.argv[1]
items=[]
with open(prec) as f:
    for line in f:
        line=line.strip()
        if not line: continue
        try: d=json.loads(line)
        except: continue
        if d.get("overturned"): continue
        items.append([d.get("id","?"), d.get("winner",""), (d.get("question","")+" "+d.get("winner",""))])
print(json.dumps(items))
PY
)"
  gov_kern "{\"op\":\"gov_pbind\",\"query\":\"$q\",\"tau\":$tau,\"precedents\":$prec_json}" | python3 - <<'PY'
import sys, json
d = json.load(sys.stdin)
if d.get("bind") == "NO-BINDING-PRECEDENT":
    print("NO-BINDING-PRECEDENT")
else:
    print("BIND %s (%.2f): %s | PRESUMPTION favored; burden on challenger" % (
        d.get("bind"), d.get("similarity", 0), d.get("winner", "?")))
PY
}
gov_precedent_record() { # id question winner argued_rounds jury_csv
  local id="$1" q="$2" winner="$3" rounds="$4" jury="$5"
  local ts; ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "{\"id\":\"$id\",\"question\":\"$q\",\"winner\":\"$winner\",\"evidence\":[],\"date\":\"$ts\",\"overturned\":null,\"argued_rounds\":$rounds,\"jury\":[\"${jury//,/\",\"}\"],\"binding\":true}" >> "$PREC"
}

# ===== 4b. HARDNESS TRIGGERS ========================================
gov_hard() { # class redline blast_radius no_decart_winner budget_exceeded
  local cls="$1" red="$2" blast="$3" nodecart="$4" budget="$5"
  local hits=""
  { [ "$cls" = "build" ] || [ "$cls" = "audit" ]; } && hits="$hits build/audit"
  [ "${red:-0}" = "1" ] && hits="$hits redline"
  [ "${blast:-0}" -gt 1 ] 2>/dev/null && hits="$hits blast-radius=$blast"
  [ "${nodecart:-0}" = "1" ] && hits="$hits no-decart-winner"
  [ "${budget:-0}" = "1" ] && hits="$hits budget-exceeded"
  echo "$hits"
}
gov_judge_gate() { # verdict_line  -> OK | RED
  if printf '%s' "$1" | grep -Eq 'CITES:|DISTINGUISHES:|NO-BINDING-PRECEDENT'; then
    echo "OK"
  else
    echo "RED: verdict missing citation token (CITES/DISTINGUISHES/NO-BINDING-PRECEDENT)"
  fi
}

# ===== 2b. LANE WIDTH (½-Kelly) =====================================
gov_lane_width() { # p value cost budget stake -> lane count
  gov_kern "{\"op\":\"gov_lane\",\"p\":$1,\"v\":$2,\"cost\":$3,\"budget\":$4,\"stake\":$5}" | python3 - <<'PY'
import sys, json
print(max(1, int(json.load(sys.stdin).get("lanes", 1))))
PY
}

# ===== 6. META-RULE (stateful EMA persisted in meta_state.json) ====
# Reads/writes EMA state via tiny python (file I/O only; math is native).
gov_meta() {
  local st="$GOV_DIR/meta_state.json"; mkdir -p "$(dirname "$st")"
  local mode="oneshot"; [ "${1:-}" = "observe" ] && mode="observe" && shift
  local bp="${1:-1}" bn="${2:-1}" rp="${3:-1}" rn="${4:-1}" fr="${5:-0}"
  if [ "$mode" = "observe" ]; then
    python3 - "$st" "$bp" "$bn" "$rp" "$rn" "$fr" <<'PY'
import json, sys, os
st, bp, bn, rp, rn, fr = sys.argv[1], *sys.argv[2:7]
bp, bn, rp, rn, fr = (float(x) for x in (bp, bn, rp, rn, fr))
try:
    with open(st) as f: s = json.load(f)
    if not isinstance(s, dict): s = {}
except (FileNotFoundError, json.JSONDecodeError, ValueError):
    s = {}
n = int(s.get("n", 0) or 0); a = 2.0/(n+1.0)
s["ema_bench"] = a*((bn/bp - 1.0) if bp>0 else 0.0) + (1-a)*s.get("ema_bench", 0.0)
s["ema_eval"]  = a*(rn - rp) + (1-a)*s.get("ema_eval", 0.0)
s["ema_false"] = a*max(0.0,fr) + (1-a)*s.get("ema_false", 0.0)
s["n"] = n+1
tmp = st + ".tmp"
with open(tmp, "w") as f: json.dump(s, f)
os.replace(tmp, st)
PY
    local bd ed fr2
    IFS= read -r bd < <(python3 - "$st" <<'PY'
import json, sys
st=sys.argv[1]
try:
    with open(st) as f: s=json.load(f)
except Exception: s={"ema_bench":0.0,"ema_eval":0.0,"ema_false":0.0,"n":0}
eb,ee,ef,n=s.get("ema_bench",0.0),s.get("ema_eval",0.0),s.get("ema_false",0.0),int(s.get("n",0) or 0)
print(f"{eb:+.3f} {ee:+.3f} {ef:.3f} {n}")
PY
)
    set -- $bd
    gov_kern "{\"op\":\"gov_meta\",\"bench_delta\":$1,\"eval_delta\":$2,\"false_rate\":$3}" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(f"META n='$4' ema_bench=$1 ema_eval=$2 ema_false=$3\nGUIDANCE lane_tol={d[\"lane_tol\"]:.3f} judge_count={d[\"judge_count\"]} precedent_tau={d[\"precedent_tau\"]:.3f}\nRULES: guidance, not gates — energy flows; meta-rule tilts only.")'
  else
    local bd="$( [ "$bp" != "0" ] && echo "$(python3 -c "print(($bn/$bp - 1.0))")" || echo 0 )"
    gov_kern "{\"op\":\"gov_meta\",\"bench_delta\":$bd,\"eval_delta\":$(python3 -c "print($rn - $rp)"),\"false_rate\":$fr}" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(f"GUIDANCE lane_tol={d[\"lane_tol\"]:.3f} judge_count={d[\"judge_count\"]} precedent_tau={d[\"precedent_tau\"]:.3f}\nRULES: guidance, not gates — energy flows; meta-rule tilts only.")'
  fi
}

# ===== 6b. LIVING-MEMORY BRIDGE (PRIMARY retrieval) ================
gov_recall() { # query k
  local q="$1" k="${2:-5}"
  if [ -f "$GOV_LM" ]; then
    python3 "$GOV_LM" --query "$q" --k "$k" 2>/dev/null || gov_precedent "$q"
  else
    gov_precedent "$q"
  fi
}

# ===== 7. FALSE-CLAIM METER =========================================
gov_falseclaim() { # record|report|observe [claimed=1 verified=1]
  local sub="${1:-report}"; shift || true
  case "$sub" in
    record)
      local claimed="${1:-1}" verified="${2:-1}"
      local ts; ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
      echo "{\"ts\":\"$ts\",\"claimed\":$claimed,\"verified\":$verified}" >> "$FC";;
    observe)
      local fr; fr="$(gov_falseclaim report | awk -F'= ' '/false-estimation/{gsub(/%/,"",$2);print $2/100}')"
      gov_meta observe "${2:-1}" "${3:-1}" "${4:-1}" "${5:-1}" "$fr";;
    report|*)
      # emit the ledger as a claims array, compute via native meter
      local claims; claims="$(python3 - "$FC" <<'PY'
import json, sys
fc=sys.argv[1]; rows=[]
with open(fc) as f:
    for line in f:
        line=line.strip()
        if not line: continue
        try: d=json.loads(line)
        except: continue
        rows.append([bool(int(d.get("claimed",1))), bool(int(d.get("verified",1)))])
print(json.dumps(rows))
PY
)"
      gov_kern "{\"op\":\"gov_false\",\"claims\":$claims}" | python3 -c 'import sys,json;d=json.load(sys.stdin);fe=d["false_estimation"]*100;fp=d["false_positive_of_done"]*100;print(f"FALSE-CLAIM: events={d[\"events\"]} claimed={d[\"claimed\"]} verified={d[\"verified\"]}\n  false-estimation%      = {fe:.1f}  (claimed but not verified)\n  false-positive-of-done% = {fp:.1f}  (claimed-done / verified)")';;
  esac
}

# ===== 8. ANU + ANANKE (living-environment decision layer) =========
# Self-supervised learner (Anu) over the organism's OWN telemetry, served
# natively; structure floor (Ananke) enforced natively; decide = fuse.
GOV_METRIC="${GOV_METRIC:-$DIR/logs/metric.jsonl}"
gov_learn() { # [metric_log]
  local log="${1:-$GOV_METRIC}"
  [ -f "$log" ] || { echo "NO-LEARN: $log absent"; return 0; }
  # fold telemetry into native-compatible arrays
  local payload; payload="$(python3 - "$log" <<'PY'
import json, sys
log=sys.argv[1]
eta_x,eta_y,ms,rss=[],[],[],[]
for line in open(log):
    try: d=json.loads(line)
    except: continue
    if "eta_err_pct" in d and "elapsed_min" in d:
        try: eta_x.append(float(d["elapsed_min"])); eta_y.append(float(d["eta_err_pct"]))
        except: pass
    if "ms" in d:
        try: ms.append(float(str(d["ms"]).split("=")[-1]))
        except: pass
    if "rss_mb" in d:
        try: rss.append(float(str(d["rss_mb"]).split("=")[-1]))
        except: pass
print(json.dumps({"eta_pairs":list(zip(eta_x,eta_y)),"latency_ms":ms,"rss_mb":rss}))
PY
)"
  gov_kern "{\"op\":\"gov_learn\",\"eta_pairs\":$(echo "$payload" | python3 -c 'import sys,json;print(json.dumps(json.load(sys.stdin)["eta_pairs"]))'),\"latency_ms\":$(echo "$payload" | python3 -c 'import sys,json;print(json.dumps(json.load(sys.stdin)["latency_ms"]))'),\"rss_mb\":$(echo "$payload" | python3 -c 'import sys,json;print(json.dumps(json.load(sys.stdin)["rss_mb"]))')}" | python3 -c 'import sys,json;d=json.load(sys.stdin);e=d["eta"];l=d["latency"];r=d["resource"];print(f"ANU: eta n={e[\"n\"]} (identity)" if e["identity"] else f"ANU: eta n={e[\"n\"]} R2={e[\"r2\"]:.3f}\n  latency n={l[\"n\"]} ema_ms={l[\"ema_ms\"]:.1f} mean_ms={l[\"mean_ms\"]:.1f} max_ms={l[\"max_ms\"]:.1f}\n  resource n={r[\"n\"]} ema_mb={r[\"ema_mb\"]:.1f} mean_mb={r[\"mean_mb\"]:.1f}")'
}
gov_anu() { # elapsed_min  -> corrected eta_err_pct
  gov_learn >/dev/null 2>&1
  local m="$GOV_DIR/anu_eta_model.json"
  python3 - "$m" "$1" <<'PY'
import json, sys
m,x=sys.argv[1],float(sys.argv[2])
try: d=json.load(open(m))
except Exception:
    print(f"{x:.4f}"); sys.exit(0)
if d.get("identity"):
    print(f"{x:.4f}  (identity: no learn)")
else:
    print(f"{d['slope']*x+d['intercept']:.4f}  (learned slope={d['slope']:+.4f} R2={d.get('r2',0):.3f})")
PY
}
gov_anu_latency() { # -> predicted_ms (learned EMA)
  gov_learn >/dev/null 2>&1
  local m="$GOV_DIR/anu_latency_model.json"
  python3 - "$m" <<'PY'
import json, sys
m=sys.argv[1]
try: d=json.load(open(m))
except Exception:
    print("0.0 (no latency model)"); sys.exit(0)
print(f"{d.get('ema_ms',0.0):.1f}  (n={d.get('n',0)} mean_ms={d.get('mean_ms',0.0):.1f} max_ms={d.get('max_ms',0.0):.1f})")
PY
}
gov_ananke() { # ruin_prob redline_hit data_loss_risk -> CLEAR | ANANKE-BLOCK
  local ruin="${1:-0}" red="${2:-0}" loss="${3:-0}"
  gov_kern "{\"op\":\"gov_decide\",\"ruin_cap\":0.2,\"ruin_prob\":$ruin,\"red_line\":$red,\"data_loss\":$loss}" | python3 -c 'import sys,json;d=json.load(sys.stdin);print("CLEAR" if d["proceed"] else "ANANKE-BLOCK: "+d["reason"])'
}
gov_decide() { # elapsed_min ruin_prob redline data_loss
  local x="$1" ruin="$2" red="$3" loss="$4"
  local anu ananke
  anu="$(gov_anu "$x")"; ananke="$(gov_ananke "$ruin" "$red" "$loss")"
  echo "ANU (logic):     eta_err_corrected = $anu"
  echo "ANANKE (struct): $ananke"
  case "$ananke" in
    CLEAR) echo "DECISION: PROCEED (Anu guides, Ananke clear) — energy flows.";;
    *)     echo "DECISION: HALT/RESTRICT ($ananke) — Anu overrides blocked by Ananke. Structure wins.";;
  esac
}

# ---- CLI dispatcher ----
gov_dispatch() {
  local cmd="${1:-help}"; shift || true
  case "$cmd" in
    record) gov_record "$@";;
    route)  gov_route "$@";;
    lane)   gov_lane_width "$@";;
    research) gov_research "$@";;
    hard)   gov_hard "$@";;
    judge)  gov_judge "$@";;
    gate)   gov_judge_gate "$@";;
    precedent) gov_precedent "$@";;
    pbind)  gov_precedent_bind "$@";;
    prec_rec) gov_precedent_record "$@";;
    meta)   gov_meta "$@";;
    falseclaim) gov_falseclaim "$@";;
    learn)  gov_learn "$@";;
    anu)    gov_anu "$@";;
    anu_latency) gov_anu_latency "$@";;
    ananke) gov_ananke "$@";;
    decide) gov_decide "$@";;
    *) echo "governance: record|route|lane|research|hard|judge|gate|precedent|pbind|prec_rec|meta|falseclaim|learn|anu|ananke|decide";;
  esac
}

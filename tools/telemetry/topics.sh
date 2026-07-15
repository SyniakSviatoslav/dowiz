#!/usr/bin/env bash
# topics.sh — dedicated Telegram topic aggregators (2026-07-15).
# Reuses the proven primitives from lib.sh: tg_send (honors TELEGRAM_TOPIC_ID),
# log_event (vectorless JSONL ledger), and ser (native f64 EDGE adapter).
# No new deps, no reinvention: every topic is a `tg_send` to a known message_thread_id.
#
# Topic ids (created this session):
#   267 Hermes (default; DOD plan/step/retro + monitor land here)
#   272 Hetzner (host-resource heartbeat)
#   291 Planning  (unified plans/tasks/roadmaps/todos, last 7d, chronological)
#   292 Git       (commits/pushes/workflow alerts, both repos)
#   293 Cloudflare(worker logs/alerts; wrangler tail or log-file poll)
#   294 Benchmarks(entropy/eval/bench results from bebop + dowiz)
set -uo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "$DIR/lib.sh"

DOWIZ=/root/dowiz
# Live protocol repo = openbebop (SyniakSviatoslav/OpenBebop). The local /root/bebop-repo
# checkout also carries the archived `bebop.git` as `origin`; the LIVE code is on the
# `openbebop` remote. We watch the `openbebop/main` ref (fetch-then-read, no extra clone).
BEBOP_CHECKOUT=/root/bebop-repo
BEBOP_REF=openbebop/main
ROADMAP="$DOWIZ/docs/design/ROADMAP-GROUND-TRUTH-2026-07-11.md"

# Pull the live openbebop ref so watchers see fresh commits (best-effort; silent on offline).
bebop_fetch() { git -C "$BEBOP_CHECKOUT" fetch openbebop --quiet 2>/dev/null || true; }

# ---- unified plans/tasks aggregator (last 7 days, chronological) ----
# Combines: (a) DOD plan.jsonl, (b) git log both repos, (c) roadmap doc headers.
plans_aggregate() {
  bebop_fetch  # best-effort pull of live openbebop ref before reading
  python3 - "$LOG_DIR" "$DOWIZ" "$BEBOP_CHECKOUT" "$BEBOP_REF" "$ROADMAP" <<'PY'
import json, os, subprocess, sys, datetime
log_dir, dowiz, bebop_co, bebop_ref, roadmap = sys.argv[1:6]
since = (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(days=7)).strftime("%Y-%m-%d")
events = []  # (ts, repo, kind, text)

def add(ts, repo, kind, text):
    events.append((ts, repo, kind, text))

# (a) DOD plans
_dod_plans = []
pj = os.path.join(log_dir, "plan.jsonl")
if os.path.exists(pj):
    for l in open(pj):
        if not l.strip(): continue
        d = json.loads(l)
        _dod_plans.append(d)
        add(d.get("ts", since), "DOD", "plan",
            f"{d.get('id')} — {d.get('title','')[:50]} (⏱{d.get('eta_min',0)}min 🪙{d.get('eta_tokens',0)} 🤖{d.get('agents',1)})")

# (b) git log both repos (last 7d)
# dowiz = local path; openbebop = ref inside the checkout
try:
    out = subprocess.check_output(
        ["git", "-C", dowiz, "log", f"--since={since}",
         "--pretty=format:%ad|%h|%s", "--date=short"],
        stderr=subprocess.DEVNULL).decode().splitlines()
    for line in out:
        if "|" not in line: continue
        d, h, s = line.split("|", 2)
        add(d, "dowiz", "git", f"{h} {s[:60]}")
except Exception:
    pass
try:
    out = subprocess.check_output(
        ["git", "-C", bebop_co, "log", bebop_ref, f"--since={since}",
         "--pretty=format:%ad|%h|%s", "--date=short"],
        stderr=subprocess.DEVNULL).decode().splitlines()
    for line in out:
        if "|" not in line: continue
        d, h, s = line.split("|", 2)
        add(d, "openbebop", "git", f"{h} {s[:60]}")
except Exception:
    pass

# (c) roadmap doc top-level plan items (## N. headers)
if os.path.exists(roadmap):
    cur = None
    for l in open(roadmap):
        if l.startswith("## ") and any(c.isdigit() for c in l[:4]):
            cur = l[3:].strip()
            add(since, "ROADMAP", "plan", cur[:70])

events.sort(key=lambda e: e[0])
ICON = {"DOD": "📋", "dowiz": "🟢", "openbebop": "🔵", "ROADMAP": "🗺", "git": "◆", "plan": "▸"}
def ri(repo): return ICON.get(repo, "•")
# ETA rollup from DOD plans present in the window.
eta_min = sum(int(d.get("eta_min", 0)) for d in _dod_plans)
eta_tok = sum(int(d.get("eta_tokens", 0)) for d in _dod_plans)
n_plans = len(_dod_plans)
print("🗂 UNIFIED PLANS & TASKS — last 7d")
print(f"📊 {len(events)} items · {n_plans} DOD plans · ⏱ ΣETA {eta_min}min · 🪙 Σ~{eta_tok} tok")
print("─" * 32)
cur_day = None
for ts, repo, kind, text in events:
    day = str(ts)[:10]
    if day != cur_day:
        cur_day = day
        print(f"\n📅 {day}")
    print(f"  {ri(repo)} [{repo}] {text}")
PY
}

# ---- git watcher (60s loop, only NEW commits since last seen) ----
git_watch_loop() {
  local iv="${1:-60}"
  local last="$LOG_DIR/.git_watch_state"
  mkdir -p "$LOG_DIR"
  echo "git-watch: posting new commits from dowiz + openbebop to topic 292 every ${iv}s" >&2
  while true; do
    bebop_fetch
    seen="$(cat "$last" 2>/dev/null || echo '')"
    fresh=""
    # dowiz = local path
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      h="${line%%|*}"
      case "$seen" in *"$h"*) continue;; esac
      fresh="$fresh$line"$'\n'; seen="$seen $h"
    done < <(git -C "$DOWIZ" log --since="7 days ago" --pretty=format:"%h|%ad|%s" --date=short 2>/dev/null)
    # openbebop = live ref inside the checkout
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      h="${line%%|*}"
      case "$seen" in *"$h"*) continue;; esac
      fresh="$fresh$line"$'\n'; seen="$seen $h"
    done < <(git -C "$BEBOP_CHECKOUT" log "$BEBOP_REF" --since="7 days ago" --pretty=format:"%h|%ad|%s" --date=short 2>/dev/null)
    if [ -n "$fresh" ]; then
      printf '%s' "$seen" > "$last"
      while IFS= read -r line; do
        [ -z "$line" ] && continue
        h="$(printf '%s' "$line" | cut -d'|' -f1)"
        d="$(printf '%s' "$line" | cut -d'|' -f2)"
        s="$(printf '%s' "$line" | cut -d'|' -f3-)"
        TELEGRAM_TOPIC_ID=292 tg_send "🔀 git $d $h — $s" || echo "git-watch send failed" >&2
      done <<< "$fresh"
    fi
    sleep "$iv"
  done
}

# ---- cloudflare watcher (persistent loop; proven method: wrangler tail) ----
cf_watch_loop() {
  local iv="${1:-60}"
  local logfile="${CF_LOG_FILE:-$HOME/ops/cf/cf.log}"
  echo "cf-watch: -> topic 293 (proven method: 'wrangler tail'; fallback CF_LOG_FILE poll)" >&2
  local noted=0
  while true; do
    if command -v wrangler >/dev/null 2>&1; then
      echo "cf-watch: wrangler tail -> topic 293" >&2
      TELEGRAM_TOPIC_ID=293 wrangler tail --format json 2>/dev/null | while IFS= read -r ev; do
        TELEGRAM_TOPIC_ID=293 tg_send "☁️ cf: $(printf '%s' "$ev" | head -c 200)" || true
      done
    elif [ -f "$logfile" ]; then
      tail -n 0 -F "$logfile" 2>/dev/null | while IFS= read -r line; do
        TELEGRAM_TOPIC_ID=293 tg_send "☁️ cf: $(printf '%s' "$line" | head -c 200)" || true
      done
    else
      if [ "$noted" -eq 0 ]; then
        TELEGRAM_TOPIC_ID=293 tg_send "☁️ Cloudflare topic ready. Proven method: 'wrangler tail' (install wrangler + auth) or set CF_LOG_FILE=/path/to/cf.log to poll a log file. No live CF signal on this box yet." || true
        noted=1
      fi
    fi
    sleep "$iv"
  done
}

# ---- benchmarks/entropy/eval watcher (60s loop, posts deltas) ----
bench_watch_loop() {
  local iv="${1:-120}"
  local last="$LOG_DIR/.bench_state"
  mkdir -p "$LOG_DIR"
  echo "bench-watch: entropy/eval/bench -> topic 294 every ${iv}s" >&2
  while true; do
    # openbebop entropy_ledger: run the rust test that prints compression_length_bits if present
    if [ -d "$BEBOP_CHECKOUT/rust-core" ]; then
      out="$(cd "$BEBOP_CHECKOUT" && cargo test -p rust-core entropy_ledger -- --nocapture 2>/dev/null | grep -iE 'compression_length_bits|entropy' | tail -3)"
      [ -n "$out" ] && TELEGRAM_TOPIC_ID=294 tg_send "📊 openbebop entropy_ledger: $(printf '%s' "$out" | tr '\n' ' ' | head -c 200)" || true
    fi
    # dowiz eval-layer result if newer than last seen
    res="$DOWIZ/eval-layer/deepeval-result.json"
    if [ -f "$res" ]; then
      mt="$(stat -c %Y "$res" 2>/dev/null || echo 0)"
      pl="$(cat "$last" 2>/dev/null || echo 0)"
      if [ "$mt" -gt "$pl" ] 2>/dev/null; then
        printf '%s' "$mt" > "$last"
        summ="$(python3 -c 'import json;d=json.load(open("'"$res"'"));print(str(d)[:180])' 2>/dev/null)"
        TELEGRAM_TOPIC_ID=294 tg_send "📊 dowiz eval-layer: $summ" || true
      fi
    fi
    sleep "$iv"
  done
}

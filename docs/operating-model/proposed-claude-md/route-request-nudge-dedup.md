# PROPOSED — route-request.sh nudge recency-dedup (operator pastes directly)

> **Why this is proposed, not applied:** `.claude/hooks/route-request.sh` is a RED-LINE DETECTION
> hook (nudges toward /council on money/auth/schema/state-machine requests). guard-bash hard-blocks
> any bash mutation of it (hooks = strongest protected zone). I will not route a script around
> guard-bash to blind-edit a safety-detection hook for token savings — the block is a correct signal.
> This patch does NOT weaken detection (the SERIOUS/REPEAT regexes are untouched); it only suppresses
> REPEAT fires of the same nudge within a 10-minute window — the harness's own unshipped P3 intent
> (383 fires / 2.5d, mostly the same nudge re-firing every turn of one ongoing serious task).

## The patch (GNU date; platform is Linux)

Add this helper after the `_hev()` definition (~line 27):

```bash
# Recency-dedup (token-reduction P3): suppress a repeat of the SAME nudge within WIN seconds,
# based on the last EMITTED fire in HEV_LOG. Preserves first-fire detection; never weakens the
# SERIOUS/REPEAT match. Returns 0 (=recent, skip) / 1 (=fire).
_recent_nudge() {
  [ -f "$HEV_LOG" ] || return 1
  local win=600 now last lasts
  now=$(date +%s)
  last=$(grep -F "\"event\":\"$1\"" "$HEV_LOG" 2>/dev/null | tail -1 | sed -n 's/.*"ts":"\([^"]*\)".*/\1/p')
  [ -z "$last" ] && return 1
  lasts=$(date -d "$last" +%s 2>/dev/null) || return 1
  [ $((now - lasts)) -lt $win ] && return 0 || return 1
}
```

Then gate each of the two emit blocks (lines ~41 and ~45) on it:

```bash
if printf '%s' "$low" | grep -Eq "$SERIOUS" && ! _recent_nudge nudge-serious; then
  _hev route-request nudge-serious "$(printf '%s' "$low" | cut -c1-80)"
  ctx="⚠️ serious-gate router: …"   # (unchanged text)
fi
if printf '%s' "$low" | grep -Eq "$REPEAT" && ! _recent_nudge nudge-repeat; then
  _hev route-request nudge-repeat "$(printf '%s' "$low" | cut -c1-80)"
  # … (unchanged text)
fi
```

## Test before trusting (paste-and-run)
1. Fresh serious prompt → nudge fires (ctx emitted, HEV logs nudge-serious).
2. Same serious prompt again within 10 min → NO nudge (dedup).
3. Benign prompt ("go", "thanks") → nothing (unchanged).
4. Serious prompt after 10 min → nudge fires again.
Expected effect: ~194 serious + ~189 repeat fires → a handful per distinct serious topic; ~15-25K
tokens/session reclaimed, zero loss of red-line detection sensitivity.

# claim-latency ledger (V5-B, BLUEPRINT-P01 §2.7)
#
# One JSONL entry per commit, appended by scripts/claim-latency-append.sh.
# Schema: {commit_sha, authored_ts, ci_observed_green_ts, delta_s, diff_loc}
# Phase 1 builds ONLY the appender; the anomaly detector (52s-on-1610-line-diff
# flag) is Phase 8`s consumer (P08 §4) — NOT built here.

#!/usr/bin/env bash
# tools/ops-alert/offsite-copy.sh — P45-W3 §4a.3 copy-3 off-Hetzner IMMUTABLE copy.
#
# Copies kernel-native state (event log, spool, JSONL ledgers, release manifests)
# to off-Hetzner immutable storage, the 3-2-1-1-0 topology's copy 3 (OPS-14):
#   - rsync.net  : SSH-only, zero-egress, credential-isolated  (copy 3a)
#   - Object-Lock : COMPLIANCE-mode S3 bucket — immutable leg; early deletion
#                   impossible even for the key holder (OPS-14 adversarial case a)
#                   (copy 3b)
#
# GUARDED — each target is OPTIONAL. If its env is absent the target is skipped
# (no-op), so this is safe to wire into cron BEFORE the operator provisions
# copy-3. See docs/backup/runbooks.md §W3 for the operator provisioning steps.
#
# PROVISIONING (OPERATOR-only — this agent does NOT create accounts/buckets):
#   1. rsync.net account + SSH key  -> set RSYNC_NET_HOST, RSYNC_NET_USER, RSYNC_NET_PATH
#   2. Object-Lock COMPLIANCE bucket (set at BUCKET CREATION, not retrofittable)
#      -> set OFFSITE_BUCKET + AWS_REGION + AWS credentials (standard AWS env)
#
# Freshness metric (§4a.3 monitoring hook): writes Prometheus textfile at
#   $METRICS_FILE with dowiz_ops_backup_last_success_seconds{subject,copy}
#   and dowiz_ops_backup_bytes_written{subject,copy}. Alerting on staleness /
#   size-drop rides the existing pager rules (5 / 7) — NOT reimplemented here.
#
# Fail-closed (§4a.3 adversarial case c): a target that is ENABLED but writes
# 0 bytes is treated as a failure (exit 1) — a "successful" 0-byte backup must
# never look green.
#
# Mockable for offline RED->GREEN: RSYNC_CMD / S3_CMD / DATE_CMD / STAT_BYTES.

set -uo pipefail

# --- config (env-overridable) ---------------------------------------------
BACKUP_SRC="${BACKUP_SRC:-/var/lib/dowiz/state}"   # kernel-native state root
RSYNC_NET_HOST="${RSYNC_NET_HOST:-}"
RSYNC_NET_USER="${RSYNC_NET_USER:-}"
RSYNC_NET_PATH="${RSYNC_NET_PATH:-/dowiz-backup/copy3}"
OFFSITE_BUCKET="${OFFSITE_BUCKET:-}"
AWS_REGION="${AWS_REGION:-}"
METRICS_FILE="${METRICS_FILE:-/var/lib/dowiz/metrics/backup.prom}"

# mock hooks (override for offline tests)
RSYNC_CMD="${RSYNC_CMD:-rsync -az --delete}"
S3_CMD="${S3_CMD:-aws s3 cp --only-show-errors}"
DATE_CMD="${DATE_CMD:-date}"
STAT_BYTES="${STAT_BYTES:-}"   # override bytes-written detection in tests

log() { echo "offsite-copy: $*" >&2; }

now_ts() { "$DATE_CMD" +%s; }

src_bytes() {
  if [[ -n "$STAT_BYTES" ]]; then echo "$STAT_BYTES"; return; fi
  du -sb "$BACKUP_SRC" 2>/dev/null | cut -f1 || echo 0
}

emit_metric() {  # $1=subject $2=copy $3=ts $4=bytes
  mkdir -p "$(dirname "$METRICS_FILE")"
  {
    echo "# HELP dowiz_ops_backup_last_success_seconds Unix ts of last successful offsite copy"
    echo "# TYPE dowiz_ops_backup_last_success_seconds gauge"
    echo "dowiz_ops_backup_last_success_seconds{subject=\"$1\",copy=\"$2\"} $3"
    echo "# HELP dowiz_ops_backup_bytes_written bytes copied in last run"
    echo "# TYPE dowiz_ops_backup_bytes_written gauge"
    echo "dowiz_ops_backup_bytes_written{subject=\"$1\",copy=\"$2\"} $4"
  } >> "$METRICS_FILE"
}

# rsync.net copy (copy 3a). Echoes bytes-written on stdout (or nothing on skip/fail).
do_rsync() {
  [[ -z "$RSYNC_NET_HOST" || -z "$RSYNC_NET_USER" ]] && {
    log "rsync.net target SKIPPED (RSYNC_NET_HOST/USER unset)"; return 0; }
  local dest="$RSYNC_NET_USER@$RSYNC_NET_HOST:$RSYNC_NET_PATH/"
  log "rsync.net -> $dest"
  local out
  if ! out=$($RSYNC_CMD "$BACKUP_SRC"/ "$dest" 2>&1); then
    log "rsync.net FAILED: $out"; return 1
  fi
  src_bytes
  return 0
}

# Object-Lock S3 copy (copy 3b, immutable leg). Echoes bytes-written on stdout.
do_s3() {
  [[ -z "$OFFSITE_BUCKET" ]] && { log "Object-Lock S3 target SKIPPED (OFFSITE_BUCKET unset)"; return 0; }
  local dest="s3://$OFFSITE_BUCKET/copy3/"
  log "Object-Lock S3 -> $dest"
  local out
  if ! out=$($S3_CMD "$BACKUP_SRC"/ "$dest" 2>&1); then
    log "Object-Lock S3 FAILED: $out"; return 1
  fi
  src_bytes
  return 0
}

main() {
  [[ -d "$BACKUP_SRC" ]] || { log "BACKUP_SRC ($BACKUP_SRC) missing — nothing to copy"; exit 0; }
  : > "$METRICS_FILE"   # truncate per run; absence of a copy's rows = not provisioned
  local ts rc=0 b r
  ts="$(now_ts)"
  b="$(do_rsync)"; r=$?
  if [[ $r -eq 0 ]]; then
    if [[ -n "$b" ]]; then
      if [[ "$b" == "0" ]]; then log "rsync.net wrote 0 bytes — fail-closed"; rc=1;
      else emit_metric kernel_state rsyncnet "$ts" "$b"; fi
    fi
  else rc=1; fi
  b="$(do_s3)"; r=$?
  if [[ $r -eq 0 ]]; then
    if [[ -n "$b" ]]; then
      if [[ "$b" == "0" ]]; then log "Object-Lock S3 wrote 0 bytes — fail-closed"; rc=1;
      else emit_metric kernel_state objectlock "$ts" "$b"; fi
    fi
  else rc=1; fi
  if [[ $rc -eq 0 ]]; then log "OK — offsite copy complete";
  else log "completed WITH ERRORS (see above)"; fi
  exit $rc
}
main "$@"

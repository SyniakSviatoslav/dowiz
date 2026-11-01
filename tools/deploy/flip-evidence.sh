# Sourced by deploy.sh and rollback.sh: ONE LIST OF WHAT SAYS A VENUE WAS FLIPPED (W-MG-TIDY; until now deploy's exit 14 read
# two witnesses and rollback's exit 53 four, so a build could ship past a flip it would then refuse to return to).
# Reads: $FLIP_MARK $VERSIONS $REPO. Prints the first witness, or nothing.
#
# THE MARK IS NOT THE ONLY WITNESS (W-MG-TRANS H1): it lives in one cache directory on one box, and "no mark" used to mean
# "no flip", which switched prod over flipped venues. So: the mark; any version recorded with a list; the tree's wrangler.toml
# (unparsable counts); any commit that ever set a list. The last is a line regex over what TOML can spell (a bare or quoted
# key, indented, inside an inline `vars = { .. }`), comments excluded. `-G` is an extended regex.
FLIP_RE="^[^#]*SHA256_VENUES[\"']?[[:space:]]*=[[:space:]]*[\"'][^\"']"
flip_evidence() {
  if [ -f "$FLIP_MARK" ]; then head -1 "$FLIP_MARK"; [ -s "$FLIP_MARK" ] || echo "flip mark $FLIP_MARK"; return; fi
  line=$(grep -E ' sha256_venues=[^ ]' "$VERSIONS" 2>/dev/null | tail -1)
  if [ -n "$line" ]; then echo "versions.log: $line"; return; fi
  toml=$REPO/workers/api/wrangler.toml
  if ! var=$(python3 "$(dirname "${BASH_SOURCE[0]}")/scheme_var.py" "$toml" 2>/dev/null); then echo "$toml does not parse"; return; fi
  if [ -n "${var#* }" ]; then echo "$toml: SHA256_VENUES=\"${var#* }\""; return; fi
  commit=$(git -C "$REPO" log -1 --format=%h -G"$FLIP_RE" -- workers/api/wrangler.toml 2>/dev/null)
  if [ -n "$commit" ]; then echo "commit $commit set SHA256_VENUES"; fi
}

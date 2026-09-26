#!/bin/sh
# Publish checked cuts.
#   sh tools/learn/publish.sh OUT_ROOT [ID...]   into the staging tree ($LEARN_STAGE or ~/.cache/dowiz-learn/media,
#                                                NEVER public/: the videos are gated) + the repo's manifest
#                                                workers/api/public/learn/media/manifest.json (no media in it);
#                                                a cut failing check.sh is held back and named
#   sh tools/learn/publish.sh --manifest-only
#   sh tools/learn/publish.sh --r2 [--dry-run]   the staging tree -> R2 bucket dowiz-learn (gated route)
# --r2 sources /root/.cf_deploy_token (the ONLY token file tried; never printed) and puts a probe
# object FIRST: a token without R2 write stops there with exit 4 and says so. Logic and tests:
# publish.mjs / publish.test.mjs.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
case " $* " in
  *" --r2 "*)
    case " $* " in *" --dry-run "*) ;; *)
      TOKEN_FILE=${LEARN_TOKEN_FILE:-/root/.cf_deploy_token}
      [ -r "$TOKEN_FILE" ] || { echo "publish: REFUSED -- no token file $TOKEN_FILE; nothing uploaded" >&2; exit 4; }
      set -a; . "$TOKEN_FILE"; set +a ;;
    esac ;;
esac
node "$HERE/publish.mjs" "$@"
rc=$?
[ "$rc" -eq 4 ] && echo "publish: R2 upload REFUSED (rc=4) -- see the lines above; the tree copy is unaffected" >&2
exit $rc

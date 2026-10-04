#!/bin/sh
# no-tracking's proof: a gate is triggered before it is trusted (W-MR0 2026-10-04, GATE-2).
# Each case plants one file in a SCRATCH copy of workers/api/public/store; the tree is never touched.
#   1. the storefront as it is                                       -> pass (0)
#   2. a planted navigator.sendBeacon                                -> refuse (1)
#   3. an IntersectionObserver whose handler calls fetch             -> refuse (1)
#   4. a scroll listener that calls a local function which fetches   -> refuse (1)
#   5. `dowiz.taste` inside a JSON.stringify request body            -> refuse (1)
#   6. `dw_profile` inside fetch(...)'s arguments                    -> refuse (1)
#   7. an IntersectionObserver that only toggles a class            -> pass (0)
#   8. the profile key read and written only to localStorage         -> pass (0)
#   9. an empty store directory                                      -> refuse (2): read nothing
#  10. `taste_sync` in an order body behind notObjected()          -> pass (0)
#  11. the same field sent without the objection check               -> refuse (1)
#  12. `guest_taste` scored on the device and kept in localStorage    -> pass (0)
#  13. a canvas fingerprint (toDataURL) hashed into a request         -> refuse (1)
#  14. navigator.userAgent sent as a value                            -> refuse (1)
#  15. /iPhone/.test(navigator.userAgent), a yes/no                   -> pass (0)
#  16. hardwareConcurrency read                                       -> refuse (1)
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
S=$(mktemp -d)
trap 'rm -rf "$S"' EXIT
fail=0
R="$S/r"; D="$R/workers/api/public/store"
fresh() { rm -rf "$R"; mkdir -p "$D"; cp "$REPO"/workers/api/public/store/*.js "$D/"; }
want() { # want <rc> <label>
  node "$HERE/no-tracking.mjs" "$R" >"$S/out" 2>&1; rc=$?
  echo "prove: $2 -> rc=$rc (want $1): $(tail -1 "$S/out")"
  [ "$rc" -eq "$1" ] || { cat "$S/out"; fail=1; }
}
fresh; want 0 "storefront as it is"
fresh; echo "addEventListener('pagehide', () => navigator.sendBeacon('/api/x', '1'));" > "$D/zz.js"; want 1 "planted sendBeacon"
fresh; printf 'const io = new IntersectionObserver(es => {\n  for (const e of es) fetch("/api/seen", { method: "POST" });\n});\n' > "$D/zz.js"; want 1 "IntersectionObserver calls fetch"
fresh; printf 'function report(y){ return fetchRemembered({ y }); }\nwindow.addEventListener("scroll", () => report(scrollY));\n' > "$D/zz.js"; want 1 "scroll listener via a local function"
fresh; printf 'const t = localStorage.getItem("dowiz.taste.v1");\nfetch("/api/o", { method: "POST", body: JSON.stringify({ items: [], t: localStorage.getItem("dowiz.taste.v1") }) });\n' > "$D/zz.js"; want 1 "dowiz.taste in a request body"
fresh; printf 'fetch(`/api/o?p=${localStorage.dw_profile_v1}`);\n' > "$D/zz.js"; want 1 "dw_profile in fetch arguments"
fresh; printf 'const io = new IntersectionObserver(es => { for (const e of es) e.target.classList.add("in"); });\n' > "$D/zz.js"; want 0 "IntersectionObserver toggles a class"
fresh; printf 'const k = "dowiz.taste.v1";\nlocalStorage.setItem(k, localStorage.getItem("dowiz.taste.v1") || "{}");\n' > "$D/zz.js"; want 0 "profile key stays in localStorage"
rm -rf "$R"; mkdir -p "$D"; want 2 "empty store directory"
fresh; printf 'const body = { items: [], ...(notObjected() ? { taste_sync: syncVector() } : {}) };\nfetch("/api/o", { method: "POST", body: JSON.stringify(body) });\n' > "$D/zz.js"; want 0 "taste_sync behind notObjected()"
fresh; printf 'const body = { items: [], taste_sync: syncVector() };\nfetch("/api/o", { method: "POST", body: JSON.stringify(body) });\n' > "$D/zz.js"; want 1 "taste_sync without the objection check"
fresh; printf 'const c = document.createElement("canvas");\nconst id = c.toDataURL();\nfetch("/api/o", { method: "POST", body: JSON.stringify({ id }) });\n' > "$D/zz.js"; want 1 "canvas fingerprint"
fresh; printf 'fetch("/api/o", { method: "POST", body: JSON.stringify({ ua: navigator.userAgent }) });\n' > "$D/zz.js"; want 1 "userAgent sent as a value"
fresh; printf 'const IOS2 = /iPhone|iPad/.test(navigator.userAgent);\n' > "$D/zz.js"; want 0 "userAgent as a yes/no"
fresh; printf 'const cores = navigator.hardwareConcurrency;\n' > "$D/zz.js"; want 1 "hardwareConcurrency read"
fresh; printf 'const guest_taste = { maki: 3 };\nlocalStorage.setItem("x", JSON.stringify(guest_taste));\n' > "$D/zz.js"; want 0 "guest_taste on the device"
[ "$fail" -eq 0 ] && echo "no-tracking.prove: GREEN -- 16 of 16 cases" || echo "no-tracking.prove: RED"
exit "$fail"

Programs the parser ACCEPTS today (exit 0) although they are malformed: a call's argument
list closed by `]` (d08) and a body `let` followed by another `let` without `;` (d10). The
progress-guard parser skips what it cannot place. Not run by diag_check.sh (it globs
bench/diag_neg/*.bp only); T90 step 2 = make these exit 95 at the hand-counted position
(d08 5:27, d10 3:13) without rejecting any gate or bebop.bp itself.

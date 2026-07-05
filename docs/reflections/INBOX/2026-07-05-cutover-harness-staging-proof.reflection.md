# Reflection — cutover harness build + S1 staging mechanism proof (2026-07-05)

Qualified: closed a stage (cutover-half opened: harness live dark on staging), ≥3 files,
red-line-adjacent (migrations draft, deploy infra). Worker: lead session (Fable).

## 1. flyctl ssh-console banner corrupts captured secrets

**WHERE:** Rust staging app crashlooped 10× on boot ("must be a postgres:// URL");
migration runner failed with `getaddrinfo EAI_AGAIN base`.
**WHY (causal):** `flyctl ssh console -C <cmd>` writes its own status line
("No machine specified, using <id>…") to **stdout**, not stderr. Any `$(… | tr -d '\n')`
capture concatenates the banner into the value. Two independent consumers (fly secrets set,
node-pg-migrate URL) failed the same way before the shape of the corruption was visible —
the value was redacted for display, so the defect hid inside the very hygiene that kept the
secret unprinted.
**FIX APPLIED:** every capture now shape-asserts (`grep '^postgres://'`).
**RATCHET CANDIDATE:** a lesson keyed on `flyctl ssh console` capture; better primitive:
`flyctl machine exec` or `--machine <id>` (bannerless) for env reads.

## 2. flycast port semantics — proxy service port, not internal_port

**WHERE:** first S1 flip auto-degraded within 4s: `upstream-unhealthy-at-request`.
**WHY (causal):** `.flycast` rides the Fly **proxy**: the reachable port is the service
port (80 for [http_service]), while `internal_port` (8080) is only the machine-local bind.
The upstream URL `…flycast:8080` was structurally dead. Two networking planes (proxy vs
6PN .internal) have opposite port semantics; the ADR's example URL carried the trap.
**SAFETY MACHINE WORKED:** the health breaker + at-request degrade returned S1 to Node
globally (DB row `updated_by='auto-degrade: upstream-unhealthy-at-request'`) with zero
user-visible failures (GET fallthrough covered the window). First live firing = design proven.
**RATCHET CANDIDATE:** deploy-draft comment updated → also assert healthz THROUGH the
front-door's exact URL as a deploy exit item (done manually this time).

## 3. Post-edit red-line gate fires on pre-existing lines (false positive)

**WHERE:** editing packages/config/src/index.ts (added 4 CUTOVER_* env vars) tripped
"RED-LINE: raw PII".
**WHY (causal):** post-edit-gates.sh greps the WHOLE FILE for red-line patterns on any
edit; line 111 (`BACKUP_PII_FIELDS` — a field-NAME list containing `customer_phone`)
pre-dates the edit and matches. Every future edit to this file will trip the gate
regardless of content.
**RATCHET CANDIDATE (human-gated — hooks are protect-paths):** make the red-line scan
diff-aware (added lines only), or an inline allowlist marker for field-name literals.

## 4. First live parity-oracle catches: null-vs-absent + prod-default base URL

**WHERE:** S1 flip live deep-diff of /public/locations/demo/menu → 50 leaf diffs.
**WHY (causal):** (a) serde `skip_serializing_if = "Option::is_none"` is Rust-idiomatic
but Node serializes `imageUrl: null` unconditionally — absent-vs-null is a contract shape
divergence invisible to typecheckers on both sides; (b) the Rust twin defaults
APP_BASE_URL to the PROD host in code — unset env on staging silently emitted prod URLs.
**FIX APPLIED:** dto emits null (commit 48d97bed, 828 cargo tests green); APP_BASE_URL
set per-env.
**RATCHET CANDIDATES:** (1) sweep the remaining 5 `skip_serializing_if` sites against the
Node serializers before each surface's flip (add to per-surface cutover DoD);
(2) promote APP_BASE_URL to boot-REQUIRED in the Rust config (no prod default) — one-line,
belongs to the next rebuild commit batch.

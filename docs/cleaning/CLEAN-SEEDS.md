# CLEAN-SEEDS.md — defect-class seed catalog (Phase A · HARVEST)

> Source: every error / negative-pattern / risk / invariant / lesson distilled from the project's
> accumulated memory (50 memory files · handoff `v4_5`/`v4_4`/`v3.1` · inventory HTML · regression
> ledger · lessons · audit-gate · reflections · migration blueprint · compliance gate).
> Each row is a **seed** = a detection rule to sweep the whole repo with in Phase B.
> **This phase is read-only. No occurrences swept yet. No fixes.**

## Legend

- **class**: `DESIGN` · `DATAFLOW` (FE↔contract) · `SEC` · `BACKEND` · `RESIL` · `TEST` · `REPO` · `SCHEMA`
- **severity**: `S0` security/privacy/contract/integrity · `S1` resilience/correctness · `S2` UI-state/design-system · `S3` cosmetic/hygiene
- **gate**: `✅<rule>` = already enforced by an authoritative gate (Phase B = confirm zero + check variants the gate misses) · `⚠️<rule>` = gate exists but **warn-level/narrow** so live offenders may remain · `—` = **no gate** (highest cleaning value) · `MV` = manual-verify semantic invariant grep can't see.
- `detect`: exact command/AST entry. `MV` rows produce ledger entries even when grep is silent.

Existing authoritative gates (from `tools/eslint-plugin-local/src/index.js` + `scripts/`): `no-hardcoded-string`, `no-raw-sql`, `no-hardcoded-color`, `no-hardcoded-tailwind-color`, `no-arbitrary-tailwind`, `no-arbitrary-font-size`, `no-raw-form-control`, `no-ts-nocheck`, `no-raw-any`, `no-duplicate-import`, `require-auth-hook`, `no-empty-catch`, `no-process-exit`, `no-permissive-status-assertion`, `no-mock-in-prod`, `no-insecure-random`, `no-direct-websocket`; gates `verify:rls`, `verify:env`, `verify:migrations`, `compliance:gate`, `guardrail:owner-active`, `guardrail:spike-boundary`; 20-row regression ledger.

---

## SEC — security / privacy (mostly S0)

| seed_id | pattern | why_bad | detect | sev | gate | origin |
|---|---|---|---|---|---|---|
| SEED-SEC-COOKIE | `document·cookie` write / `Set‑Cookie` header | cookie regression — must be localStorage; breaks embed (Safari 3p-cookie block) | `grep -rn "document\.cookie\|Set-Cookie" apps packages` | S0 | — | v3.1 §15, ledger#12, audit-gate |
| SEED-SEC-TS-IGNORE | `@ts-ignore` / `@ts-expect-error` | per-line type-off; hides bad casts (sibling of `@ts-nocheck`) | `grep -rn "@ts-ignore\|@ts-expect-error" apps packages` | S1 | — | audit-gate §A |
| SEED-SEC-AS-UNKNOWN | `as unknown as X` (double-cast escape) | bypasses `no-raw-any`; silent type lie | `grep -rn "as unknown as" apps packages` | S1 | — | cleaning (variant of ✅no-raw-any) |
| SEED-SEC-AS-ANY | `as any` | disables type safety | `TSAsExpression→TSAnyKeyword` | S1 | ✅no-raw-any | DoD §5 |
| SEED-SEC-JSON-NO-PARSE | `.json()` without `safeParse`/try | uncaught parse → handler crash, no fallback | `grep -rn "\.json()" apps/api/src apps/web/src \| grep -v safeParse` | S1 | — | phase2-exit (Zod .strict variant) |
| SEED-SEC-INSECURE-RANDOM | non-CSPRNG `Math.random( )` minting token/otp/secret/nonce/session id | predictable → forgeable | AST (security-named id) | S0 | ✅no-insecure-random | ledger#2, ADR-0003 |
| SEED-SEC-DEV-LOGIN | dev-login bypass not NODE_ENV-gated; dev-kid token accepted on prod | live prod owner-JWT backdoor | `grep -rn "signDevToken\|ALLOW_DEV_LOGIN\|isDevRequestAuthorized" apps/api/src` + verify prod gate | S0 | ✅ledger#1 boot-guard | dev-login-backdoor |
| SEED-SEC-JWT-RS256-KID | JWT not RS256 / missing `kid` | symmetric secret in client; key-rotation impossible | `grep -rn "HS256\|HS512\|algorithm" apps/api/src` + verify RS256+kid | S0 | MV | v4_5 §7, DoD §5 |
| SEED-SEC-SECRETS-GIT | secret/DSN/private key anywhere in git history | permanent exposure, can't revoke | `git log --all -S "PRIVATE KEY" -S "SERVICE_ROLE" --oneline` + secret scan all history | S0 | ⚠️verify:secrets | v4_5 §7 (§34) |
| SEED-SEC-SECRETS-BUNDLE | secret/DSN/stack-trace/PII in client bundle or error page | leaks to attacker | grep client build for env keys; check 4xx/5xx pages render no stack | S0 | MV | v4_5 §7 |
| SEED-SEC-HARDCODED-CREDS | test creds (`test@dowiz.com`/`test123456`) + telegram token/secret in e2e files | usable on prod; bot hijack; in git history | `grep -rn "test@dowiz.com\|test123456\|TELEGRAM.*TOKEN" e2e apps` | S0 | — | staging-full-audit |
| SEED-SEC-PII-LLM | customer/courier/owner name/phone/email/address sent to LLM (must be **menu-only**) | raw PII to 3rd party; not a subprocessor | `grep -rn "createMessage\|generateText\|OPENROUTER\|OPENCODE" apps/api/src` + trace payload | S0 | MV | v4_4 §3, v4_5 §7, staging-audit |
| SEED-SEC-PII-QUEUE | pg-boss payload carries PII (name/phone/email/address) not just `order_id` (claim-check) | PII persists in queue table + logs | `grep -rn "pgboss.send\|enqueue\|boss\.send" apps -A6 \| grep -iE "phone\|email\|address\|name"` | S0 | MV | v3.1 §15, p0-privacy |
| SEED-SEC-PII-BUS-WS | WS/MessageBus broadcast carries item names / PII | PII egress to client; minimal claim-check only | `grep -rn "order.created\|fetchOrderDelta\|broadcast" apps/api/src` + trace payload | S0 | MV | p0-privacy-hardening |
| SEED-SEC-CUSTOM-CSS-PURIFY | `custom_css`/uploaded SVG rendered without DOMPurify; lost CSP nonce (SSR) | XSS / style injection | `grep -rn "custom_css\|dangerouslySetInnerHTML\|image/svg" apps packages \| grep -v DOMPurify` | S0 | — | v3.1 §12 |
| SEED-SEC-SSRF-BRAND | brand/website extractor fetches user URL without per-redirect re-validation | SSRF / DNS-rebind TOCTOU | `grep -rn "assertPublicUrl\|brand-extractor\|fetch.*website" apps/api/src` + check redirect revalidation | S0 | MV | dev-login-backdoor batch |
| SEED-SEC-IDOR-COURIER | `acceptCourierAssignment` scoped by `order_id` only, not caller `courier_id` | cross-courier assignment hijack | `grep -rn "acceptCourier\|courier.*accept" apps/api/src` + verify `WHERE courier_id=caller` | S0 | MV | dev-login-backdoor batch |
| SEED-SEC-ANON-ORDER-IDOR | `GET /orders/:id` guarded by UUID only, no tenant/auth scope | order enumeration / IDOR | `grep -rn "orders/:id\|/orders/" apps/api/src/routes` + verify tenant scope | S0 | MV | v1-verification P2 |
| SEED-SEC-TOKEN-IN-URL | JWT/token in URL query (`?token=`) | leaks via history/logs/referer | `grep -rn "token=\|jwt=\|auth_token=" apps \| grep -v Bearer` | S0 | — | non-pixel-sweep |
| SEED-SEC-UPLOAD-MAGIC | file upload without magic-byte/signature validation | code injection disguised as image | `grep -rn "upload\|putObject\|presign" apps/api/src -A8 \| grep -v "file-type\|magic\|signature"` | S1 | MV | v3.1 §12 |
| SEED-SEC-TOKEN-TTL-ACTIVE | owner token long TTL + no per-request `status='active'` re-derive on JWT-baked `activeLocationId` | revoked/insider owner keeps reading ≤24h | `grep -rn "activeLocationId" apps/api/src` + verify each authz path does live `AND status='active'` | S0 | ✅ledger#16 (partial) | ADR-0004 |
| SEED-SEC-OAUTH-GATE | `/auth/google` + `/callback` live though FE-hidden; leaks client_id | backend bypass of FE gate | `grep -rn "auth/google\|/callback" apps/api/src/routes` + verify `GOOGLE_OAUTH_ENABLED` gate | S0 | MV | staging-audit-fixes |
| SEED-SEC-ANONYMIZER | GDPR purge deletes rows instead of anonymizing PII fields | breaks audit trail / FK integrity | `grep -rn "anonymiz\|gdpr\|purge" apps/api/src` + verify nulls PII not DELETE | S0 | MV | v4_5 §7 |
| SEED-SEC-IMPORT-META-RUNTIME | `import.meta.env.DEV` used as a **runtime** guard for dev endpoints | statically false in prod build → guard ineffective / wrong | `grep -rn "import.meta.env.DEV" apps` (flag runtime-guard uses) | S1 | — | golive-remediation |
| SEED-SEC-RATELIMIT-MUTATION | mutation route without rate-limit; per-instance limit assumed global on N>1 | abuse; limit ×N bypass | `grep -rn "@fastify/rate-limit\|rateLimit" apps/api/src` + verify mutations covered + upstream WAF | S0 | MV | DoD §5, v3.1 §8 |

---

## BACKEND — server correctness

| seed_id | pattern | why_bad | detect | sev | gate | origin |
|---|---|---|---|---|---|---|
| SEED-BE-RAW-SQL | SQL keyword + interpolation/concat | SQLi | template-with-expr / `"…"+` | S0 | ✅no-raw-sql | DoD §5 |
| SEED-BE-ZOD-STRICT | `z.object(` body/params without `.strict()` | silent extra-prop acceptance | `grep -rn "z.object(" apps/api/src \| grep -v strict` (triage body/params) | S1 | — | phase2-exit §1 |
| SEED-BE-RLS-FORCE | tenant table without `ENABLE`+`FORCE` RLS / handler without `SET LOCAL` | tenant isolation bypass | `pnpm verify:rls` + `grep -rn "SET LOCAL app\." apps/api/src` | S0 | ✅verify:rls | v3.1 §15, blueprint I2 |
| SEED-BE-PRIVPOOL-WHERE | privileged/analytics pool query without `WHERE location_id` | cross-tenant leak / noisy-neighbor | `grep -rn "operationalPool\|adminPool\|privilegedPool" apps/api/src -A6 \| grep -v location_id` | S1 | MV | v4_5 §7 (§34) |
| SEED-BE-STATUS-GUARD | status `UPDATE` without `WHERE … AND status=$expected` / bypass `assertTransition` | double-confirm race; illegal transition | `grep -rn "UPDATE orders SET status" apps/api/src` + verify guard/assertTransition | S1 | MV | DoD §5 |
| SEED-BE-IDEMPOTENCY-TX | idempotency in-memory, or enqueue not in same tx as insert (outbox) | dup orders on retry/crash; outbox drift | `grep -rn "Idempotency\|request_hash\|outbox" apps/api/src` + verify unique-constraint + tx | S0 | MV | v3.1 §8, DoD §5 |
| SEED-BE-MONEY-INT | money as float — `parseFloat`/`toFixed(2)`/`*0.01`/`/100` | rounding drift; wrong charge | `grep -rEn "parseFloat\|toFixed\(2\)\|\* *0\.01\|/ *100\b" apps packages` (triage money columns) | S0 | ⚠️ledger#7 | systemic-coherence, money-fix |
| SEED-BE-NOTIFY-CRITPATH | `await notify()` on critical path / failure rolls back order | notify must be off critical-path | `grep -rn "notify\|sendMessage\|telegram" apps/api/src -A8 \| grep -E "await\|throw" \| grep -v catch` | S0 | MV | v4_5 §7 |
| SEED-BE-EXTERNAL-TIMEOUT | `fetch`/`axios`/external call without timeout+fallback | event-loop block; cascade | `grep -rn "fetch(\|axios\." apps/api/src \| grep -v "timeout\|AbortController"` | S1 | MV | v4_5 §7 |
| SEED-BE-DELIVER-FRICTION | `deliver` auto-closed by signal correlation without courier override | red-line: correlation→friction, not verdict | `grep -rn "deliver\|DELIVERED\|auto.*complete" apps/api/src` + verify courier can override | S0 | MV | v4_5 §7, v3.1 §11 |
| SEED-BE-GPS-FILTER | GPS ingest without sanity filter (accuracy>100m, speed>150) | spoof / garbage trusted | `grep -rn "accuracy\|coords\|position" apps/api/src` + verify reject thresholds | S1 | MV | v3.1 §11 |
| SEED-BE-CASH-HOLD | cash HOLD not atomic / divergence not alerted to owner | red-line: divergence→friction-alert | `grep -rn "cash\|HOLD\|reconcil" apps/api/src` + verify atomic + alert | S1 | MV | v4_5 §7 |
| SEED-BE-AUTOBAN | anti-fraud auto-bans actor without human review / irreversible | red-line: zero auto-ban | `grep -rn "ban\|block\|fraud" apps/api/src` + verify human-in-loop + reversible | S0 | MV | v4_5 §7 |
| SEED-BE-CREATE-REPLACE-STALE | `CREATE OR REPLACE FUNCTION/VIEW` copied from an OLD migration body | silently reverts later localizations/fixes | `grep -rn "CREATE OR REPLACE" packages/db/migrations` + verify copied from latest | S1 | MV | lesson read-public-menu-redefine |
| SEED-BE-HEALTH-SPLIT | `/health` returns 503 for non-critical (Redis) dep | non-critical kills LB routing; masks DB | `grep -rn "/health\|/livez\|/readyz" apps/api/src -A20` + verify degraded vs fatal split | S1 | MV | v3.1 §15 |
| SEED-BE-SIGTERM-DRAIN | no `SIGTERM` handler / doesn't drain jobs+WS | job loss + reconnect storm on deploy | `grep -rn "SIGTERM\|SIGINT" apps/api/src apps/worker/src` | S1 | MV | v3.1 §15 |
| SEED-BE-AUTOCANCEL-PERSIST | auto-cancel via in-memory timer (no `timeout_at` col + poller) | crash loses timeout; PENDING hangs | `grep -rn "timeout_at\|auto.*cancel\|setTimeout" apps/api/src` | S0 | MV | v3.1 §8/§12 |
| SEED-BE-PROCESS-EXIT | `process.exit()` in library code | abrupt; throw instead | AST | S2 | ✅no-process-exit | hygiene |
| SEED-BE-EMPTY-CATCH | empty `catch {}` | swallows error | AST | S1 | ✅no-empty-catch | hygiene |
| SEED-BE-UNHANDLED-PROMISE | `.then(` without `.catch`/await | silent failure | `grep -rn "\.then(" apps/api/src packages \| grep -v "catch\|await"` | S1 | — | audit-gate §C |
| SEED-BE-ENV-NULLCHECK | `process.env.X` used without presence guard | undefined → boot TypeError | `pnpm verify:env` + `grep -rn "process.env\." apps/api/src` | S1 | ⚠️verify:env | v1-verification |
| SEED-BE-OTP-GATE | OTP routes/SMS not gated by `OTP_ENABLED`; raw 400 to user | dead/unfriendly flow | `grep -rn "OTP_ENABLED\|/otp" apps/api/src` | S1 | MV | otp-disabled-money-fix |
| SEED-BE-SOFT-CONFIRM | velocity soft-confirm returns bare 200 with silent rollback (no order) | UI shows success, DB empty | `grep -rn "soft_confirm\|velocity" apps/api/src apps/web/src` + verify surfaced | S1 | MV | sold-out-chip-dead |
| SEED-BE-CONTRACT-FIELD-PARITY | client field name ≠ server Zod field (e.g. `delivery.notes` vs `delivery_instructions`) | silent 400 checkout break | cross-grep client payload keys vs route schemas | S1 | MV | staging-audit-fixes |

---

## DATAFLOW — frontend ↔ contract

| seed_id | pattern | why_bad | detect | sev | gate | origin |
|---|---|---|---|---|---|---|
| SEED-DF-DIRECT-FETCH | `fetch(`/`axios`/`XMLHttpRequest`/`$.ajax` bypassing `apiClient` | loses auth header/dedup/error-normalize | `grep -rEn "fetch\(\|axios\|XMLHttpRequest\|\$\.ajax" apps/web/src \| grep -v apiClient` | S1 | — | v1-verification |
| SEED-DF-DIRECT-WS | `new WebSocket()` outside shared `useWebSocket` | bypasses reconnect/frame-order | AST (frontend) | S1 | ✅no-direct-websocket | ledger#5/#7 |
| SEED-DF-CLIENT-PRICE | client recomputes `total`/price/status (must be server-sourced) | drift vs server charge | `grep -rEn "subtotal\|total\s*=\|reduce.*price\|\* *qty" apps/web/src` (triage) | S1 | MV | DATAFLOW invariant |
| SEED-DF-WS-RECONCILE | reconnect trusts WS event order without `GET` reconcile | stale/wrong status after reconnect | `grep -rn "onreconnect\|reconnect\|onopen" apps/web/src` + verify refetch | S1 | MV | seam-polish |
| SEED-DF-WS-TERMINAL-LOCK | WS status handler can revert terminal state (DELIVERED→IN_DELIVERY) | order goes backwards | `grep -rn "order.status\|setStatus" apps/web/src` + verify terminal guard | S1 | ✅ledger (runtime monotonic) | seam-polish |
| SEED-DF-WS-UNSUB | subscribe without unsubscribe on cleanup/close | handler pile-up → N× duplicate fan-out | `grep -rn "subscribe\|addEventListener" apps -A4 \| grep -v "unsubscribe\|removeEventListener\|return ()"` | S1 | MV | v1-verification P1-WSDUP |
| SEED-DF-NODE-CRON | `node-cron` (in-process scheduler) | double-fires on N>1; dead-scheduler invisible | `grep -rn "node-cron\|cron.schedule" apps packages` | S1 | — | v3.1 §8 |
| SEED-DF-LOCAL-WS-MAP | WS clients in local Map/Set, broadcast local-only | clients on other instances miss updates | `grep -rn "new Map()\|new Set()" apps/api/src` (triage WS clients) + verify MessageBus | S1 | MV | v3.1 §8 |
| SEED-DF-COURIER-STATUS | courier "Online" badge shows account-active, not shift state | misroutes assignments; dishonest | `grep -rn "Online\|account_active\|shift" apps/web/src` + verify shift-derived | S1 | ✅ledger (honesty fix) | fee-courier-seed |
| SEED-DF-IMPORT-SILENT-ZERO | menu import returns 0 products silently on LLM failure | user thinks import worked | `grep -rn "parse.*menu\|ocr\|import" apps/api/src` + verify error surfaced | S1 | MV | owner-fixes-batch |
| SEED-DF-MENU-LOCALE | menu fetch without `?locale=` → backend defaults EN | SQ/UK user sees English modifiers | `grep -rn "/menu\|locations/.*menu" apps/web/src` + verify locale param | S1 | MV | v1-hardening |

---

## DESIGN — design-system & UI hygiene

| seed_id | pattern | why_bad | detect | sev | gate | origin |
|---|---|---|---|---|---|---|
| SEED-DS-COLOR-HEX | hardcoded `#hex` outside `tokens.css` | breaks per-tenant branding | ✅rule + `grep -rEn "#[0-9a-fA-F]{3,8}"` | S2 | ✅no-hardcoded-color | ledger#6 |
| SEED-DS-COLOR-NONHEX | `rgb(`/`rgba(`/`hsl(`/named color in JSX/CSS | same as above, **gate only catches hex** | `grep -rEn "rgb\(\|rgba\(\|hsl\(" packages/ui apps/web/src \| grep -v tokens.css` | S2 | ⚠️(hex-only) | v3.1 §15 |
| SEED-DS-TW-COLOR | hardcoded tailwind color class | bypasses tokens | ✅rule | S2 | ✅no-hardcoded-tailwind-color | ledger#6 |
| SEED-DS-ARBITRARY-TW | `p-[13px]`/`text-[#fff]` arbitrary bracket | off-scale/off-token | ✅rule | S2 | ✅no-arbitrary-tailwind/-font-size | ui-loop |
| SEED-DS-FONT | `font-family:` not `var(--brand-font*)` | brand FOUC/drift | `grep -rEn "font-family\s*:" packages/ui apps/web/src \| grep -v "var(--brand-font"` | S2 | — | audit-gate |
| SEED-DS-RAW-FORM | native `<select>`/`<textarea>` in apps/web | inconsistent control | ✅rule | S2 | ✅no-raw-form-control | consolidation |
| SEED-DS-HARDCODED-STRING | user-visible string not via `t('key','fallback')` | untranslated; diacritics lost | ✅rule (warn) | S1 | ⚠️no-hardcoded-string | ledger#8 |
| SEED-DS-DARK-ON-DARK | partial tenant theme + default-dark tokens | dark-on-dark unreadable cards | `grep -rn "brand-surface\|brand-border" packages/ui/src/theme` + verify derivePalette fills all | S1 | ✅ledger#6 (derivePalette) | client-theme-rootcause |
| SEED-DS-PRIMARY-AS-TEXT | `--brand-primary` used as text color (AA fail ~3.7:1) | unreadable on light tenant | `grep -rEn "color.*brand-primary\|text-.*brand-primary" apps/web packages/ui` + verify `-readable` | S1 | ⚠️ | systemic-coherence |
| SEED-DS-OPACITY-MUTED | `opacity-{40,70}` on already-tuned `text-muted` | drops below AA | `grep -rn "text-muted.*opacity\|opacity.*muted" packages/ui apps/web` | S2 | — | non-pixel-sweep |
| SEED-DS-NESTED-INTERACTIVE | clickable card wrapping buttons | a11y nested-interactive; kbd nav broken | MV (axe in E2E) + `grep -rn "onClick" apps/web \| grep -i card` | S2 | MV | non-pixel-sweep, systemic-coherence |
| SEED-DS-ICON-UNLABELED | icon-only button without `aria-label` | SR says "button" | `grep -rn "<button" apps/web packages/ui` (triage icon-only) | S2 | MV | systemic-coherence |
| SEED-DS-INPUT-UNLABELED | `<select>`/`file`/`range` without label/aria-label | SR no field purpose | `grep -rEn "<select\|type=['\"]file\|type=['\"]range" apps/web packages/ui` | S2 | MV | systemic-coherence |
| SEED-DS-ORPHAN-ARIA | `role="tab"` etc. without required parent / `aria-current` | orphan role; WCAG fail | `grep -rn "role=['\"]tab" apps/web` | S2 | MV | systemic-coherence |
| SEED-DS-DEAD-BUTTON | `onClick` noop / `{/* TODO */}` / disabled-no-reason | looks interactive, does nothing | `grep -rn "onClick" apps/web packages/ui` (triage noop) | S2 | MV | audit-gate §B |
| SEED-DS-DEAD-STATE | data component missing loading/empty/error | mid-screen void on slow/empty/fail | MV (per data-driven component) | S2 | MV | design-review, audit-gate §D |
| SEED-DS-FALSE-INVALID | `:invalid` red-ring on pristine required field | looks broken before input | `grep -rn ":invalid" apps/web` + verify reset present | S2 | ✅(fixed; re-add check) | fe-polish-batch |
| SEED-DS-BROKEN-IMAGE | product image 404 with no fallback glyph | blank box | `grep -rn "<img\|background-image\|Image" apps/web` + verify onError fallback | S2 | MV | seam-polish |
| SEED-DS-MOBILE-OVERFLOW | modal CTA (`flex-1`+`nowrap`) overflows ≤390px | price clipped / button unclickable | Playwright @390px on product modal | S2 | MV | staging-audit-fixes |
| SEED-DS-TAP-TARGET | courier critical action <48/56px | misclick while driving (safety) | MV (measure courier screen targets) | S2 | MV | v3.1 §15 |
| SEED-DS-INVENTORY-DRIFT | page/component in inventory but missing in code (or extra screen) | blind spot / scope drift | MV (cross-check inventory list ↔ `apps/web/src/pages`) | S1 | MV | inventory HTML |

---

## RESIL — resilience / ops

| seed_id | pattern | why_bad | detect | sev | gate | origin |
|---|---|---|---|---|---|---|
| SEED-RS-SCHEMA-DRIFT | deploy without running migrations / boot needs newer head | prod crash-loop (DOWN) | verify `fly.toml` `release_command` + CI migrate step | S0 | ✅ledger#3 | prod-outage-schema-drift |
| SEED-RS-MIGRATION-HEAD | code `__EXPECTED_MIGRATION_HEAD__` ≠ migrations tail | boot-guard FATAL at startup | compare `scripts/build-apps.ts` head vs `ls packages/db/migrations \| tail -1` | S0 | ⚠️ | migration blueprint §8.1 |
| SEED-RS-POOL-STARVATION | pool max too small vs queries-per-request (e.g. /public/menu) | conn-timeout → HTTP 500 → empty menu | count `db.query` per hot route vs pool max | S1 | ✅(fixed) | public-menu-pool-starvation |
| SEED-RS-SSR-CACHE | public `/s/:slug` SSR lacks `Cache-Control`+`menu_version` | hammers Postgres; DoS vector | `grep -rn "Cache-Control\|menu_version" apps/api/src` | S1 | MV | v3.1 §13, v4_4 §5 |
| SEED-RS-WORKER-LIVENESS | worker has no liveness / false-green possible | dead worker undetected; PENDING stalls | `grep -rn "liveness\|heartbeat\|/health" apps/worker/src` | S0 | MV | v4_5 §7 |
| SEED-RS-BACKUP-RESTORE | backup not validated by restore-test | corrupt backup found only in incident | `grep -rn "restore" scripts apps` + verify restore assertion | S0 | ⚠️backup:restore | v4_5 §7 (§32) |
| SEED-RS-STORAGE-BACKUP | object storage not synced to R2 versioned | deleted images lost; image_url breaks | check backup-worker for storage→R2 versioned sync | S0 | MV | v4_5 §7, v3.1 §5 |
| SEED-RS-PIPE-EXIT | `cmd \| tail/head/grep` masks exit code | false-green deploy/commit | `grep -rn "\| *tail\|\| *head" scripts .github` (CI/deploy) | S0 | — | deploy-validation-traps |
| SEED-RS-FREE-TIER-ALERT | no 80% threshold alert on Free-tier limits | silent dark at 100% | `grep -rn "free-tier\|80\|threshold" scripts` + verify alerting | S1 | ⚠️free-tier:watch | v4_5 §8 |

---

## TEST — test hygiene

| seed_id | pattern | why_bad | detect | sev | gate | origin |
|---|---|---|---|---|---|---|
| SEED-TS-SKIP-ONLY | `test.skip`/`.only`/`.fixme`/`describe.only` | disables/quarantines coverage | `grep -rEn "(test\|it\|describe)\.(skip\|only\|fixme)" e2e apps/api/tests` | S1 | — | ledger#11 |
| SEED-TS-FAKE-ASSERT | `expect(true)` / commented-out assertion | fake-green | `grep -rEn "expect\(true\)\|expect\(false\)" e2e apps/api/tests` + grep `// expect` | S1 | — | ledger#11 |
| SEED-TS-WAIT-TIMEOUT | `waitForTimeout`/`sleep`/inflated `retries` | flaky-masking; balloon duration | `grep -rn "waitForTimeout\|sleep(" e2e apps/api/tests` + check `playwright.config` retries | S1 | — | ledger#11 |
| SEED-TS-PERMISSIVE-STATUS | `expect([200,400]).toContain(x)` | hides wrong status | ✅rule | S1 | ✅no-permissive-status-assertion | ledger |
| SEED-TS-MOCK-IN-PROD | mock/fake/stub var in prod path | test data in prod | ✅rule | S1 | ✅no-mock-in-prod | hygiene |
| SEED-TS-STALE-SPEC | spec selectors point at removed markup (e.g. flow-start-hero) | spec can't verify | run specs; grep dead `data-testid`/class selectors | S2 | MV | landing-polish-loop2 |
| SEED-TS-I18N-PARITY | parity gate warn-only; missing sq/uk keys ship | English leaks to SQ/UK | `tsx scripts/i18n-parity.ts --strict` | S1 | ⚠️i18n parity | ui-loop, fe-polish |
| SEED-TS-ASSERT-VS-SPEC | assertion mirrors code, not inventory/contract | tautological test | MV (review key specs vs contract) | S1 | MV | convergence X10 |

---

## REPO — repo / build hygiene

| seed_id | pattern | why_bad | detect | sev | gate | origin |
|---|---|---|---|---|---|---|
| SEED-RP-ROOT-CLUTTER | committed `*.png`/`*.log`/`fix.*`/`analyze.mjs` at repo root | bloats clones; misleads | `find . -maxdepth 1 -type f \( -name "*.png" -o -name "*.log" -o -name "fix.*" \)` | S2 | — | blueprint §19 |
| SEED-RP-TRANSFER-ARTIFACTS | `/root/restore`, `*.tar.gz`, `*.zip` left around | leftover transfer junk | `find . -name "*.tar.gz" -o -name "*.zip"; ls -d /root/restore 2>/dev/null` | S3 | — | migration handoff |
| SEED-RP-WINDOWS-JUNK | `settings.local.json` Windows paths / PowerShell / `C:\` | cross-platform junk | `grep -nE "C:\\\\\|/c/Users\|PowerShell" .claude/settings.local.json` | S2 | — | harness-tuneup |
| SEED-RP-HOOK-TOPLEVEL | `git rev-parse --show-toplevel` in hooks w/o fallback | fails in worktree/non-git/CI | `grep -ln "git rev-parse --show-toplevel" .claude/hooks/*` + verify fallback | S2 | — | migration handoff |
| SEED-RP-SPIKES-IMPORT | import from `spikes/` in prod code | quarantine leaked to build | `grep -rn "spikes/" apps packages` | S1 | ✅guardrail:spike-boundary | agent-operating-model |
| SEED-RP-ENV-DRIFT | `.env.example` lists removed `WHATSAPP_*`/`BAILEYS`; missing new keys | misleads onboarders | `grep -c "^WHATSAPP\|BAILEYS" .env.example` (expect 0); diff vs config EnvSchema | S2 | — | migration blueprint §19 |
| SEED-RP-GITIGNORE-ENV | `.env.*` not ignored (except `!.env.example`) | secret leak risk | `grep -n "env" .gitignore` + verify | S0 | MV | DoD §5 |
| SEED-RP-DUP-IMPORT | same module imported twice | hygiene | ✅rule | S3 | ✅no-duplicate-import | hygiene |

---

## SCHEMA — migration / schema-fold debt

| seed_id | pattern | why_bad | detect | sev | gate | origin |
|---|---|---|---|---|---|---|
| SEED-SC-MIGRATION-SEQ | numbering conflict/gaps (016-017 / 024-026 / 027 / 030 vs timestamp scheme) | unclear head; order-check breakage | `ls packages/db/migrations` + `pnpm verify:migrations` | S2 | ⚠️verify:migrations | v4_5 §8 |
| SEED-SC-PREFLIGHT-JSONB | `orders.preflight jsonb` added out-of-band (Stage 27) | schema-fold debt vs foundation | `grep -rn "preflight" packages/db/migrations` | S1 | MV | v4_5 §8 |
| SEED-SC-SUBSCRIPTION-FIELDS | `subject_type`/`opted_in` added out-of-band (Stage 28) | schema-fold debt | `grep -rn "subject_type\|opted_in" packages/db/migrations` | S1 | MV | v4_5 §8 |
| SEED-SC-NOTNULL-NO-DEFAULT | RLS-retrofit NOT NULL col without DEFAULT breaks inserts/seed (prep_time, request_hash, modifier location_id) | 500 on insert; seed fails | grep migrations for `NOT NULL` adds + verify DEFAULT or all inserts provide value | S1 | MV | preptime, visual-net |
| SEED-SC-SEARCH-PATH | `SECURITY DEFINER` function without `SET search_path=public` | schema-resolution hijack | `grep -rn "SECURITY DEFINER" packages/db/migrations \| ...` + verify search_path set | S0 | MV | staging-full-audit |
| SEED-SC-PGBOSS-BOOTSTRAP | queue-name added after migration 0011 without new bootstrap migration | fresh-provision deadlock | diff `shared-types/queue-names` vs pgboss bootstrap migration | S1 | ⚠️verify:fresh-provision | blueprint §8.1 |
| SEED-SC-COMPLIANCE-PII | new PII col / subprocessor / raw-PII sink / missing DPIA | undocumented PII; legal | `pnpm compliance:gate` (§A/§B/§C/DPIA) | S0 | ✅compliance:gate | compliance-repo |
| SEED-SC-PG-VERSION | Postgres version / pgboss grants not verified vs expected (bootstrap) | feature/behavior drift; job-create perm denied | MV (`SELECT version()`, pgboss grants) | S2 | MV | v3.1 §14 |

---

## Inventory baseline (for SEED-DS-INVENTORY-DRIFT cross-check in Phase B)

Top-level surfaces the inventory (`docs/deliveryos_v2_pages_components.html`) declares MUST exist:

- **Client `/s/:slug`**: Menu (SSR), Cart, Checkout, Order Status, Menu embed (`?embed=true`)
- **Owner `/admin/*`**: Dashboard, Orders (Kanban), Menu, Couriers, Analytics, CRM, Settings, Branding, AI assistant, Promotions
- **Courier `/courier/*`**: Tasks, Live Delivery (`/delivery/:id`)
- **Shared**: ~14 UI atoms (ThemeProvider, BrandLogo, StatusBadge…), ~11 hooks (useWebSocket, useGeolocation, useCart…), ~8 utils (normalizePhone, checkDeliveryZone, calcETA…)

Phase-B drift check = this list vs `git ls-files apps/web/src/pages` (missing = blind spot; extra = scope drift → flag, not fix).

---

## Manual-verify seeds (produce ledger rows even when grep is silent)

Semantic invariants that grep can't fully see — must be confirmed by reading code + a test:
`SEED-SEC-JWT-RS256-KID`, `SEED-SEC-SECRETS-BUNDLE`, `SEED-SEC-PII-*`, `SEED-SEC-CUSTOM-CSS-PURIFY`, `SEED-SEC-SSRF-BRAND`, `SEED-SEC-IDOR-COURIER`, `SEED-SEC-ANON-ORDER-IDOR`, `SEED-SEC-UPLOAD-MAGIC`, `SEED-SEC-OAUTH-GATE`, `SEED-SEC-ANONYMIZER`, `SEED-SEC-RATELIMIT-MUTATION`, `SEED-BE-PRIVPOOL-WHERE`, `SEED-BE-STATUS-GUARD`, `SEED-BE-IDEMPOTENCY-TX`, `SEED-BE-NOTIFY-CRITPATH`, `SEED-BE-DELIVER-FRICTION`, `SEED-BE-GPS-FILTER`, `SEED-BE-CASH-HOLD`, `SEED-BE-AUTOBAN`, `SEED-BE-CREATE-REPLACE-STALE`, `SEED-BE-HEALTH-SPLIT`, `SEED-BE-AUTOCANCEL-PERSIST`, `SEED-DF-CLIENT-PRICE`, `SEED-DF-WS-RECONCILE`, `SEED-DF-WS-UNSUB`, `SEED-DF-LOCAL-WS-MAP`, `SEED-DS-DEAD-STATE`, `SEED-DS-TAP-TARGET`, `SEED-DS-INVENTORY-DRIFT`, `SEED-RS-SSR-CACHE`, `SEED-RS-WORKER-LIVENESS`, `SEED-RS-STORAGE-BACKUP`, `SEED-TS-ASSERT-VS-SPEC`, `SEED-SC-*` (preflight, subscription, not-null, search-path, pgboss, pg-version), `SEED-RP-GITIGNORE-ENV`.

---

## Phase-A exit status

- Sources covered: 50 memory files · `v4_5`/`v4_4`/`v3.1` · inventory HTML · regression ledger (20 rows) · lessons (4) · audit-gate · reflections INBOX · migration blueprint · compliance gate. ✅
- Seeds: **~95** deduped, every one with a `detect` method (concrete or `MV`). ✅
- Classes align with the prompt catalog (DESIGN/DATAFLOW/SEC/BACKEND/RESIL/TEST/REPO/SCHEMA). ✅
- **Zero fixes, zero repo sweep performed.** ✅

> **STOP-checkpoint A.** Awaiting GO before Phase B (read-only sweep → `CLEAN-LEDGER.md`).

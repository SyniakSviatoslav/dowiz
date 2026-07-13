# Integration Ports + Reactive Interface — RESEARCH CONSPECT

> Дата: 2026-07-13 · 6 паралельних дослідницьких звітів (A1..A6), зведені для
> [INTEGRATION-PORTS-PLAN.md](./INTEGRATION-PORTS-PLAN.md) + [BLUEPRINTS](./BLUEPRINTS-INTEGRATION-PORTS.md).
> Governing law (оператор дослівно): «Кор незмінний — будь-які інтеграції це порти до моєї системи і
> протоколу, ніколи із прямим втручанням.»

## A2 — Ports-architecture (BACKBONE)
- Гексагональна ports-and-adapters. Порт = adapter на межі: READ-side (out) підписується на scoped
  проєкцію event-log; WRITE-side (in) подає signed intent у ворота (той самий `POST /api/orders`/
  `apply_event`, 409 на illegal). Кор вирішує, порт ніколи.
- 3 композовані ворота ВЖЕ в коді: wire-gate (`HybridGate::RequireBoth`), Law-gate
  (`order_machine::assert_transition`), money-gate (i64 by type, `money.rs`).
- Adapter НІКОЛИ не імпортує `dowiz-kernel` = компіляційний firewall. Capability-not-bearer;
  consent = sign delegation; RevocationSet buildable (UCAN-style, attenuation-only, offline verify).
- Port taxonomy: embed/deep-link/REST/webhook/MCP/headless/realtime/backup/analytics/matcher. 3 tiers
  (Ready/Customizable/Dev-agent). RED core-untouched test. Human directory + scope→plain-language renderer.
  Build order additive.

## A3 — MCP / agentic / RAG
- Кожна інтеграція = capability-scoped port. MCP 2026: spec finalized 2026-07-28, stateless core, OAuth
  2.1 resource-server, tools/resources/prompts, stdio+Streamable-HTTP.
- Розширити `scope.rs` Resource: Menu 0x05 / Order 0x06 / Analytics 0x07 / Customer 0x08 / Corpus 0x09.
  Tools→scopes. Agentic-RAG hybrid pgvector+BM25+rerank. A2A/AP2 agent-payments (але cash→no payment tool).
  Agent-as-capability mesh (sub-agent attenuation-only). 6 структурних лімітів.

## A1 — Reactive interface
- Operator-as-API-contract: 8 params (S/Γ/M/c²/g/φ/A_max/Q). Кожен сигнал → зміна одного параметра
  `M·Ü+Γ·U̇+c²·L·U=S`. Нуль if-джерело розгалужень.
- Device signals з чесною 2026-доступністю: Compute Pressure (thermal, Chromium-only reduce-work);
  Battery/Network (Chromium enhancement); Page Visibility + frame-time + prefers-* (універсально, основа).
- QualityGovernor граційна деградація: Q3 WebGPU compute → Q2 fragment shader → Q1 CSS springs+View
  Transitions → Q0 static. Intent ≤100ms. Multimodal = S₁+S₂ суперпозиція. 7 cool tricks (OKLCH,
  linear() springs, View Transitions, WebGPU metaball/reaction-diffusion, potential-well, Green's ripples,
  spectral drill-down) + reject pile (Ambient Light Sensor, Idle Detection API — dead/creepy).

## A5 — ANU QRNG / quantum / PQ-mesh
- PQ-mesh final vision: суверенні локальні ноди, hybrid Ed25519⊕ML-DSA-65 identity + ML-KEM-768→
  XChaCha20-Poly1305, Argon2id, fail-closed ChaCha20 DRBG (`rng.rs:475`, `compile_error!` `:465`),
  AnchorRoster genesis-frozen UCAN-subset, matcher.rs pure deterministic (no central dispatcher).
- Entropy Port = **mix NEVER replace**: OS getrandom = mandatory fail-closed floor, QRNG advisory-only.
  Proposed EntropySource trait + SeedPool що MIXES через SHA3-512 (`hash.rs:352`). QRNG на seed+reseed
  time ONLY, ніколи per-byte/frame. Monotone (adding source never subtracts) / fail-closed (only OS floor
  fails seed) / visible (UI enumerates) / is_local() network-vs-device.
- ANU QRNG 2026: `api.quantumnumbers.anu.edu.au` (`x-api-key`, `?length=1-1024&type=uint8/uint16/hex16`),
  free key at `quantumnumbers.anu.edu.au`, legacy `qrng.anu.edu.au` retiring. 3-step onboarding: get key →
  live test → status chip «Entropy: OS ✓ + ANU quantum ✓», fail-closed to OS if beacon down.
- Quantum-noise-as-encryption HONEST: Y-00/AlphaEta/QKD потребують ОПТИЧНОГО заліза (shot-noise, coherent
  transceivers), НЕ софт. Software offering = «quantum-SEEDED» не «quantum-encrypted». Defense-in-depth на
  FIPS PQ, ніколи не заміняє. Marketing rule: «keys generated with real quantum randomness on top of
  post-quantum-secure crypto» — правдиво/перевірювано.

## A6 — Notifications / backups / made-for-humans onboarding
- Integration = capability-scoped PORT, кор недоторканий. 3 наявні assets reuse: Web-Push/VAPID
  (`web/src/lib/push.js`+`sw.js`), messenger deep-links (`apps/web/src/lib/messenger.ts`), ChannelLedger
  (`channel.js`↔`analytics.rs`), server/ axum routes, PQ primitives.
- Notification port: kernel emits order.* → tail (holds EmitNotification) → routing-table → OUTBOX (local
  durable) → async fan-out → ChannelAdapter (WebPush/Telegram/WhatsApp/SMS/Email/Webhook PQ+HMAC). Retry
  exp-backoff+jitter → DLQ; dedupe idem_key; offline drains on reconnect; reuse `/api/healthz` ratchet.
- Backup port: event-log = source of truth, fold deterministic → backup = event-log, restore = replay →
  bit-identical. XChaCha20-Poly1305 encrypt (venue's own key, dowiz never holds). Sinks: Download/S3-R2/
  Google-Drive (ciphertext only) / their-Postgres-pgrust. Automerge CRDT mesh (ships no encryption → we own
  it), PQ sync frames. Restore tamper → Poly1305 fails → refuse.
- Onboarding: directory by SPHERE. Card = plain What/Why + capability-in-plain-words + 3-step + tier badge.
  Consent mints scoped SignedFrame (ML-DSA-65, offline AnchorRoster verify). Connected-list + one-click
  Revoke. Made-for-humans doc template (Diátaxis tutorial+how-to, outcome-first, «For developers» collapsed,
  review rule: no card leads with config/API-key/acronym). Zapier/n8n/Make = webhook-port universal (n8n
  zero-build today). RED R1-R6 each reachable red→green.

## A4 — Platform bridges per-sphere
- Port doctrine: capability-scoped, kernel+event-log sovereign, degrade-closed, revocable. Scope grammar
  maps bebop PQ identity (ML-DSA-65⊕Ed25519). Publishable pk_ (client HTML, low-risk, origin-allowlist+
  rate-limit) vs secret sk_ (server-side). 3 tiers.
- MARKETING: Google Business Profile (2026 pure redirect, order на dowiz, zero code); Meta CAPI/Google Data
  Manager/TikTok Events (server-side mandatory); `?ch=` first-party spine (already `Storefront.svelte:93`).
  Cash+no-webhook → attribution shape = capture gclid/fbclid/ttclid+hashed-phone → on order.completed
  server-side upload. Time-sensitive: Google→Data-Manager 15Jun2026, 63-day; Meta retired 7d/28d Jan2026;
  SHA-256 hashed EMQ≥7.
- SALES: cash no-Stripe. Loyalty build-in-house (own ledger) + wallet-pass (only port). Consented
  re-engagement (opt-in+unsubscribe red-line). Reviews nudge (+0.1★≈+1% covers). **Higgsfield** = manual
  AI-video tool, NOT port. **Apollo** = B2B lead-gen, category error, internal GTM only, never in UI.
- ANALYTICS: venue owns data, read-only tenant-scoped egress. Google Sheets/CSV (80% case), Postgres/
  Supabase sink (replica), pgrust local-first (source of truth), webhook/MCP (dev/agent). Read-only enforced
  by cap-class.
- SOCIAL/MESSENGERS: Telegram primary ($0, full push+OTP). WhatsApp App/Cloud API (utility templates,
  per-message 1Jul2025). Instagram push IMPOSSIBLE (deprecated 27Apr2026) → discovery+catalog+in-window.
  Viber ~€100/mo skip-by-default. TikTok discovery-funnel only. Adapter-honesty: scope advertises per-channel
  capability so UI never offers impossible push.
- HOSTING: every builder gates <script>/iframe behind paid tier (only self-host-WP + plain-HTML ungated).
  Port H cross-origin sandboxed iframe (different origin, postMessage origin-allowlist). One `<script>`
  first (covers all), marketplace apps (Shopify/Wix/WP.org/Webflow) where cut friction.
- BUILD ORDER: (1) landing-capture; (2) Port H embed; (3) Port F Telegram+WhatsApp; (4) Port E CSV/Sheets;
  (5) Port B conversion-upload; (6) C/D/G + marketplace apps. Honest cuts: Apollo, Higgsfield-as-port,
  Viber-default, TikTok/IG order-push, TikTok-Shop/Order-with-Google-checkout.

---
**Ground-truth код-якорі:** `proto-cap/src/{scope.rs,hybrid_gate.rs,roster.rs,signed_frame.rs}`,
`kernel/src/{order_machine,money,analytics,domain}.rs`, `kernel/src/pq/{rng.rs:475/465,hash.rs:352}`,
`crates/bebop/src/matcher.rs`, `web/src/lib/{push.js,channel.js}`, `apps/web/src/lib/messenger.ts`,
`server/` (axum channel+venue+healthz routes), `index.astro`+`Storefront.svelte:93` (`?ch=` stamp).

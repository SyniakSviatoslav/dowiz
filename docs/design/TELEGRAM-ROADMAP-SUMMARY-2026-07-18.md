# TELEGRAM ROADMAP SUMMARY — 2026-07-18

> 7 повідомлень, кожне < 800 символів (ліміт Telegram 4096; запас свідомий).
> PLAIN TEXT: `tg_send` (`tools/telemetry/lib.sh:121-125`) НЕ передає `parse_mode`
> (лише chat_id/text/disable_web_page_preview/message_thread_id) — тому жодного
> `**`/`_` форматування. Надсилати по черзі, MSG 1 → MSG 7, через власний Bash-виклик
> ліда: `source tools/telemetry/lib.sh && tg_send "$(...)"`. Throttle вже вбудований
> (TG_MIN_GAP 3.5s).

--- MSG 1 ---
DOWIZ ROADMAP — повний зріз, 2026-07-18.

Зараз існує: P01–P53, 5 компонентів екосистеми (CORE / PROTOCOL / DELIVERY / AGENT / ECOSYSTEM-OPS), 22 повних фазових блупринти + 9 layer-блупринтів (A–I) у docs/design/CORE-ROADMAP-2026-07-17/. Навігація: CORE-ROADMAP-INDEX.md; канон: MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md (§1–§14).

ГОЛОВНЕ: критичний шлях = P34 (mesh → kernel wiring) — важіль #1 всього роадмапу: ~70% протокольного коду вже збудовано й доведено тестами, але stranded від ядра dowiz.

Сьогодні паралельний swarm заземлив перший реальний код для P40/P41/P42/P47/P49 (+G3 render shell) — 29+ коммітів на main.

--- MSG 2 ---
CORE (P31–P33): ~90% готово, не вузьке місце. P31 math-first ядро — DONE-heavy (S0–S7); P32 гідравлічні контури — P32a DONE, wiring-gaps лишаються; P33 ledger-гігієна — PLANNED (аудит, без коду). Головна хвороба компонента: built-but-unwired.

PROTOCOL (P34–P36): P34 = наступна дія #1 (WIRING-GAP: усі поглинуті юніти DONE — claim_machine, HRW matcher, k-of-n PoD, KernelFacade). P35 docker-swap PARTIAL (DK-01/02/03/07 DONE, реальний wasip2-компонент). P36 — 2 живі регресії (див. MSG 7).

Swarm сьогодні (реальний код на main): CORDIC Q30 sin/cos (eqc-rs), Kalman SoA AVX2 consumer, Laplacian eigenmodes, drift-gate CI grep-gate — хвилі D/E/F/LAP зелені.

--- MSG 3 ---
DELIVERY (P37–P39, P47–P49, P51–P53) — продуктова поверхня.

P37 HTTP-поверхня — PLANNED (0%, головний розблоковувач майже всього). P38a WebGPU render — math substrate DONE; сьогодні G3 заземлив DOM-free FieldSim render shell (web/src, wasm-binding). P38b Sea & Sheet — PLANNED. P39 app-shell — Tauri 2 Wave-0 (рішення сьогодні).

P48 hub власника: єдина розмовна вісь (conversation spine) клієнт–власник–кур'єр поверх event_log + narrative-cinematic шар інтерфейсу (комміт 8d8d6fcbc). P49 ідентичність клієнта — код заземлено: kernel/src/ports/customer.rs (per-order capability grant, privacy-minimal). P51 карта: OSM vector + field-render маршрутів, без платних тайлів, повністю offline-capable. P52 кур'єрська поверхня (shift/claims/run/PoD/earnings) — blueprint готовий, MVP-blocking.

--- MSG 4 ---
AGENT (P40–P42): з "чат-бекенду" стає агентне ядро — swarm сьогодні заземлив код усіх трьох фаз (усі тепер PARTIAL):

- P40 AgentLoop executor: kernel/src/agent/loop.rs (651 рядків, fail-closed) — 626236886.
- P41 три режими (no-AI / local-offline / connected): kernel/src/ports/llm.rs — AiMode + BackendConfig::from_env, default Off — e74fc3e4f.
- P42 MCP port + tool boundary: kernel/src/ports/mcp.rs + ports/tool.rs — 575a75a20. Blueprint P42 також фіксує native local-browser MCP engine (fenced agentic-browser tool) і Skills-pattern discovery.

Auth: native Rust OAuth2 PKCE SSO як додатковий вхід hub-а, zero-custody (без Better Auth runtime) — f02de5d57. Структурна межа автономії агента: tools лише через capability-scoped ToolPort, прямий kernel-імпорт валить build (firewall red-proof).

--- MSG 5 ---
ECOSYSTEM/OPS (P22, P43–P46, P50, P53) — свідомо останні.

P43 інтеграційні порти (blueprint ~90KB): Telegram-first ChannelSend; httpSMS як власна SMS-інфра (платні Twilio-класу — опція); WhatsApp Cloud API; SimpleX Chat як додатковий privacy-канал; Proton Mail/Drive порт; media-import порт (Ghost-Downloader досліджено і ВІДХИЛЕНО — власний нативний fetch під жорсткими caps, без BitTorrent/HLS/YouTube).

P22 соцпостинг — Telegram Wave-0. P44 кеш-шари — LOW PRIORITY / далеке майбутнє. P45 ops floor — HARD-blocked на P37. P46 multi-product — термінальний вузол роадмапу. P50 compliance: аудит ON DISK (P50-COMPLIANCE-AUDIT.md), first-order gate відкритий. P53 Tor/onion: C-tor sidecar (не arti — за їхнім же warning), Onion-Location + QR, W0 збирається вже сьогодні.

--- MSG 6 ---
Рішення оператора, зафіксовані 2026-07-18:

- R-3 RootDelegationPolicy: Option A — OperatorSigned + монотонний IssuanceBudget (0512807bb).
- P39: Tauri 2 native wrapper — Wave-0, не відкладено (mobile Android+iOS з одного Rust-кодбейзу; NFC/biometric плагіни).
- P47 оплата: готівка → крипта → платіжні системи (Stripe тощо — лише готові аудитовані SDK, без власного коду).
- P48: WebGPU без DOM-винятку ("продовження рендер бекенду через фізику") + admin-поверхня Є hub-архітектурою: omnichannel intake, замовлення з будь-якого входу.
- P49 ідентичність: некритично до 5–50 реальних клієнтів; Wave-0 = per-order capability grant.
- Юридична позиція (P47-P50 §5.6): відповідальність на користувачах через згоду/ToS; протокол — нейтральна інфраструктура. Аудит звужено до ToS + consent-capture.

--- MSG 7 ---
Потребує уваги НЕГАЙНО:

1) P36 — 2 живі регресії в bebop: (a) no_std wasm32 build RED — E0425 в at_rest.rs:74 (fix = один use-рядок за прецедентом event_log.rs:22); (b) insecure-TLS default-on. Обидві поза критичним шляхом P34, але RED зараз.

2) Консистенс-аудит корпусу сьогодні: P31–P53 — без дублів і дір; усі внутрішні лінки INDEX + MASTER валідні (0 битих); line-drift виправлено (facade.rs:64 → :123 у 3 місцях, event_log.rs:359 → :330); §10.2 розширено до P31–P53; статуси P40/P41/P42/P47/P49 звірено з реальним кодом swarm-а і виправлено датованими нотатками.

3) Відкрите: блупринти P40–P49 ще НЕ reconciled design-vs-implementation (swarm міг реалізувати інакше, ніж спроєктовано) — це окремий свідомий пас.

# DOWIZ BRAND + EXPANSION — SESSION HANDOFF (2026-07-07)

## Done & verified this session
- **Brand system** authored: `docs/design/dowiz-brand/BRAND-BIBLE.md` — "Warm Cosmo-Noir" (Cowboy Bebop × cosmo-gothic × Ukrainian irony), the HYBRID thesis, "Hybrid is a feature, not a bug", full color/type/motion/texture + trilingual dry-wit voice (project-wide canon incl. sales/marketing; money/auth plain) + Horizon Drift spec.
- **Tokens:** `[data-skin="bebop"]` block added to `packages/ui/src/theme/tokens.css` (warm near-black field, amber `#E8A544`, teal `#46B0A4`, Fraunces/Oswald/Space Mono, daylight-courier variant, grain layer). Paper/Moebius scheduled for deletion (Pass 2).
- **Landing (Pass 2) built + LIVE at `/`:** `apps/web/src/pages/landing/` (`LandingPage.tsx`, `HorizonDrift.tsx`, `CityPopRadio.tsx`, `landing.css`). Nomadic skeleton (stage+HUD, gated sessions), Horizon Drift ship animation, 88.2 FM city-pop easter-egg (press R), dry-wit trilingual copy. `/start` = menu upload (no redirect). Fonts added to `apps/web/index.html`. **Verified: typecheck clean, build green, i18n parity 1520×3, screenshotted (looks great).**
- **Plan:** `docs/design/dowiz-brand/EXPANSION-PLAN.md` — authoritative. Layers 0/1/2, entry points, auth, security-review track, open-wa, + a **RESEARCH FINDINGS (distilled)** section holding verdicts from 8 of 9 research agents.
- **Memory:** `dowiz-brand-voice-canon-2026-07-07.md` (brand+voice canon) + `ask-dont-guess-clarify-2026-07-07.md`.

## Pending (next session)
1. ✅ All 9 research reports distilled into `EXPANSION-PLAN.md` → RESEARCH FINDINGS.
2. **Nothing committed yet** — all changes are in the working tree. Review, then commit on the feature branch (message per brand voice).
3. **🔴 First real fix (independent of brand work):** the `pnpm verify:secrets` CI step is a no-op (gitleaks never installed) — every CI run silently skips secret scanning. One-line install fix; do before OSS/prod push.

## Next steps (priority order)
- **Wire the city-pop stream:** set a verified free embeddable city-pop live stream id in `CityPopRadio.tsx` (`CITY_POP.youtubeId`, currently empty → shows dry-wit "signal lost") + allow `frame-src https://www.youtube-nocookie.com` in CSP. (Operator decision: which stream.)
- **Fix CSP** `connect-src`/`frame-src` — unblocks TMA (`telegram-web-app.js`), city-pop embed, and is prerequisite for voice.
- **Layer 0 (release hardening), order:** `cargo update -p num-bigint` → rsa fix (force `jwt-simple` `optimal`/BoringSSL, test RAM) → secret history scrub (git-filter-repo + GitHub Support cache purge) → LICENSE→AGPLv3 + reconcile SPDX to `-or-later` + headers → DCO via `dco-check-action` → SECURITY.md + Private Vuln Reporting + security.txt → then public.
- **Pass 2 rest (brand):** flip `paperSkinAttr()`→bebop, delete Paper block+helpers, migrate admin/courier + daylight toggle, visual-regression proof.
- **Auth (red-line, gated):** ship additive `POST /auth/telegram/tma` initData HMAC fast-path first; Better Auth pilot on OWNER login only (human-gated).
- **Entry points:** QR → TMA → WhatsApp **Cloud API** (NOT open-wa). Telemetry: deterministic `#sys` digest to existing `TELEGRAM_OPS_CHAT_ID` (separate ops bot token) + Fly's free managed Grafana.

## Operator decisions (2026-07-07 — DECIDED)
- **City-pop stream: APPROVED to "find one"** — next session: research a verified, free, embeddable Japanese city-pop live stream, wire its id into `CityPopRadio.tsx` (`CITY_POP.youtubeId`) + allow `frame-src https://www.youtube-nocookie.com` in CSP. (Deferred here only because we hit the hard context stop mid-wrap — this is a green-lit next-session task, not an open question.)
- **Better Auth pilot (OWNER login only): GO** — red-line; still requires the RED-proof (tampered hash / stale auth_date → 401) + staging validation + human gate before merge. Ship the additive Telegram `initData` HMAC fast-path first.
- **WhatsApp Cloud API over open-wa: GO** (confirmed — official, no ban risk, ~0 box RAM).

## Env note
Preview server was running on `http://localhost:4174/` (vite preview of `apps/web/dist`).

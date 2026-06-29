# Environment-variable classification (G5(c) — tooling-integration-eval)

> **Fail-closed classification gate.** Every env identifier in `packages/config/src/index.ts`
> matching `/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/` MUST appear below classified as
> `internal` or `external-subprocessor`. `scripts/guardrail-license.mjs` (`process.exit(1)`):
> - an **unclassified** matched env blocks (a new `SKYVERN_BASE_URL`/`DEEPEVAL_*`/`CONFIDENT_AI_*`
>   cannot merge until a human classifies it here);
> - every `external-subprocessor` row's **subprocessor** MUST also appear in `compliance/subprocessors.md`.
>
> This replaces the old closed `SERVICE_ENV` allowlist (Breaker H2/RA-1): the only path to green is an
> explicit, reviewable, semantically-loaded classification — not a hand-grown array a new external env
> can be slipped into invisibly. Seeded once (2026-06-29, reviewed). Residual = *misclassification*
> (declaring an external env `internal`) — an explicit compliance claim, owned by CI maintainer + reviewer.

| ENV | class | subprocessor |
|---|---|---|
| APP_BASE_URL | internal | — |
| BACKUP_ENCRYPTION_KEY | internal | — |
| COURIER_PII_ENCRYPTION_KEY | internal | — |
| DEV_AUTH_SECRET | internal | — |
| JWT_DEV_PRIVATE_KEY | internal | — |
| JWT_DEV_PUBLIC_KEY | internal | — |
| JWT_PRIVATE_KEY | internal | — |
| JWT_PUBLIC_KEY | internal | — |
| VAPID_PRIVATE_KEY | internal | — |
| VAPID_PUBLIC_KEY | internal | — |
| LLM_ENDPOINT | internal | — |
| MEM0_OLLAMA_URL | internal | — |
| TRANSLATION_ENDPOINT | internal | — |
| GOOGLE_CLIENT_SECRET | external-subprocessor | Google |
| GROQ_API_KEY | external-subprocessor | Groq |
| GROQ_ENDPOINT | external-subprocessor | Groq |
| OPENAI_API_KEY | external-subprocessor | OpenAI |
| OPENAI_ENDPOINT | external-subprocessor | OpenAI |
| OPENCODE_ZEN_API_KEY | external-subprocessor | OpenCode Zen |
| OPENCODE_ZEN_ENDPOINT | external-subprocessor | OpenCode Zen |
| OPENROUTER_API_KEY | external-subprocessor | OpenRouter |
| OPENROUTER_ENDPOINT | external-subprocessor | OpenRouter |
| R2_ENDPOINT | external-subprocessor | Cloudflare R2 |
| R2_PUBLIC_URL | external-subprocessor | Cloudflare R2 |
| R2_SECRET_ACCESS_KEY | external-subprocessor | Cloudflare R2 |
| REDIS_URL | external-subprocessor | Upstash Redis |
| RESEND_API_KEY | external-subprocessor | Resend |
| ROUTING_API_KEY | external-subprocessor | OpenRouteService |
| ROUTING_BASE_URL | external-subprocessor | OpenRouteService |
| SENTRY_DSN | external-subprocessor | Sentry |
| TELEGRAM_BOT_SECRET | external-subprocessor | Telegram |
| TELEGRAM_BOT_TOKEN | external-subprocessor | Telegram |

LAST-REVIEWED: 2026-06-29

### Notes
- `LLM_ENDPOINT` / `MEM0_OLLAMA_URL` / `TRANSLATION_ENDPOINT` are config knobs that default to
  self-hosted/local (ollama); the actual external egress paths are the provider-specific keys above
  (Groq/OpenAI/Zen/OpenRouter), each independently classified + registered.
- VAPID keys are self-generated (Web Push uses no third-party processor for key custody).

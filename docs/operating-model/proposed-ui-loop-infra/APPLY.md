# APPLY — UI Build-Verification Loop infra (operator action)

Layers 2–3 need protected files (`package.json` deps; `.github` for CI). `protect-paths.sh` blocks
direct edits. Below is what to add. Full spec: [ui-build-verification-loop.md](../ui-build-verification-loop.md).

## Layer 2 — Storybook (isolation + every state)
Add dev deps + scripts to root / `packages/ui` `package.json`:
```json
"devDependencies": {
  "storybook": "^8", "@storybook/react-vite": "^8", "@storybook/addon-a11y": "^8"
},
"scripts": {
  "storybook": "storybook dev -p 6006 -c packages/ui/.storybook",
  "build-storybook": "storybook build -c packages/ui/.storybook -o storybook-static"
}
```
Then `packages/ui/.storybook/{main,preview}.ts`; one story per state × variant × {390,768,1280} × {al,en}
(state list = the Task-Exit enrich, not re-invented).

## Layer 3 — deterministic visual harness (Playwright-in-Docker)
`@playwright/test` is already installed (`toHaveScreenshot` available). Add a visual project config with:
`reducedMotion:'reduce'`, fixed `timezoneId`, perceptual threshold, and **masks** for dynamic dowiz zones
(`RelativeTime`, MapLibre canvas, Recharts, avatars, `pickup_code`/QR). Generate baselines **only** in a
pinned Docker image so they're machine-independent:
```bash
pnpm build-storybook && npx serve storybook-static -p 6006 &
docker run --rm -e CI=true -e TEST_BASE_URL='http://host.docker.internal:6006' \
  -v "$PWD":/app -w /app mcr.microsoft.com/playwright:v1.4x-jammy \
  pnpm exec playwright test visual --update-snapshots   # omit flag to compare
```
Iron rule: never commit baselines generated outside the pinned image (non-deterministic = noise).
Optional review UI: Lost Pixel or Argos (OSS, self-host). Not Chromatic/AI-SaaS.

## Layer 4 — vision (optional, when OpenRouter credits return)
The loop currently uses Claude subagents as the eye (works, no creds). To use the cheaper OpenRouter
path from the spec, add a script that posts each screenshot + tokens/spec to a vision model and parses
the A–F JSON; rotate keys; scope to changed components only.

## CI enforcement (`.github/workflows/*` — protected)
Add to the FE job: `pnpm exec tsx scripts/i18n-parity.ts` and (once baselines exist) the Docker visual
compare, so the floor + visual gates run on every PR.

---
TRIGGER: packages/db/migrations/**read*public*menu**
CAUSE: >
  read_public_menu is redefined by several CREATE OR REPLACE migrations over
  time. The OLDEST definition (022) lacks the locale join and the published-
  status serving; the LIVE behavior lives in a LATER migration (033 added
  modifier/group localization + published serving; 055 added primary_media_id).
  Copying the stale 022 body silently reverts localization and status fixes,
  and dropping the `p_locale text DEFAULT ''::text` default breaks every
  caller that invokes read_public_menu(slug) with one argument.
ACTION: >
  When writing a new migration that CREATE OR REPLACEs read_public_menu in
  packages/db/migrations/** → cause: the function is redefined repeatedly and
  the oldest body is stale → do: grep ALL migrations that define/redefine
  read_public_menu (`grep -rl "CREATE OR REPLACE FUNCTION read_public_menu" packages/db/migrations/`),
  take the LAST one by migration number (or, safer, read the live function's
  signature/body straight from the deployed DB) — NEVER hard-code a specific
  migration number as "the livest" in this ACTION or anywhere else, the
  migration set grows (022→033→055→157+ and counting) and any fixed number
  written down today is stale tomorrow. Whichever body you copy, PRESERVE the
  `p_locale text DEFAULT ''::text` signature exactly — never drop the DEFAULT
  or the one-arg callers (SSR /s/:slug, MenuPage) break.
LINK: packages/db/migrations/1790000000055_read-public-menu-primary-media.ts:23 (rebuilt from live 033, NOT stale 022)
SCOPE: Migrations that redefine read_public_menu / read_public_menu_all_locales ONLY. Not other PL/pgSQL functions.
STATUS: active
---

# Redefining read_public_menu: copy the livest def, keep the DEFAULT param

Source: memory `product-media-seam-phase1.md`, `v1-hardening-ux-fixes.md`.

`read_public_menu` has been `CREATE OR REPLACE`d several times:

- `022` (`1780338982022_read_public_menu.ts`) — original, raw modifier names,
  no locale join.
- `033` (`1790000000033_localize-modifiers.ts`) — added modifier/group
  localization by `p_locale` and published/open status serving (the original
  gated on `status='active'` and returned "Menu not found" to crawlers).
- `055` (`1790000000055_read-public-menu-primary-media.ts`) — rebuilt from the
  LIVE 033 def (explicitly NOT stale 022), added `primary_media_id`, preserved
  the `p_locale text DEFAULT ''::text` signature.

When you redefine it again: this list is historical, not exhaustive (157+ migrations exist as
of 2026-07-02 and growing) — grep ALL migrations for `CREATE OR REPLACE FUNCTION
read_public_menu` and take the LAST one by number, or read the live function body from the
deployed DB. Never assume 055 (or any other specific number) is still "the livest" — that was
true only at authoring time. Whichever body you copy, keep
`p_locale text DEFAULT ''::text` — single-argument callers
(`read_public_menu('demo')`, used by SSR `/s/:slug` and `MenuPage`) break if
the DEFAULT is dropped.

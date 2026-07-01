# demo-builder — run report · ArtePasta

**Loop:** `demo-builder` (v0.1, CERTIFIED) · REUSE run (no loop changes)
**Prospect:** ArtePasta — Bulevardi Epidamn, Durrës 2001, Albania (Italian / pasta trattoria)
**Date:** 2026-07-01 · **Env:** staging (`https://dowiz-staging.fly.dev`) · **Mode:** preview-only (no outreach)
**Outcome:** ✅ **CERTIFIED-PREVIEW** → https://dowiz-staging.fly.dev/s/artepasta

---

## Metrics

### Outcome
| metric | value |
|---|---|
| outcome | `certified-preview` |
| final state | `VERIFIED` |
| needs-review | 0 |
| claim invites minted | **0** (preview-only; `--send-invite` not passed) |

### Menu quality (Layer 1)
| metric | value | gate |
|---|---|---|
| categories | 6 | ≥2 ✓ |
| items | 16 | ≥6 ✓ |
| description coverage | 100% (16/16) | ≥50% ✓ |
| price validity | all integer, in-band | ✓ |
| source | hand-authored | Wolt menu present but API-locked; RestaurantGuru had none |

### Branding (Layer 2)
| metric | value |
|---|---|
| cuisine seed | `italian` |
| primary / bg / text | `#3f7d4f` / `#fbf6ee` / `#111111` |
| text-on-bg contrast | ≥ 4.5 (AA) ✓ — distinct from Eljo's `#c1352b` & the sushi demo |

### Visual acceptance gate (Layer 3 — live Playwright, real DOM)
| viewport | items rendered | console errors | add btns | cart FAB | order btns | noindex |
|---|---|---|---|---|---|---|
| mobile | 16 (≥3 ✓) | 0 ✓ | 0 ✓ | 0 ✓ | 0 ✓ | true ✓ |
| desktop | 16 (≥3 ✓) | 0 ✓ | 0 ✓ | 0 ✓ | 0 ✓ | true ✓ |

gate `pass: true`, exitCode 0 — **certified on the first pass-2 attempt** (the hydration-wait fix held; no false negative).

### Timing (wall-clock)
| stage | ms |
|---|---|
| Pass 1 (create source, L1/L2 compute) | 352 |
| Enrich seam (menu_draft → ENRICHED, in-container) | 3,520 |
| Pass 2 (mint → spine → verify → **visual gate ×2**) | 5,594 |
| Theme seam (`location_themes` upsert, in-container) | 1,999 |
| **total** | **≈ 11.5 s** |

### Safety attestations (verified, not assumed)
- **Never-orderable (B3):** shadow spine — org `owner_id NULL`, location `status=closed` / `published_at NULL`; gate confirmed 0 add/cart/order affordances in the render.
- **noindex:** `x-robots-tag: noindex` present on both viewports.
- **No outreach:** 0 claim invites minted; no email.
- **Secret hygiene:** `PROVISION_OPS_SECRET` + DB creds stayed inside the staging container; the two DB seams ran in-box via `flyctl ssh` (secret never printed).

---

## Identifiers
| | |
|---|---|
| place_id | `gmaps:0x134fdb3c68de013d:0xada5292cc9e31ae1` |
| slug | `artepasta` |
| source_id | `f793f049-9a99-4712-8ca4-fef23241057a` |
| org_id | `8d919519-2a0b-402e-90d8-7a780c8cd70b` |
| location_id | `e11fe132-8af7-4f62-943c-fcb32fb126d5` |
| prospect input | `loops/prospects/artepasta.json` |
| run artifact | `loops/runs/demo-builder-2026-07-01T06-09-51-936Z.json` |

## Notes
- Menu **hand-authored** (Italian trattoria): ArtePasta's real menu is on Wolt, but Wolt's assortment API is locked (404/deprecated slug endpoints) and the page is client-rendered (no SSR menu). RestaurantGuru/Google had none. The owner replaces items/prices on claim.
- Follow-ups: `--send-invite` mints the 72h claim link (still no email dispatched); rerun `loops/prospects/artepasta.json` to refresh after editing the menu.

# Model Rotation Registry

> Maintained by `.agents/rules/harness-self-improvement.md` (harness loop). Update when a model joins or leaves the rotation.

## Active models

| Model | Identifier | Context window | Known weak spots | First seen |
|---|---|---|---|---|
| deepseek-v4-flash-free | opencode/deepseek-v4-flash-free | TBD | TBD | 2026-06-09 |

## Known weak spots (from `model-specific` failure tags)

_None recorded yet. Populate via the harness self-improvement loop (Phase B)._

## Onboarding checklist

When a new model joins the rotation:

- [ ] Run the coreset against it
- [ ] Record new failures as coreset entries (not model special-cases)
- [ ] Add model to this registry with known weak spots
- [ ] Prefer Tool-level fixes to neutralize cross-model variance
- [ ] Run existing green flows across the new model — flag any regressions

## Harness constraints derived from model rotation

- Top-level instruction files kept at ~120 lines max (smallest model's budget)
- Skills loaded on demand via `.agents/` (progressive disclosure)
- Tool interface normalized — no model's native dialect baked into Skills

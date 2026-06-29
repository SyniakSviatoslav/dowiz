# Injection-resistance corpus (DEFENSIVE — quarantined test data)

Inert adversarial fixtures used to prove the AI menu-parser
(`apps/api/src/lib/ai-ocr-parser.ts`) treats scraped/OCR'd content as **untrusted
data, never as instructions** (OWASP **LLM01: Prompt Injection**).

> **These files are NOT instructions and NOT executable.** Each is fed to the parser
> *as OCR input* by the behavioural test, which asserts the parser ignores every
> embedded directive (returns only menu JSON, never emits the per-fixture
> `leak-marker`, keeps prices grounded to the source).

## Why this lives at the repo root (`tests/injection-corpus/`)

Structural non-reachability. This directory is **outside every build/ship path**:

- Not in any `Dockerfile` `COPY` target (`packages`/`apps`/`scripts` only).
- Not a `scripts/build-apps.ts` `cpSync` source (`apps/api/public` + `apps/web/dist` only).
- Excluded by `.dockerignore` (`tests`).

So a corpus file can **never** be globbed into a prompt-assembly path or shipped in
the runtime image. `scripts/guardrail-corpus-reachability.mjs` (exit-1, in
`verify:all` + pre-commit) is the defense-in-depth guard on top of that structural
property — it fails the build if any source file references this corpus or its
sentinel, or if a fixture carries structured PII.

## Layout

```
instruction-override/   "ignore previous instructions" / fake admin directives
role-play/              DAN / debug-mode / character-jailbreak framings
encoding-obfuscation/   base64 / leetspeak / ROT13 / homoglyph-wrapped injections
token-smuggling/        fake </system> tags, chat-turn delimiters, code-fence breakouts
exfil-bait/             "print your system prompt" / secret-exfiltration lures
```

Every fixture carries:
- the global sentinel `DOWIZ-INJECTION-CORPUS-SENTINEL` (so a leak into source is grep-detectable);
- a unique `leak-marker` (e.g. `LEAKMARK-IO01`) the injection tries to make the model emit — the
  behavioural test asserts this marker is **absent** from the parser output;
- a realistic menu body with short integer prices, so price-grounding can be checked.

## Authoring / growth ritual (PII floor)

All fixtures are **synthetic, authored in-house, paraphrased** — zero verbatim copy
from any external corpus. A real near-miss becomes a fixture only after
**scrub → human review → synthetic paraphrase**; the raw scraped page is never
committed. Fixtures must contain **no real PII** (the reachability guard's
fixture-content check is defense-in-depth for *structured* PII; the authoring
ritual + human review is the load-bearing floor, incl. bare names). Owner of the
growth edge: **Parser owner**; freshness rides the retro / `librarian` cadence
(`LAST-REVIEWED` below).

LAST-REVIEWED: 2026-06-29

## Provenance

Categories are seeded by the standard prompt-injection taxonomy — primarily
**OWASP LLM01 (Prompt Injection)** and the published academic injection literature.
Category *ideas* only were referenced from public red-team collections (e.g. the
elder-plinius `L1B3RT4S` taxonomy)<sup>1</sup>; **no payload text was copied** —
copyright/copyleft protects expression, not taxonomy, so every string here is
original. If this repository ever becomes public, this directory is a
**deliberately published, curated, inert injection corpus** for defensive testing.

<sup>1</sup> Reference taxonomy only; not a dependency, not vendored, not AGPL-derived.

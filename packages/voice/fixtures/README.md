# Voice real-audio eval fixtures

The `audio-eval` harness (`../scripts/audio-eval.ts`) is the **H1 "harness-deterministic" gate
artifact** (ADR-0015 §4.2): it runs a fixed WAV corpus through the *real* `WhisperProvider`
(`TransformersTranscriber` → the same matcher the storefront uses) and scores IRA +
dangerous-misfire per locale. It is deliberately **not** cloud CI — it needs the whisper model +
`@huggingface/transformers`, so it runs as a gated local/self-hosted harness whose report *is* the
gate evidence (cloud CI proves only matcher/gate wiring on text).

## Two data regimes (do not mix them)

- **`jfk-en-16k.wav` + `smoke-manifest.json` — a SMOKE fixture, not a gate.** One public-domain
  English clip. It proves the *pipeline* runs end-to-end on real audio (decode → real transcription →
  matcher → scoring → report) and that a non-command clip **fail-quiets** to no intent. It certifies
  nothing about accuracy.
- **The LAUNCH corpus is a separate, C2-consented research dataset** (resolution.md C2 / M1 / C-4):
  **≥300 clips per locale, ≥15 distinct speakers**, both noise conditions, deliberate adversarial
  near-miss pairs (accept/reject, with/without, allergen-adjacent), from **recruited adult speakers —
  explicitly NOT the platform's own couriers/workforce**. It lives in its own access-controlled,
  encrypted, time-boxed (90-day) store outside any tenant DB — **never committed to this repo**.
  Point the harness at that manifest to produce a launch-grade report.

## Audio format

16 kHz **mono**, 16-bit PCM WAV. The harness decoder validates this and errors clearly otherwise;
pre-resample/downmix the corpus before running (the launch corpus is normalized as part of its
documented recording protocol).

## Manifest schema

```jsonc
[
  {
    "wav": "clip.wav",             // path (relative to the manifest) or https URL
    "locale": "sq" | "en" | "uk",
    "expected_kind": "SET_SORT" | null,  // null = a fail-quiet clip (must resolve to NO intent)
    "expected_args": { "by": "price" },  // optional; checked against the proposal args
    "transcript_contains": "substr",     // optional ASR sanity substring
    "menu": { "products": [], "categories": [] }  // optional per-clip menu context for slot resolution
  }
]
```

## Running

```bash
# from repo root — needs @huggingface/transformers resolvable to packages/voice (heavy, optional dep)
node --import tsx packages/voice/scripts/audio-eval.ts \
  packages/voice/fixtures/smoke-manifest.json --out report.json --dtype q8 --device cpu
```

Proven 2026-07-01 against `Xenova/whisper-base` (cpu/q8): the smoke clip transcribed verbatim and
correctly resolved to **no** intent (fail-quiet), ~3.2 s/clip cold. Determinism holds given the
pinned model + greedy decode (temperature 0, `num_beams: 1`, `do_sample: false`).

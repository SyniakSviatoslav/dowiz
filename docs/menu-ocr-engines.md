# Menu-import OCR engines

The menu-import OCR step (behind `MenuParserProvider` → `AiOcrParser`) supports
two interchangeable OCR engines for **image** inputs. The engine is selected by
configuration; the rest of the pipeline (LLM structuring, PII redaction,
canonical draft, and owner human-review) is identical regardless of engine.

> PDF inputs always use direct text extraction (pdfjs), not OCR.

## Engines

| Engine | How | Default | Notes |
|--------|-----|---------|-------|
| `tesseract` | in-process `tesseract.js` (`sqi+eng`) | ✅ yes | zero extra setup; the safe default |
| `paddle` | on-demand PaddleOCR (PP-OCRv5/v6) subprocess | no | higher accuracy; requires a local Python venv |

## Selecting the engine

Resolution order (first match wins), lower-cased:

1. `config.ocr_engine` on the parse request (per-import)
2. `MENU_OCR_ENGINE` environment variable (global)
3. default → `tesseract`

When unset, **no external process is spawned** — the default stays in-process,
so there is zero product-runtime impact until `paddle` is explicitly opted into.
The engine actually used is recorded in the result provenance (`ocr_engine`).

## Enabling PaddleOCR (`paddle`)

PaddleOCR runs as an **on-demand subprocess** (not a daemon). It needs a local
Python environment with `paddlepaddle` + `paddleocr` (both Apache-2.0):

```bash
# one-time, from the repo root
python3 -m venv .venv-paddle
.venv-paddle/bin/pip install paddlepaddle paddleocr
```

Then point the API at it and turn the engine on:

| Env var | Default | Purpose |
|---------|---------|---------|
| `MENU_OCR_ENGINE` | `tesseract` | set to `paddle` to enable |
| `PADDLE_OCR_PYTHON` | `python3` | interpreter (e.g. `.venv-paddle/bin/python`) |
| `PADDLE_OCR_SCRIPT` | `<cwd>/scripts/paddle-ocr.py` | the OCR script |
| `PADDLE_OCR_LANG` | `sq` | PP-OCRv5/v6 language (Albanian/Latin; e.g. `en`) |

> Deployment: to use `paddle` in production the API image must include Python +
> `paddleocr` + `apps/api/scripts/paddle-ocr.py`. Since the default is
> `tesseract`, the standard deploy is unaffected until you opt in.

## Source & proof

- Subprocess script: `apps/api/scripts/paddle-ocr.py`
- Provider wiring: `apps/api/src/lib/ai-ocr-parser.ts` (image OCR branch)
- Proof test: `apps/api/tests/paddle-ocr-seam.test.ts`
  — run from `apps/api`: `node --test --import tsx tests/paddle-ocr-seam.test.ts`

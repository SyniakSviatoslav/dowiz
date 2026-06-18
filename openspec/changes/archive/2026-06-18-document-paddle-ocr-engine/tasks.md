## 1. Author documentation

- [x] 1.1 Create `docs/menu-ocr-engines.md` covering both engines, the
      `MENU_OCR_ENGINE` switch, and the safe `tesseract` default
- [x] 1.2 Document enabling `paddle`: the `.venv-paddle` setup and the
      `PADDLE_OCR_PYTHON` / `PADDLE_OCR_SCRIPT` / `PADDLE_OCR_LANG` env vars
- [x] 1.3 Link the proof test (`apps/api/tests/paddle-ocr-seam.test.ts`) and the
      subprocess script (`apps/api/scripts/paddle-ocr.py`)

## 2. Verify

- [x] 2.1 `openspec validate document-paddle-ocr-engine --strict` passes
- [x] 2.2 No code/build impact (docs-only); commit does not break husky gates

## Why

Stage A shipped PaddleOCR as a config-selectable OCR engine behind
`MenuParserProvider`, but the switch (`MENU_OCR_ENGINE`, default `tesseract`)
and its operational requirements (the local Python venv, `PADDLE_OCR_PYTHON` /
`PADDLE_OCR_SCRIPT`) are undocumented. An operator enabling `paddle` today has
no canonical reference, and there is no written contract for the safe default.
This is also the first real change exercised through OpenSpec (Stage B proof).

## What Changes

- Add `docs/menu-ocr-engines.md` documenting the two OCR engines, the
  `MENU_OCR_ENGINE` switch, the safe `tesseract` default, and how to enable
  `paddle` (venv + env vars), with a pointer to the proof test.
- No code or runtime behavior changes; documentation only.

## Capabilities

### New Capabilities
- `menu-ocr`: the menu-import OCR engine selection contract — which engine runs,
  how it is selected, and the safe default.

## Impact

- New file: `docs/menu-ocr-engines.md`. No code, API, schema, or dependency
  changes. Zero product-runtime impact (default engine unchanged).

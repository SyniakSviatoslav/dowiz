## ADDED Requirements

### Requirement: Selectable menu-import OCR engine with a safe default

The menu-import OCR step behind `MenuParserProvider` SHALL support more than one
OCR engine, selected by configuration, and SHALL default to the in-process
`tesseract` engine when no selection is made.

#### Scenario: Default engine when unset
- **WHEN** a menu image is parsed and neither `config.ocr_engine` nor the
  `MENU_OCR_ENGINE` environment variable is set
- **THEN** the `tesseract` engine is used and no external process is spawned

#### Scenario: Opting into PaddleOCR
- **WHEN** `MENU_OCR_ENGINE` (or `config.ocr_engine`) is set to `paddle`
- **THEN** the PaddleOCR (PP-OCRv5/v6) subprocess performs OCR for that import
- **AND** the engine actually used is recorded in the result provenance

#### Scenario: Documentation exists for operators
- **WHEN** an operator needs to enable or audit the OCR engine selection
- **THEN** `docs/menu-ocr-engines.md` documents the switch, the safe default,
  the PaddleOCR setup (venv + `PADDLE_OCR_PYTHON`/`PADDLE_OCR_SCRIPT`), and the
  proof test that exercises the paddle path

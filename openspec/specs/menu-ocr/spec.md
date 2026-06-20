# menu-ocr Specification

## Purpose
Define the menu-import parsing pipeline: how raw menu images/PDFs become a
canonical, owner-reviewable draft. The pipeline is **selectable and zero-cost by
default** — OCR engine (tesseract/paddle) and structuring backend (heuristic/LLM)
are swappable by configuration, and the whole path runs with **no paid API and
no keys configured**. A real LLM, when wired, is an optional accuracy upgrade.

## Requirements
### Requirement: Menu structuring works with no LLM and no paid API

The structuring step (raw OCR/PDF text → canonical draft) SHALL produce a usable
draft when no LLM provider is configured, using an in-process heuristic
structurer that makes no network calls and requires no API key. A configured LLM
(self-hosted `ollama`, or BYO-key `groq`/`openai`) MAY be used as an optional
upgrade. No menu-parsing path SHALL depend on a paid vision API.

#### Scenario: Heuristic default when no LLM is configured
- **WHEN** menu text is structured and none of `LLM_ADAPTER`, `LLM_PROVIDER`,
  `LLM_ENDPOINT`, `GROQ_API_KEY`, or `OPENAI_API_KEY` is set
- **THEN** the in-process heuristic structurer produces the draft with no network
  call, and the provider used is recorded in result provenance

#### Scenario: Optional LLM upgrade
- **WHEN** an LLM provider is explicitly configured (e.g. `LLM_PROVIDER=ollama`
  or `GROQ_API_KEY` is present)
- **THEN** that provider performs structuring for the import
- **AND** its identity is recorded in result provenance for audit


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


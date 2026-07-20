---
CONTEXT:   Phase-2 migration 055 to add primary_media_id to read_public_menu (CREATE OR REPLACE).
DECISIONS: Copied the function body from the FIRST migration that defined read_public_menu
           (1780338982022) + added one field, instead of from the latest live definition.
WHERE:     Prod release_command aborted: "cannot remove parameter defaults from existing
           function"; the copy also silently dropped published-status serving (032), locale-
           aware modifiers (033), empty-category fix (016), slug lookup (018).
WHY:       Wrong assumption that the EARLIEST migration defining a plpgsql function is its
           canonical/current shape. A function is the LATEST CREATE OR REPLACE, not the first;
           I ignored migrations 016/018/032/033 that redefined it (incl. adding the
           `p_locale text DEFAULT ''::text` param default). I should have queried the live
           function signature/body (or grepped ALL definitions) before copying.
CONFIDENCE: high
NEXT-TIME: Before CREATE OR REPLACE of an existing DB function: grep EVERY migration defining
           it, take the last; or read the live signature from the deployed DB. Never assume the
           first definition is current.
LINK:      packages/db/migrations/1790000000055_read-public-menu-primary-media.ts ; commit d7daa1be
---

DISPOSITION (librarian, 2026-07-20): archived without re-distilling. The lesson this reflection
produced (docs/lessons/2026-06-22-read-public-menu-redefine.md, TRIGGER
`packages/db/migrations/**read*public*menu**`) was PRUNED this run — `packages/db/**` no longer
exists (legacy thin-layer removal, REGRESSION-LEDGER row #21, 2026-07-13; root CLAUDE.md
"drop js"). The TRIGGER glob can never match again. No successor lesson: the underlying DB
migration mechanism this fix targeted was deleted wholesale, not superseded by an equivalent
Rust-kernel path.

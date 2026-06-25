# ADR 0008: BOM Seam — Polymorphic `recipe_components` Node (Schema Now, Derived Runtime Later)

**Status:** PROPOSED (design-time — Triadic Council). Implements brief §6.2 (SEAM tier).
**Version:** v3 — close the AFTER-DELETE `FOR EACH ROW` TRUNCATE gap with a statement-level `AFTER TRUNCATE`
companion trigger (Breaker R2-M2); stop over-claiming native-FK equivalence.
v2 — hardened after Breaker H2/H3/L1 (honest re-scope of the migration-free claim; AFTER-DELETE
referential guard; deferred cycle-guard flag). **Resolution:** `…/resolution.md` H2/H3/L1/R2-M2.
**Supersedes:** nothing · **Extends:** the "schema now, runtime later" pattern established by
ADR 0002 (`product_media` seam) and the RLS conventions of `1790000000054_product-media-seam.ts`.
**Companion design:** `docs/design/mvp-sensor-seams/proposal.md` §2.2.
**Red-line:** 🔴 `packages/db/migrations/` (irreversible DDL). Inert at MVP — no runtime reader.

## Context

The North-Star availability model is `min(derived-from-ingredients, manual-cap)` (brief §2.3). At MVP we
ship **only the schema** so the irreversible BOM DDL/RLS cost is paid once, in the cheapest window, with
**FLAT/manual runtime** (no recursion, no derivation, no tree-walk — brief §6.2, §7). The load-bearing claim
to prove: *recipes reference a node → the manual→derived upgrade is migration-free*. A wrong seam here is
the exact retro-migration the brief most fears.

Grounded: `ingredients`/`recipe_components` are confirmed **absent** (grep over all migrations). `products`
is a rich table (price/translations/media/modifiers) — its identity must not be polluted.

## Decision

**Two inert tables, RLS FORCE from creation (copy `product_media` exactly).**

```sql
-- ingredients (raw + intermediate/batch nodes)
CREATE TABLE ingredients (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  location_id   uuid NOT NULL REFERENCES locations(id),
  name          text NOT NULL,
  kind          text NOT NULL DEFAULT 'raw'       CHECK (kind IN ('raw','intermediate')),
  is_batch_made boolean NOT NULL DEFAULT false,
  unit          text,
  current_stock numeric,                          -- manual count; NULL = untracked
  tracking_mode text NOT NULL DEFAULT 'untracked' CHECK (tracking_mode IN ('untracked','manual','derived')),
  waste_pct     numeric NOT NULL DEFAULT 0,
  reset_cadence text,
  last_set_at   timestamptz,
  created_at    timestamptz NOT NULL DEFAULT now()
);

-- recipe_components: "node consumes qty of ingredient"
CREATE TABLE recipe_components (
  id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  location_id    uuid NOT NULL REFERENCES locations(id),
  parent_kind    text NOT NULL CHECK (parent_kind IN ('product','ingredient')),
  parent_id      uuid NOT NULL,                   -- POLYMORPHIC: products.id OR ingredients.id (no DB FK)
  ingredient_id  uuid NOT NULL REFERENCES ingredients(id),
  qty_per_parent numeric NOT NULL,
  unit           text,
  created_at     timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX recipe_components_parent_idx     ON recipe_components (location_id, parent_kind, parent_id);
CREATE INDEX recipe_components_ingredient_idx ON recipe_components (ingredient_id);
```

Both tables get, **in the same migration** (mirror `product_media` lines 63-83):
```sql
ALTER TABLE <t> ENABLE ROW LEVEL SECURITY;
ALTER TABLE <t> FORCE  ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON <t>
  USING ( location_id IN (SELECT app_member_location_ids()) )
  WITH CHECK ( location_id IN (SELECT app_member_location_ids()) );
REVOKE ALL ON <t> FROM anon, authenticated, service_role;
-- GRANT SELECT,INSERT,UPDATE,DELETE TO deliveryos_api_user (guarded by pg_roles EXISTS).
```

### Why polymorphic, not two tables / double-FK / products-as-ingredients

Decision matrix in proposal §2.2. Polymorphic wins because real-world nodes cross the
'product'/'ingredient' boundary freely (a batch sauce can be both consumed and sold), so any **table boundary**
(Option B) forces a cross-table row MOVE — the retro-migration the brief fears. The fallback if polymorphic
integrity proves fragile is the **double-nullable-FK + CHECK exactly-one** (Option C): an in-place column
edit, not a row move. Products-as-ingredients (Option D) pollutes the rich `products` identity — rejected.

### Proof: manual→derived is migration-free — the HONEST scope (Breaker H2)

The brief's "migration-free upgrade" means **the manual→derived READER swap is free** — and that is **proven**
below. It does NOT mean "every future BOM topology change is free"; the Breaker correctly showed the FLAT MVP
runtime authors only **direct product→raw-ingredient** rows (no UI/runtime for intermediate/batch nodes), so
*introducing a new intermediate node into an existing flat recipe later* is a deliberate, owner-initiated
re-modelling, not a free reader swap. v2 states both halves honestly:

**(a) Reader swap — genuinely migration-free [PROVEN]:**
- **Manual (MVP):** product availability = `(products.stock_remaining IS NULL OR > 0)` (ADR-0007). No
  `recipe_components` row is dereferenced. The seam is inert.
- **Derived (North Star), FLAT topology:** a NEW read-only SQL function computes
  `available_units(product) = min( manual_cap, floor( min over recipe_components of ingredient.current_stock /
  qty_per_parent ) )`. For the rows the FLAT MVP actually authored (product → raw ingredient), **this is a
  SELECT over the rows that already exist** — no `parent_id` remap, no `kind` flip, no table move, no row
  rewrite. The migration is: **add a function + flip `tracking_mode` to 'derived'**. Zero DDL on
  `recipe_components`. **Claim proven for the reader swap.**

**(b) Introducing an intermediate/batch node later — a named, owner-driven backfill [NOT free, honestly
documented]:** when an owner decides "this sauce is now a batch node sold retail AND consumed by 3 dishes,"
the BOM UI performs: (1) insert `ingredients(kind='intermediate')`; (2) author the intermediate's child
recipe (`parent_kind='ingredient'` rows); (3) re-point the affected products' existing `recipe_components`
from the raw ingredients to the intermediate. **Step (3) is a row UPDATE** — a deliberate data edit the owner
initiates through the future BOM UI, **NOT a schema migration and NOT a silent surprise**. The recursive
reader of §"derived" already handles `parent_kind='ingredient'` rows, so no schema delta is needed to *read*
the new topology — only the owner's data edit to *create* it. This is the one backfill the seam needs, and it
is owner-driven application data, not a ret-migration of the kind the brief fears (a forced, hidden DDL/remap
at upgrade time). The brief's invariant — "the manual→derived **upgrade** is migration-free" — holds; "every
later re-modelling is free" was never the claim, and v2 says so.

### Concurrency-correctness of the single-node shape

A shared ingredient is ONE `ingredients` row. When the derived runtime lands, the atomic guard (ADR-0007's
conditional-UPDATE primitive) is applied to that **one node**, so concurrent orders contend on the same row →
no double-spend. A naïve flatten (copying an ingredient into N product rows) would let concurrent orders
decrement *copies* → oversell. The brief calls this out (§6.2: "наївний flatten переплатив би під гонкою").
The single-node shape is the concurrency-correct cut, and it is what this seam preserves.

## Integrity of the missing `parent_id` FK — write-side AND delete-side (Breaker H3)

Polymorphic `parent_id` has no DB FK. Integrity is held by **four** guards (v1 had only the first three, which
covered INSERT but NOT the parent's DELETE → orphan rows on product delete):
1. `parent_kind` CHECK to the enum.
2. An application assertion on write (the referenced parent exists).
3. **Inertness** — no reader dereferences `parent_id` until the derived reader lands; that reader adds the
   integrity join then.
4. **(NEW) An `AFTER DELETE` referential guard** giving the cascade semantics the polymorphic column cannot
   declare natively — in the same seam migration:
```sql
CREATE OR REPLACE FUNCTION recipe_components_parent_cascade() RETURNS trigger AS $$
BEGIN
  DELETE FROM recipe_components
   WHERE parent_kind = TG_ARGV[0] AND parent_id = OLD.id;
  RETURN OLD;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER products_recipe_cascade
  AFTER DELETE ON products
  FOR EACH ROW EXECUTE FUNCTION recipe_components_parent_cascade('product');
CREATE TRIGGER ingredients_recipe_cascade
  AFTER DELETE ON ingredients
  FOR EACH ROW EXECUTE FUNCTION recipe_components_parent_cascade('ingredient');
```

**The TRUNCATE gap (Breaker R2-M2) — closed with a statement-level companion trigger.** A `FOR EACH ROW`
DELETE trigger does **not** fire on `TRUNCATE` (Postgres fires only statement-level `AFTER TRUNCATE` triggers
for that path). So the v2 claim "exactly like a native `ON DELETE CASCADE` FK" was **false for the TRUNCATE
path** — a `TRUNCATE products CASCADE` (a common test-data-reset / tenant-purge shortcut, and the repo already
runs bulk test-data ops) would orphan `recipe_components` exactly as the original H3. v3 adds the
statement-level companion so both paths are covered:

```sql
-- statement-level guard for the TRUNCATE path the FOR EACH ROW trigger cannot see.
-- On TRUNCATE products/ingredients, remove the now-parentless recipe_components rows.
CREATE OR REPLACE FUNCTION recipe_components_parent_truncate() RETURNS trigger AS $$
BEGIN
  DELETE FROM recipe_components WHERE parent_kind = TG_ARGV[0];
  RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER products_recipe_truncate
  AFTER TRUNCATE ON products
  FOR EACH STATEMENT EXECUTE FUNCTION recipe_components_parent_truncate('product');
CREATE TRIGGER ingredients_recipe_truncate
  AFTER TRUNCATE ON ingredients
  FOR EACH STATEMENT EXECUTE FUNCTION recipe_components_parent_truncate('ingredient');
```

With both triggers, a hard single-row DELETE AND a TRUNCATE remove the dependent `recipe_components` rows.
Inert-safe (zero rows at MVP). **Honest scope (no over-claim):** this is referential-integrity-by-trigger, not
a declared FK — it is *functionally equivalent to `ON DELETE CASCADE` for the DELETE and TRUNCATE paths*, but a
declared FK additionally gives the planner referential metadata and rejects an orphan *insert* the polymorphic
column cannot. The remaining honest gap is the **soft-delete-without-hard-purge** path: products are
soft-deleted in the menu manager, so neither trigger fires and `recipe_components` linger by design until a
hard purge — which is **correct** (a soft-deleted product may be restored; cascading its recipe would lose it),
and the future derived reader filters on the product's live state, so a soft-deleted product's recipe is never
read into a BOM aggregate. The runner-up Option C (double-nullable-FK + CHECK) remains the documented exit if
trigger-based integrity proves insufficient. v3 states this scope rather than claiming blanket FK-equivalence.

## Deferred (North-Star) — recorded MISSING flags (Breaker L1)

The recursive derived reader is out of this batch's scope, but two guards MUST exist before it ships, and the
*data* that would break them is enterable today (the seam allows `parent_kind='ingredient'` rows), so the
guards must validate **pre-existing** rows, not just new INSERTs:
- **Cycle guard (L1):** the future `available_units()` tree-walk MUST carry a depth-cap + visited-set memo AND
  a one-time validation pass that rejects/flags any pre-existing A↔B cycle. A manually-entered or import-bugged
  cycle is inert today but a non-terminating reader tomorrow. Owner: North-Star phase lead. **Status: MISSING
  until the derived reader lands.**
- The integrity join (Option C exit) that dereferences `parent_id` lands with the same reader.

## Proof / DoD

- **Inertness test**: an order with recipes present but `tracking_mode='untracked'` behaves **byte-identically**
  to today (the kill criterion — zero regression). The reader never touches these tables.
- **RLS test**: cross-tenant SELECT on `ingredients`/`recipe_components` returns 0 rows (extend
  `packages/db/scripts/verify-rls.ts`).
- **Orphan-cascade test (H3)**: insert a product + its `recipe_components`; DELETE the product → its
  `recipe_components` rows are gone (the AFTER DELETE FOR EACH ROW trigger). Same for an ingredient parent.
- **Orphan-TRUNCATE test (R2-M2)**: insert products + their `recipe_components`; `TRUNCATE products CASCADE` →
  the matching `recipe_components` rows are gone (the AFTER TRUNCATE statement-level trigger). This is the path
  the FOR EACH ROW trigger does NOT cover; red before the statement-level trigger, green after.
- **Migration-free assertion (doc-level)**: the future derived reader's SQL is sketched above and reviewed to
  confirm the **reader swap** reads only pre-existing columns — no schema delta. The intermediate-node
  introduction is documented as the one owner-driven backfill (not a hidden ret-migration). (No runtime test
  now; the seam is inert by design.)

## Consequences

- The irreversible BOM DDL/RLS lands once, cheaply, before any North-Star pressure.
- `courier_sequence` (brief §6.2, claimed "уже в orders") is **not** found in `orders` by grep — if confirmed
  absent on landing, add `ALTER orders ADD courier_sequence int` as a trivial additive seam in the same batch.
- Forward-only: `down()` is a no-op (inert tables; a down-migration would only risk dropping a table once the
  North-Star phase has written real recipe data — same posture as ADR-0002).

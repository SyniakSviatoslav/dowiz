import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * Seed real `product_media` for the demo storefront — Phase-2 (slug-robust follow-up to 056,
 * which hardcoded slug='sushi-durres'; staging's published demo uses slug='demo'). Targets
 * slug IN ('demo','sushi-durres') so it seeds whichever the environment has. Idempotent:
 * cinematic-product-media E2E fixture. Lets the lazy media endpoint surface a
 * non-empty gallery + a `spin` on staging WITHOUT R2: every `storage_key` /
 * `poster_key` / spin frame is a public CC `https://` Wikimedia URL, which the
 * resolver passes through verbatim (no /media/ proxy needed).
 *
 * Also flips that location to `plan='business'` so the server-side tier gate
 * (plan !== 'business' → {media:[]}) actually opens for the demo.
 *
 * Forward-only + idempotent + a no-op when sushi-durres does not exist:
 *  - the plan flip and every INSERT…SELECT key off `locations.slug='sushi-durres'`;
 *    if the slug is missing, every SELECT yields zero rows → nothing happens.
 *  - each product is guarded by `NOT EXISTS (… product_media pm WHERE pm.product_id = p.id)`
 *    so a re-run inserts nothing once media is present.
 *  - primary_media_id is only set where it is still NULL.
 * down() is a no-op (matches the seam migrations 046/054 forward-only stance).
 *
 * Picks the first 3 products of the location by (sort_order, id) for the image
 * galleries; the first of those three also gets a `spin` row.
 */

// Reused from migration 046's curl-verified Wikimedia CC/PD pool (hotlink-safe,
// CSP img-src https:). Indices chosen for visual variety per product.
const IMG = [
  'https://upload.wikimedia.org/wikipedia/commons/thumb/d/d3/Sushi_combination.jpg/960px-Sushi_combination.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/e/ed/Sushi_food_in_Tokyo%2C_Japan.jpg/960px-Sushi_food_in_Tokyo%2C_Japan.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/2/22/Vegan_sushi_plate.jpg/960px-Vegan_sushi_plate.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/0/03/Makizushi2.jpg/960px-Makizushi2.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/3/3b/Nigiri_Sushi_%2826478725732%29.jpg/960px-Nigiri_Sushi_%2826478725732%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/a/a1/Nigiri_Sushi_%2826478732232%29.jpg/960px-Nigiri_Sushi_%2826478732232%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/e/e8/Tuna_nigiri_sushi_-_Sushiko_%282648979899%29.jpg/960px-Tuna_nigiri_sushi_-_Sushiko_%282648979899%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/b/bd/Deluxe_sashimi_platter_01.jpg/960px-Deluxe_sashimi_platter_01.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/d/d0/Deluxe_sashimi_platter_02.jpg/960px-Deluxe_sashimi_platter_02.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/b/b7/Sashimi-01.jpg/960px-Sashimi-01.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/3/35/Tuna_Sashimi_%286858891649%29.jpg/960px-Tuna_Sashimi_%286858891649%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/d/dc/Salmon_Sushi_and_Sashimi_Platter_-_W_Sushi.jpg/960px-Salmon_Sushi_and_Sashimi_Platter_-_W_Sushi.jpg',
];

// 3 image storage_keys per gallery product (sort_order 0,1,2). Three distinct
// triples so the 3 seeded products don't all show the same photos.
const GALLERIES: string[][] = [
  [IMG[0], IMG[4], IMG[7]],
  [IMG[1], IMG[5], IMG[8]],
  [IMG[2], IMG[6], IMG[9]],
];

// 4 pseudo-frames for the single `spin` row + a poster (one of them).
const SPIN_FRAMES = [IMG[10], IMG[11], IMG[3], IMG[0]];
const SPIN_POSTER = SPIN_FRAMES[0];

const MIME = 'image/jpeg';
const ALT = 'Sushi photo';

export async function up(pgm: MigrationBuilder): Promise<void> {
  // 0. Tier gate: flip the demo location to business (no-op if slug missing).
  pgm.sql(`
    UPDATE locations SET plan = 'business' WHERE slug IN ('demo', 'sushi-durres');
  `);

  // The first 3 products of the location, ranked 0..2 by (sort_order, id).
  // `target_products` CTE is reused by every INSERT below.
  const targetCte = `
    target_products AS (
      SELECT p.id AS product_id,
             p.location_id,
             (row_number() OVER (ORDER BY p.sort_order, p.id) - 1)::int AS rank
      FROM products p
      JOIN locations l ON l.id = p.location_id
      WHERE l.slug IN ('demo', 'sushi-durres')
        AND NOT EXISTS (
          SELECT 1 FROM product_media pm WHERE pm.product_id = p.id
        )
      ORDER BY p.sort_order, p.id
      LIMIT 3
    )`;

  // 1. Image galleries — 3 image rows per product (sort_order 0,1,2).
  //    rank (0,1,2) selects which gallery triple; (rank,slot)→storage_key via the
  //    `gallery(rank, slot, url)` VALUES table. ~30KB pseudo-bytes per row.
  const galleryValues = GALLERIES.flatMap((triple, rank) =>
    triple.map((url, slot) => `(${rank}, ${slot}, '${url}')`),
  ).join(',\n      ');

  pgm.sql(`
    WITH ${targetCte},
    gallery(rank, slot, url) AS (VALUES
      ${galleryValues}
    )
    INSERT INTO product_media
      (location_id, product_id, kind, storage_key, mime_type, bytes, alt, sort_order, available)
    SELECT tp.location_id,
           tp.product_id,
           'image'::product_media_kind,
           g.url,
           '${MIME}',
           (30000 + g.slot * 1000)::bigint,
           '${ALT}',
           g.slot,
           true
    FROM target_products tp
    JOIN gallery g ON g.rank = tp.rank;
  `);

  // 2. One `spin` row on the first seeded product (rank=0). frameKeys are 4 public
  //    URLs the resolver maps to meta.frameUrls; poster_key is one of them.
  //    sort_order 3 so it never collides with the image rows (0,1,2).
  pgm.sql(`
    WITH ${targetCte}
    INSERT INTO product_media
      (location_id, product_id, kind, storage_key, mime_type, bytes, poster_key, alt, sort_order, available, meta)
    SELECT tp.location_id,
           tp.product_id,
           'spin'::product_media_kind,
           '${SPIN_POSTER}',
           '${MIME}',
           120000::bigint,
           '${SPIN_POSTER}',
           '360° spin of the dish',
           3,
           true,
           jsonb_build_object(
             'frameCount', 4,
             'frameKeys', jsonb_build_array(
               '${SPIN_FRAMES[0]}','${SPIN_FRAMES[1]}','${SPIN_FRAMES[2]}','${SPIN_FRAMES[3]}'
             )
           )
    FROM target_products tp
    WHERE tp.rank = 0;
  `);

  // 3. primary_media_id → each seeded product's first image row (kind='image',
  //    lowest sort_order). Only set where still NULL (re-run safe). Scoped to the
  //    demo location so it never touches other tenants.
  pgm.sql(`
    UPDATE products p
    SET primary_media_id = first_img.id
    FROM (
      SELECT DISTINCT ON (pm.product_id)
             pm.product_id, pm.id
      FROM product_media pm
      JOIN locations l ON l.id = pm.location_id
      WHERE l.slug IN ('demo', 'sushi-durres')
        AND pm.kind = 'image'
      ORDER BY pm.product_id, pm.sort_order, pm.created_at
    ) AS first_img
    WHERE p.id = first_img.product_id
      AND p.primary_media_id IS NULL;
  `);
}

export async function down(): Promise<void> {
  // Forward-only seed — matches migrations 046/054. Re-running up() is a no-op
  // once media exists; there is nothing safe to reverse.
}

import type { MigrationBuilder } from 'node-pg-migrate';

// Replace the demo storefront's broken image_key URLs (pointed at a non-existent
// netlify host that served HTML, not JPEGs) with real, permanently-hosted
// Wikimedia Commons sushi photos (CC/PD, hotlink-safe, CSP img-src https:).
// Each product gets a distinct photo by cycling the pool in name order, so
// adjacent cards differ. All curl-verified 200 / image/jpeg.
const POOL = [
  'https://upload.wikimedia.org/wikipedia/commons/thumb/d/d3/Sushi_combination.jpg/960px-Sushi_combination.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/e/ed/Sushi_food_in_Tokyo%2C_Japan.jpg/960px-Sushi_food_in_Tokyo%2C_Japan.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/2/22/Vegan_sushi_plate.jpg/960px-Vegan_sushi_plate.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/0/03/Assorted_Western_sushi_%28%E7%9B%9B%E3%82%8A%E5%90%88%E3%82%8F%E3%81%9B%29.jpg/960px-Assorted_Western_sushi_%28%E7%9B%9B%E3%82%8A%E5%90%88%E3%82%8F%E3%81%9B%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/0/03/Makizushi2.jpg/960px-Makizushi2.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/3/3b/Nigiri_Sushi_%2826478725732%29.jpg/960px-Nigiri_Sushi_%2826478725732%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/a/a1/Nigiri_Sushi_%2826478732232%29.jpg/960px-Nigiri_Sushi_%2826478732232%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/e/e8/Tuna_nigiri_sushi_-_Sushiko_%282648979899%29.jpg/960px-Tuna_nigiri_sushi_-_Sushiko_%282648979899%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/b/bd/Deluxe_sashimi_platter_01.jpg/960px-Deluxe_sashimi_platter_01.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/d/d0/Deluxe_sashimi_platter_02.jpg/960px-Deluxe_sashimi_platter_02.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/b/b7/Sashimi-01.jpg/960px-Sashimi-01.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/3/35/Tuna_Sashimi_%286858891649%29.jpg/960px-Tuna_Sashimi_%286858891649%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/1/16/Tuna_sashimi_by_sunday_driver_at_a_hotel_in_Kyoto.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/d/dc/Salmon_Sushi_and_Sashimi_Platter_-_W_Sushi.jpg/960px-Salmon_Sushi_and_Sashimi_Platter_-_W_Sushi.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/7/7d/Tempura_Roll_sushi_at_Mizuya.jpg/960px-Tempura_Roll_sushi_at_Mizuya.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/5/56/Spider_Roll_and_Tempura_Shrimp_Roll_fancy_maki.jpg/960px-Spider_Roll_and_Tempura_Shrimp_Roll_fancy_maki.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/c/c1/Golden_Maki_Rainbow_Roll_sushi.jpg/960px-Golden_Maki_Rainbow_Roll_sushi.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/e/e0/Golden_Maki_Vegetarian_Dragon_sushi_roll.jpg/960px-Golden_Maki_Vegetarian_Dragon_sushi_roll.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/9/97/Fresh_and_delicious_maki_roll_from_Phengphian_Laogumnerd_Cuisine.jpg/960px-Fresh_and_delicious_maki_roll_from_Phengphian_Laogumnerd_Cuisine.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/3/38/Sushi_mini_set_01.jpg/960px-Sushi_mini_set_01.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/4/43/Sushi_mini_set_04.jpg/960px-Sushi_mini_set_04.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/2/27/Wooden_bridge-shaped_platter_decorated_with_assorted_sushi_rolls_Vikings_Luxury_Dinner_Buffet_26_January_2025_Philippines5.jpg/960px-Wooden_bridge-shaped_platter_decorated_with_assorted_sushi_rolls_Vikings_Luxury_Dinner_Buffet_26_January_2025_Philippines5.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/6/6d/Tekkadon_001.jpg/960px-Tekkadon_001.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/f/f8/Freshippo_sashimi_platter.jpg/960px-Freshippo_sashimi_platter.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/d/d6/Sushi_Sashimi_Medium_Platter_-_Yama-ya_%282570349350%29.jpg/960px-Sushi_Sashimi_Medium_Platter_-_Yama-ya_%282570349350%29.jpg',
  'https://upload.wikimedia.org/wikipedia/commons/thumb/7/7a/Various_sushi%2C_beautiful_October_night_at_midnight.jpg/960px-Various_sushi%2C_beautiful_October_night_at_midnight.jpg',
];

export async function up(pgm: MigrationBuilder): Promise<void> {
  const values = POOL.map((url, i) => `(${i}, '${url}')`).join(',\n    ');
  pgm.sql(`
    WITH pool(idx, url) AS (VALUES
    ${values}
    ),
    ranked AS (
      SELECT p.id,
             (row_number() OVER (ORDER BY p.name, p.id) - 1)::int % (SELECT count(*)::int FROM pool) AS idx
      FROM products p
      JOIN locations l ON p.location_id = l.id
      WHERE l.slug = 'sushi-durres'
    )
    UPDATE products SET image_key = pool.url
    FROM ranked JOIN pool ON pool.idx = ranked.idx
    WHERE products.id = ranked.id;
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  // Non-reversible content change — clear images so cards fall back to the
  // on-brand placeholder rather than restoring the broken netlify URLs.
  pgm.sql(`
    UPDATE products p SET image_key = NULL
    FROM locations l WHERE l.id = p.location_id AND l.slug = 'sushi-durres';
  `);
}

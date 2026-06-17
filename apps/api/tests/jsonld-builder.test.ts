import test from 'node:test';
import assert from 'node:assert/strict';
import { buildJsonLd, JsonLdSchema } from '../src/lib/jsonld-builder.js';

test('JSON-LD Builder', async (t) => {
  const dummyData = {
    location: {
      name: 'Test Resto',
      address: '123 Main St',
      public_phone: '+355691234567',
      geo: { lat: 41.3, lng: 19.8 },
      hours: [ { day: 'Monday', open: '09:00', close: '22:00' } ]
    },
    default_locale: 'en',
    supported_locales: ['en', 'sq'],
    currency: { code: 'ALL', minor_unit: 0 },
    categories: [
      {
        available_names: { en: 'Pizza', sq: 'Pica' },
        products: [
          {
            available_names: { en: 'Margherita', sq: 'Margarita' },
            available_descriptions: { en: 'Cheese and tomato', sq: 'Djath e domate' },
            price: 500
          }
        ]
      }
    ]
  };

  await t.test('builds valid JSON-LD structure', () => {
    const result = buildJsonLd('test-slug', dummyData);

    // Zod validation
    const parsed = JsonLdSchema.parse(result);

    assert.equal(parsed['@graph'].length, 2);

    const restaurant = parsed['@graph'].find((g: any) => g['@type'] === 'Restaurant');
    assert.ok(restaurant);
    assert.equal(restaurant.name, 'Test Resto');
    assert.equal(restaurant.telephone, '+355691234567');

    const menu = parsed['@graph'].find((g: any) => g['@type'] === 'Menu');
    assert.ok(menu);
    assert.equal(menu.hasMenuSection.length, 1);
    assert.equal(menu.hasMenuSection[0].name, 'Pizza');
    assert.equal(menu.hasMenuSection[0].hasMenuItem[0].name, 'Margherita');
    assert.equal(menu.hasMenuSection[0].hasMenuItem[0].offers.price, '500');
  });

  await t.test('cross-tenant isolation: two locations produce independent @graph', () => {
    const locationB = {
      ...dummyData,
      location: { ...dummyData.location, name: 'Other Place', public_phone: '+355697654321' },
      categories: [{ available_names: { en: 'Burgers' }, products: [{ available_names: { en: 'Cheeseburger' }, price: 800 }] }],
    };
    const a = buildJsonLd('slug-a', dummyData) as any;
    const b = buildJsonLd('slug-b', locationB) as any;

    const aRestaurant = a['@graph'].find((g: any) => g['@type'] === 'Restaurant');
    const bRestaurant = b['@graph'].find((g: any) => g['@type'] === 'Restaurant');
    assert.equal(aRestaurant.name, 'Test Resto');
    assert.equal(bRestaurant.name, 'Other Place');
    assert.notEqual(aRestaurant.telephone, bRestaurant.telephone);

    const aMenu = a['@graph'].find((g: any) => g['@type'] === 'Menu') as any;
    const bMenu = b['@graph'].find((g: any) => g['@type'] === 'Menu') as any;
    assert.equal(aMenu.hasMenuSection[0].name, 'Pizza');
    assert.equal(bMenu.hasMenuSection[0].name, 'Burgers');
  });

  await t.test('handles location with no phone or geo gracefully', () => {
    const sparse = {
      ...dummyData,
      location: { name: 'Minimal Place', address: null, public_phone: null, geo: null, hours: [] },
    };
    const result = buildJsonLd('sparse', sparse) as any;
    const parsed = JsonLdSchema.parse(result);
    const restaurant = parsed['@graph'].find((g: any) => g['@type'] === 'Restaurant');
    assert.equal(restaurant.name, 'Minimal Place');
    assert.ok(!restaurant.telephone);
    assert.ok(!restaurant.geo);
  });
});

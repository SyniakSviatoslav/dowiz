// @ts-nocheck
import { z } from 'zod';

export const JsonLdSchema = z.object({
  '@context': z.literal('https://schema.org'),
  '@graph': z.array(z.any())
});

export function buildJsonLd(slug: string, data: any, host: string = 'https://dowiz.org') {
  const url = `${host}/s/${slug}`;
  
  const restaurant: any = {
    '@type': 'Restaurant',
    '@id': `${url}#restaurant`,
    'name': data.location.name,
    'url': url,
    'acceptsReservations': false,
    'hasMenu': { '@id': `${url}#menu` }
  };

  if (data.location.address) {
    restaurant.address = {
      '@type': 'PostalAddress',
      'streetAddress': data.location.address,
      'addressCountry': 'AL' // Adjust based on env/tenant if needed
    };
  }

  if (data.location.public_phone) {
    restaurant.telephone = data.location.public_phone;
  }

  if (data.location.geo) {
    restaurant.geo = {
      '@type': 'GeoCoordinates',
      'latitude': data.location.geo.lat,
      'longitude': data.location.geo.lng
    };
  }

  if (data.location.hours && Array.isArray(data.location.hours)) {
    restaurant.openingHoursSpecification = data.location.hours.map((h: any) => ({
      '@type': 'OpeningHoursSpecification',
      'dayOfWeek': h.day,
      'opens': h.open,
      'closes': h.close
    }));
  }

  const menu: any = {
    '@type': 'Menu',
    '@id': `${url}#menu`,
    'name': 'Menu',
    'inLanguage': data.supported_locales,
    'hasMenuSection': data.categories.map((cat: any) => ({
      '@type': 'MenuSection',
      'name': cat.available_names[data.default_locale],
      'hasMenuItem': cat.products.map((prod: any) => {
        const item: any = {
          '@type': 'MenuItem',
          'name': prod.available_names[data.default_locale],
          'offers': {
            '@type': 'Offer',
            'price': prod.price.toString(),
            'priceCurrency': data.currency.code
          }
        };
        if (prod.available_descriptions && prod.available_descriptions[data.default_locale]) {
          item.description = prod.available_descriptions[data.default_locale];
        }
        if (prod.image_key) {
          item.image = `https://cdn.dowiz.org/${prod.image_key}`; // Placeholder for CDN domain
        }
        return item;
      })
    }))
  };

  return {
    '@context': 'https://schema.org',
    '@graph': [restaurant, menu]
  };
}

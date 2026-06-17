import { z } from 'zod';

interface HourEntry {
  day: string;
  open: string;
  close: string;
}

interface ProductData {
  available_names: Record<string, string>;
  available_descriptions?: Record<string, string>;
  price: number;
}

interface CategoryData {
  available_names: Record<string, string>;
  products: ProductData[];
}

interface LocationData {
  name: string;
  address?: string | null;
  public_phone?: string | null;
  geo?: { lat: number; lng: number } | null;
  hours?: HourEntry[] | Record<string, { open: string; close: string }[]> | null;
}

interface MenuData {
  location: LocationData;
  default_locale: string;
  supported_locales?: string[];
  currency: { code: string; minor_unit: number };
  categories: CategoryData[];
}

function getName(names: Record<string, string>, locale: string): string {
  return names[locale] || names['en'] || Object.values(names)[0] || '';
}

function buildHoursSpec(hours: HourEntry[]): object[] {
  return hours.map(h => ({
    '@type': 'OpeningHoursSpecification',
    dayOfWeek: h.day,
    opens: h.open,
    closes: h.close,
  }));
}

export function buildJsonLd(slug: string, data: MenuData): object {
  const loc = data.location;
  const locale = data.default_locale;

  const restaurant: Record<string, unknown> = {
    '@type': 'Restaurant',
    name: loc.name,
  };
  if (loc.public_phone) restaurant.telephone = loc.public_phone;
  if (loc.address) {
    restaurant.address = { '@type': 'PostalAddress', streetAddress: loc.address };
  }
  if (loc.geo) {
    restaurant.geo = { '@type': 'GeoCoordinates', latitude: loc.geo.lat, longitude: loc.geo.lng };
  }
  if (Array.isArray(loc.hours) && loc.hours.length > 0) {
    restaurant.openingHoursSpecification = buildHoursSpec(loc.hours as HourEntry[]);
  }

  const hasMenuSection = (data.categories || []).map(cat => ({
    '@type': 'MenuSection',
    name: getName(cat.available_names, locale),
    hasMenuItem: (cat.products || []).map(prod => {
      const item: Record<string, unknown> = {
        '@type': 'MenuItem',
        name: getName(prod.available_names, locale),
        offers: {
          '@type': 'Offer',
          price: String(prod.price),
          priceCurrency: data.currency.code,
        },
      };
      if (prod.available_descriptions) {
        const desc = getName(prod.available_descriptions, locale);
        if (desc) item.description = desc;
      }
      return item;
    }),
  }));

  const menu: Record<string, unknown> = {
    '@type': 'Menu',
    name: `${loc.name} Menu`,
    hasMenuSection,
  };

  return {
    '@context': 'https://schema.org',
    '@graph': [restaurant, menu],
  };
}

export const JsonLdSchema = z.object({
  '@context': z.string(),
  '@graph': z.array(
    z.object({
      '@type': z.string(),
    }).passthrough(),
  ),
});

// @ts-nocheck

export function buildJsonLd(slug: string, data: any, host: string = 'https://dowiz.org', extras?: { ratingValue?: number; reviewCount?: number; deliveryRadiusKm?: number }) {
  const url = `${host}/s/${slug}`;

  const restaurant: any = {
    '@type': 'Restaurant',
    '@id': `${url}#restaurant`,
    'name': data.location.name,
    'url': url,
    'acceptsReservations': false,
    'hasMenu': { '@id': `${url}#menu` },
    'currenciesAccepted': data.currency?.code || 'ALL',
    'paymentAccepted': 'Cash',
  };

  if (data.location.seo_description) {
    restaurant.description = data.location.seo_description;
  }

  const prices = data.categories?.flatMap((c: any) => c.products?.map((p: any) => p.price) || []) || [];
  if (prices.length > 0) {
    const min = Math.min(...prices);
    const max = Math.max(...prices);
    restaurant.priceRange = `${min}–${max} ALL`;
  }

  if (data.categories?.[0]?.products?.[0]?.available_names) {
    const firstProdName = data.categories[0].products[0].available_names[data.default_locale];
    if (firstProdName) {
      restaurant.servesCuisine = [firstProdName.split(' ').pop() || 'Albanian'];
    }
  }

  if (data.location.address) {
    restaurant.address = {
      '@type': 'PostalAddress',
      'streetAddress': data.location.address,
      'addressCountry': 'AL',
    };
  }
  if (data.location.public_phone) {
    restaurant.telephone = data.location.public_phone;
  }
  if (data.location.geo) {
    restaurant.geo = {
      '@type': 'GeoCoordinates',
      'latitude': data.location.geo.lat,
      'longitude': data.location.geo.lng,
    };
    const radius = extras?.deliveryRadiusKm || 5;
    restaurant.areaServed = {
      '@type': 'GeoCircle',
      'geoMidpoint': {
        '@type': 'GeoCoordinates',
        'latitude': data.location.geo.lat,
        'longitude': data.location.geo.lng,
      },
      'geoRadius': `${radius * 1000}`,
    };
  }

  if (data.location.hours && Array.isArray(data.location.hours)) {
    restaurant.openingHoursSpecification = data.location.hours.map((h: any) => ({
      '@type': 'OpeningHoursSpecification',
      'dayOfWeek': h.day,
      'opens': h.open,
      'closes': h.close,
    }));
  }

  const rc = extras?.reviewCount || 0;
  if (rc > 0 && extras?.ratingValue != null) {
    restaurant.aggregateRating = {
      '@type': 'AggregateRating',
      'ratingValue': extras.ratingValue.toFixed(1),
      'reviewCount': rc,
      'bestRating': '5',
    };
  }

  const menu: any = {
    '@type': 'Menu',
    '@id': `${url}#menu`,
    'name': 'Menu',
    'inLanguage': data.supported_locales,
    'hasMenuSection': (data.categories || []).map((cat: any) => ({
      '@type': 'MenuSection',
      'name': cat.available_names?.[data.default_locale] || Object.values(cat.available_names || {})[0] || 'Menu',
      'hasMenuItem': (cat.products || []).map((prod: any) => {
        const item: any = {
          '@type': 'MenuItem',
          'name': prod.available_names?.[data.default_locale] || Object.values(prod.available_names || {})[0] || '',
          'offers': {
            '@type': 'Offer',
            'price': (prod.price / Math.pow(10, data.currency?.minor_unit || 0)).toFixed(data.currency?.minor_unit || 0),
            'priceCurrency': data.currency?.code || 'ALL',
            'availability': prod.available ? 'https://schema.org/InStock' : 'https://schema.org/OutOfStock',
          },
        };
        if (prod.available_descriptions?.[data.default_locale]) {
          item.description = prod.available_descriptions[data.default_locale];
        }
        if (prod.image_key) {
          item.image = `${host}/images/${prod.image_key}`;
        }
        if (prod.attributes?.calories) {
          item.nutrition = {
            '@type': 'NutritionInformation',
            'calories': `${prod.attributes.calories} kcal`,
          };
        }
        return item;
      }),
    })),
  };

  const breadcrumb: any = {
    '@type': 'BreadcrumbList',
    '@id': `${url}#breadcrumb`,
    'itemListElement': [
      { '@type': 'ListItem', 'position': 1, 'name': 'Home', 'item': host },
      { '@type': 'ListItem', 'position': 2, 'name': data.location.name, 'item': url },
    ],
  };

  const faqItems: { question: string; answer: string }[] = [];

  if (data.location.address) {
    faqItems.push({ question: `Where is ${data.location.name} located?`, answer: data.location.address });
  }

  if (data.location.public_phone) {
    faqItems.push({ question: `What is the phone number for ${data.location.name}?`, answer: data.location.public_phone });
  }

  if (data.location.hours && Array.isArray(data.location.hours) && data.location.hours.length > 0) {
    const hourSummary = data.location.hours
      .filter((h: any) => h.day && h.open && h.close)
      .map((h: any) => `${h.day}: ${h.open}–${h.close}`)
      .join(', ');
    if (hourSummary) {
      faqItems.push({ question: `What are the opening hours for ${data.location.name}?`, answer: hourSummary });
    }
  }

  const radius = extras?.deliveryRadiusKm || 5;
  faqItems.push({
    question: `Does ${data.location.name} deliver?`,
    answer: `Yes, ${data.location.name} delivers within ${radius} km of their location.`,
  });

  faqItems.push({
    question: `What payment methods does ${data.location.name} accept?`,
    answer: `${data.location.name} accepts cash payment on delivery.`,
  });

  const faq: any = {
    '@type': 'FAQPage',
    '@id': `${url}#faq`,
    'mainEntity': faqItems.map((item, i) => ({
      '@type': 'Question',
      'name': item.question,
      'acceptedAnswer': {
        '@type': 'Answer',
        'text': item.answer,
      },
    })),
  };

  return {
    '@context': 'https://schema.org',
    '@graph': [restaurant, menu, breadcrumb, faq],
  };
}

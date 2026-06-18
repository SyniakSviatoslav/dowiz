const now = new Date();
const ago = (m: number) => new Date(now.getTime() - m * 60000).toISOString();

const PRODUCTS = [
  { id: 'p1',  categoryId: 'c1', name: 'Dragon Roll',       description: 'Shrimp tempura, avocado, cucumber, eel sauce, tobiko',            price: 850,  available: true,  imageUrl: null, allergens: ['gluten','seafood','eggs'],   calories: 420, sortOrder: 1,  createdAt: ago(43200) },
  { id: 'p2',  categoryId: 'c1', name: 'Spicy Tuna Roll',    description: 'Fresh tuna, spicy mayo, cucumber, sesame seeds',                  price: 780,  available: true,  imageUrl: null, allergens: ['gluten','seafood'],          calories: 380, sortOrder: 2,  createdAt: ago(43200) },
  { id: 'p3',  categoryId: 'c1', name: 'Rainbow Roll',       description: 'California roll topped with tuna, salmon, yellowtail, avocado',   price: 920,  available: false, imageUrl: null, allergens: ['gluten','seafood','eggs'],   calories: 460, sortOrder: 3,  createdAt: ago(43200) },
  { id: 'p4',  categoryId: 'c1', name: 'Philadelphia Roll',  description: 'Salmon, cream cheese, cucumber, sesame',                          price: 760,  available: true,  imageUrl: null, allergens: ['gluten','seafood','milk'],   calories: 400, sortOrder: 4,  createdAt: ago(43200) },
  { id: 'p5',  categoryId: 'c1', name: 'Volcano Roll',       description: 'Crab, spicy scallop, masago, green onion',                        price: 890,  available: true,  imageUrl: null, allergens: ['gluten','seafood','eggs'],   calories: 440, sortOrder: 5,  createdAt: ago(43200) },
  { id: 'p6',  categoryId: 'c1', name: 'Vegetable Maki',     description: 'Avocado, cucumber, pickled daikon, sesame',                       price: 560,  available: true,  imageUrl: null, allergens: ['gluten'],                    calories: 280, sortOrder: 6,  createdAt: ago(43200) },
  { id: 'p7',  categoryId: 'c1', name: 'Salmon Avocado',     description: 'Norwegian salmon, ripe avocado, light cream sauce',               price: 720,  available: true,  imageUrl: null, allergens: ['gluten','seafood','milk'],   calories: 360, sortOrder: 7,  createdAt: ago(43200) },
  { id: 'p8',  categoryId: 'c1', name: 'Tempura Prawn',      description: 'Tiger prawn tempura, spicy mayo, cucumber, tobiko',               price: 840,  available: true,  imageUrl: null, allergens: ['gluten','seafood','eggs'],   calories: 430, sortOrder: 8,  createdAt: ago(43200) },
  { id: 'p9',  categoryId: 'c2', name: 'Salmon Nigiri x2',   description: 'Hand-pressed sushi rice with fresh Atlantic salmon',              price: 420,  available: true,  imageUrl: null, allergens: ['seafood'],                   calories: 180, sortOrder: 1,  createdAt: ago(43200) },
  { id: 'p10', categoryId: 'c2', name: 'Tuna Nigiri x2',     description: 'Hand-pressed sushi rice with bluefin tuna',                      price: 480,  available: true,  imageUrl: null, allergens: ['seafood'],                   calories: 170, sortOrder: 2,  createdAt: ago(43200) },
  { id: 'p11', categoryId: 'c2', name: 'Salmon Sashimi x5',  description: '5 slices of premium Norwegian salmon, wasabi, pickled ginger',   price: 680,  available: true,  imageUrl: null, allergens: ['seafood'],                   calories: 220, sortOrder: 3,  createdAt: ago(43200) },
  { id: 'p12', categoryId: 'c2', name: 'Tuna Sashimi x5',    description: '5 slices of premium bluefin tuna',                               price: 760,  available: true,  imageUrl: null, allergens: ['seafood'],                   calories: 200, sortOrder: 4,  createdAt: ago(43200) },
  { id: 'p13', categoryId: 'c2', name: 'Sashimi Platter',    description: '10 pieces: salmon, tuna, yellowtail, octopus, sweet shrimp',     price: 1480, available: true,  imageUrl: null, allergens: ['seafood'],                   calories: 380, sortOrder: 5,  createdAt: ago(43200) },
  { id: 'p14', categoryId: 'c2', name: 'Unagi Nigiri x2',    description: 'Grilled freshwater eel, teriyaki glaze, sesame',                 price: 540,  available: true,  imageUrl: null, allergens: ['seafood','gluten'],           calories: 210, sortOrder: 6,  createdAt: ago(43200) },
  { id: 'p15', categoryId: 'c3', name: 'Chicken Ramen',      description: 'Rich tonkotsu broth, chashu chicken, soft egg, nori, green onion',price: 680, available: true,  imageUrl: null, allergens: ['gluten','eggs','milk'],       calories: 580, sortOrder: 1,  createdAt: ago(43200) },
  { id: 'p16', categoryId: 'c3', name: 'Shrimp Gyoza x6',    description: 'Pan-fried prawn and ginger dumplings, ponzu dipping sauce',      price: 480,  available: true,  imageUrl: null, allergens: ['gluten','seafood'],          calories: 310, sortOrder: 2,  createdAt: ago(43200) },
  { id: 'p17', categoryId: 'c3', name: 'Yakitori Skewers x4',description: 'Grilled chicken thigh skewers, tare sauce, spring onion',       price: 560,  available: true,  imageUrl: null, allergens: ['gluten'],                    calories: 360, sortOrder: 3,  createdAt: ago(43200) },
  { id: 'p18', categoryId: 'c3', name: 'Edamame',            description: 'Steamed young soybeans with sea salt',                           price: 280,  available: true,  imageUrl: null, allergens: [],                             calories: 160, sortOrder: 4,  createdAt: ago(43200) },
  { id: 'p19', categoryId: 'c3', name: 'Chicken Teriyaki',   description: 'Grilled chicken, homemade teriyaki glaze, steamed rice, salad',  price: 720,  available: true,  imageUrl: null, allergens: ['gluten'],                    calories: 540, sortOrder: 5,  createdAt: ago(43200) },
  { id: 'p20', categoryId: 'c4', name: 'Miso Soup',          description: 'Traditional dashi broth, silken tofu, wakame, green onion',     price: 240,  available: true,  imageUrl: null, allergens: ['gluten'],                    calories: 80,  sortOrder: 1,  createdAt: ago(43200) },
  { id: 'p21', categoryId: 'c4', name: 'Tom Yum Soup',       description: 'Lemongrass broth, shrimp, mushrooms, chili, kaffir lime',       price: 480,  available: true,  imageUrl: null, allergens: ['seafood'],                   calories: 180, sortOrder: 2,  createdAt: ago(43200) },
  { id: 'p22', categoryId: 'c4', name: 'Seaweed Salad',      description: 'Wakame, cucumber, sesame oil, rice vinegar, chili flakes',      price: 320,  available: true,  imageUrl: null, allergens: ['gluten'],                    calories: 120, sortOrder: 3,  createdAt: ago(43200) },
  { id: 'p23', categoryId: 'c4', name: 'Kaiso Salad',        description: 'Mixed sea vegetables, ginger dressing, toasted sesame',         price: 360,  available: true,  imageUrl: null, allergens: ['gluten','seafood'],          calories: 140, sortOrder: 4,  createdAt: ago(43200) },
  { id: 'p24', categoryId: 'c5', name: 'Sencha Green Tea',   description: 'Premium Japanese green tea, hot or iced',                       price: 180,  available: true,  imageUrl: null, allergens: [],                             calories: 0,   sortOrder: 1,  createdAt: ago(43200) },
  { id: 'p25', categoryId: 'c5', name: 'Yuzu Lemonade',      description: 'Fresh yuzu citrus, sparkling water, honey, mint',               price: 240,  available: true,  imageUrl: null, allergens: [],                             calories: 90,  sortOrder: 2,  createdAt: ago(43200) },
  { id: 'p26', categoryId: 'c5', name: 'Matcha Latte',       description: 'Ceremonial grade matcha, oat milk, light honey',                price: 280,  available: true,  imageUrl: null, allergens: ['milk'],                      calories: 120, sortOrder: 3,  createdAt: ago(43200) },
  { id: 'p27', categoryId: 'c5', name: 'Sake (180ml)',       description: 'Junmai Daiginjo - floral, light, fruity finish',                price: 480,  available: true,  imageUrl: null, allergens: ['gluten'],                    calories: 220, sortOrder: 4,  createdAt: ago(43200) },
  { id: 'p28', categoryId: 'c5', name: 'San Pellegrino 0.5l',description: 'Sparkling mineral water',                                       price: 120,  available: true,  imageUrl: null, allergens: [],                             calories: 0,   sortOrder: 5,  createdAt: ago(43200) },
  { id: 'p29', categoryId: 'c6', name: 'Mochi Ice Cream x3', description: 'Strawberry, matcha, mango - rice cake filled with ice cream',   price: 380,  available: true,  imageUrl: null, allergens: ['milk','gluten'],              calories: 280, sortOrder: 1,  createdAt: ago(43200) },
  { id: 'p30', categoryId: 'c6', name: 'Matcha Cheesecake',  description: 'Baked cheesecake with ceremonial matcha, white chocolate glaze',price: 340, available: true,  imageUrl: null, allergens: ['milk','eggs','gluten'],       calories: 360, sortOrder: 2,  createdAt: ago(43200) },
  { id: 'p31', categoryId: 'c6', name: 'Taiyaki',            description: 'Fish-shaped waffle with red bean or custard filling, warm',     price: 260,  available: true,  imageUrl: null, allergens: ['gluten','eggs','milk'],       calories: 240, sortOrder: 3,  createdAt: ago(43200) },
];

const CATEGORIES = [
  { id: 'c1', name: 'Sushi Rolls',        sortOrder: 1, productCount: 8, imageKey: null, createdAt: ago(43200) },
  { id: 'c2', name: 'Nigiri & Sashimi',   sortOrder: 2, productCount: 6, imageKey: null, createdAt: ago(43200) },
  { id: 'c3', name: 'Hot Dishes',         sortOrder: 3, productCount: 5, imageKey: null, createdAt: ago(43200) },
  { id: 'c4', name: 'Soups & Salads',     sortOrder: 4, productCount: 4, imageKey: null, createdAt: ago(43200) },
  { id: 'c5', name: 'Drinks',             sortOrder: 5, productCount: 5, imageKey: null, createdAt: ago(43200) },
  { id: 'c6', name: 'Desserts',           sortOrder: 6, productCount: 3, imageKey: null, createdAt: ago(43200) },
];

const LOCATION = {
  id: 'dubin-sushi-tirana', name: 'Dubin & Sushi', slug: 'dubin-sushi',
  phone: '+355 69 234 567', address: 'Rruga Ismail Qemali 8, Tiranë',
  status: 'open' as const, closesAt: '23:00', rating: 4.9, reviewCount: 218,
  deliveryEta: '30-45 min', deliveryFee: 200, minOrder: 800, currencyCode: 'ALL',
  menuVersion: 12, heroImageUrl: null, logoUrl: null,
  supportedLocales: ['sq', 'en'], defaultLocale: 'sq', lat: 41.3275, lng: 19.8187,
};

const SN_LOCATION = {
  id: 'dubin-sushi-tirana', name: 'Dubin & Sushi', slug: 'dubin-sushi',
  phone: '+355 69 234 567', address: 'Rruga Ismail Qemali 8, Tiranë',
  status: 'open', currency_code: 'ALL', delivery_fee_flat: 200,
  min_order_value: 800, free_delivery_threshold: 3000, delivery_radius_km: 5,
  tax_rate: 0, default_locale: 'sq', supported_locales: ['sq', 'en'],
};

const FALLBACK = { phone: '+355 69 234 567', showPhoneOnError: true, showPhoneOnOffline: true };

const COURIERS = [
  { id: 'cu1', name: 'Ardit Kelmendi',  maskedPhone: '+355 69 555 111', maskedEmail: 'a****i@email.com', status: 'active' as const, role: 'courier' as const, onlineStatus: 'busy' as const,    ordersToday: 11, rating: 4.9, lastLoginAt: ago(10),   createdAt: ago(86400) },
  { id: 'cu2', name: 'Blerim Hoxhaj',   maskedPhone: '+355 69 *** ***', maskedEmail: 'b****j@email.com', status: 'active' as const, role: 'courier' as const, onlineStatus: 'online' as const,  ordersToday: 8,  rating: 4.8, lastLoginAt: ago(30),   createdAt: ago(86400) },
  { id: 'cu3', name: 'Genci Dervishi',  maskedPhone: '+355 69 *** ***', maskedEmail: 'g****i@email.com', status: 'active' as const, role: 'courier' as const, onlineStatus: 'offline' as const, ordersToday: 7,  rating: 4.7, lastLoginAt: ago(180),  createdAt: ago(172800) },
];

const CUSTOMERS = [
  { id: 'cust1', name: 'Sara Mancini',     phone: '+355 69 876 543', orders: 18, ltv: 32400, lastOrder: 'today',         aliases: [] as string[] },
  { id: 'cust2', name: 'Alina Popa',       phone: '+355 69 432 187', orders: 11, ltv: 19800, lastOrder: 'yesterday',     aliases: [] as string[] },
  { id: 'cust3', name: 'Bled Gjoni',       phone: '+355 69 321 654', orders: 27, ltv: 51300, lastOrder: 'today',         aliases: ['+355 69 321 655'] },
  { id: 'cust4', name: 'Dorina Shehu',     phone: '+355 69 111 999', orders: 4,  ltv: 6800,  lastOrder: '3 weeks ago',   aliases: [] as string[] },
  { id: 'cust5', name: 'Erion Berisha',    phone: '+355 69 777 333', orders: 14, ltv: 24200, lastOrder: '2 days ago',    aliases: [] as string[] },
  { id: 'cust6', name: 'Fatbardha Koci',   phone: '+355 69 444 222', orders: 1,  ltv: 2090,  lastOrder: '2 months ago',  aliases: [] as string[] },
  { id: 'cust7', name: 'Gjergji Marku',    phone: '+355 69 555 888', orders: 32, ltv: 61400, lastOrder: 'today',         aliases: [] as string[] },
];

const PROMOTIONS = [
  { id: 'pr1', code: 'SUSHI15',   type: 'percentage' as const,  value: 15,   description: '15% off first order',                          active: true,  uses: 58,  maxUses: 200 },
  { id: 'pr2', code: 'ROLL2GET1', type: 'buy_x_get_y' as const, value: null, description: 'Buy 2 rolls - get 3rd free',                   active: true,  uses: 31,  maxUses: null },
  { id: 'pr3', code: 'LUNCH',     type: 'happy_hour' as const,  value: 20,   description: 'Mon-Fri 12:00-14:00 - 20% off hot dishes',      active: false, uses: 124, maxUses: null },
  { id: 'pr4', code: 'COMBO1',    type: 'combo' as const,       value: null, description: 'Dragon Roll + Miso Soup + Green Tea - 1,800 ALL', active: true, uses: 44,  maxUses: null },
];

const SIGNALS = [
  { id: 's1', customerId: 'cust1', kind: 'velocity_spike' as const, severity: 'high' as const,   raisedAt: ago(15), acknowledgedAt: null,    dismissedAt: null,    customerNameMasked: 'S*** M******', customerPhoneMasked: '+355 69 *** ***' },
  { id: 's2', customerId: 'cust2', kind: 'address_mismatch' as const,severity: 'medium' as const, raisedAt: ago(45), acknowledgedAt: ago(30), dismissedAt: null,    customerNameMasked: 'A*** P***',    customerPhoneMasked: '+355 69 *** ***' },
];

const ALERTS = [
  { id: 'a1', kind: 'Stop-list',        severity: 'warning' as const, message: 'Rainbow Roll has been on stop-list for 3 hours. Avg daily revenue: 1,840 ALL.', createdAt: ago(180), dwellSeconds: 10800, acknowledgedAt: null },
  { id: 'a2', kind: 'Scheduled order',  severity: 'info' as const,    message: 'Scheduled order #2296 activates at 21:45. Assign a courier in advance.',             createdAt: ago(60),  dwellSeconds: null,   acknowledgedAt: null },
];

const STAFF = [
  { id: 'st1', name: 'Kenji Tanaka',   email: 'kenji@dubinsushi.al',  role: 'owner' as const,   status: 'active' as const, lastLoginAt: ago(120) },
  { id: 'st2', name: 'Mira Hoxha',     email: 'mira@dubinsushi.al',   role: 'manager' as const, status: 'active' as const, lastLoginAt: ago(480) },
  { id: 'st3', name: 'Dritan Prifti',  email: 'dritan@dubinsushi.al', role: 'staff' as const,   status: 'active' as const, lastLoginAt: ago(1440) },
];

const INVENTORY_ITEMS = [
  { id: 'inv1', name: 'Salmon fillet',   unit: 'kg',     stock: 4.5, minStock: 2,  status: 'ok' as const },
  { id: 'inv2', name: 'Tuna fillet',     unit: 'kg',     stock: 3.2, minStock: 2,  status: 'ok' as const },
  { id: 'inv3', name: 'Rice (sushi)',    unit: 'kg',     stock: 12,  minStock: 5,  status: 'ok' as const },
  { id: 'inv4', name: 'Nori sheets',     unit: 'sheets', stock: 80,  minStock: 50, status: 'ok' as const },
  { id: 'inv5', name: 'Mozzarella',      unit: 'kg',     stock: 1.5, minStock: 2,  status: 'low' as const },
  { id: 'inv6', name: 'Avocado',         unit: 'pcs',    stock: 15,  minStock: 10, status: 'ok' as const },
];

const PAYOUTS = [
  { id: 'po1', courierName: 'Ardit Kelmendi', amount: 12400, period: '2026-05-26 to 2026-06-01', status: 'paid' as const,    paidAt: ago(1440) },
  { id: 'po2', courierName: 'Blerim Hoxhaj',  amount: 9800,  period: '2026-05-26 to 2026-06-01', status: 'pending' as const, paidAt: null },
  { id: 'po3', courierName: 'Genci Dervishi', amount: 7600,  period: '2026-05-19 to 2026-05-25', status: 'paid' as const,    paidAt: ago(10080) },
];

const COURIER_ASSIGNMENTS = [
  { id: 'as1', orderId: '2301', status: 'picked_up' as const, assignedAt: ago(30), acceptedAt: ago(28), pickedUpAt: ago(15), deliveredAt: null, cashCollected: false, cashAmount: 1806 },
  { id: 'as2', orderId: '2297', status: 'assigned' as const,  assignedAt: ago(5),  acceptedAt: null,    pickedUpAt: null,    deliveredAt: null, cashCollected: false, cashAmount: null },
];

const COURIER_EARNINGS = {
  weekly: [
    { day: 'Mon', amount: 4200 }, { day: 'Tue', amount: 3800 }, { day: 'Wed', amount: 5100 },
    { day: 'Thu', amount: 0 },    { day: 'Fri', amount: 4700 }, { day: 'Sat', amount: 6200 },
    { day: 'Sun', amount: 5400 },
  ],
  total: 29400,
};

const COURIER_HISTORY = [
  { id: 'd1', orderId: '2285', customerName: 'Gjergji Marku', address: 'Rruga Kavajës 45',    total: 2340, distance: 2.1, duration: 24, completedAt: ago(90),  rating: 5 },
  { id: 'd2', orderId: '2278', customerName: 'Bled Gjoni',     address: 'Rruga Durrësit 12',  total: 1850, distance: 1.5, duration: 18, completedAt: ago(180), rating: 4 },
  { id: 'd3', orderId: '2270', customerName: 'Sara Mancini',   address: 'Rruga Barrikadave 22',total: 3100, distance: 3.2, duration: 35, completedAt: ago(360), rating: 5 },
];

const COURIER_SHIFTS = [
  { id: 'sh1', date: '2026-06-03', startTime: '11:00', endTime: '15:00', status: 'active' as const },
  { id: 'sh2', date: '2026-06-02', startTime: '17:00', endTime: '22:00', status: 'completed' as const },
  { id: 'sh3', date: '2026-06-01', startTime: '11:00', endTime: '15:00', status: 'completed' as const },
];

type Handler = (url: string, method: string, body?: unknown) => unknown;

function enrichProduct(p: typeof PRODUCTS[number]) {
  const tastes: Record<string, Record<string, number>> = {
    'p1': { spicy: 2, richness: 2 },
    'p2': { spicy: 3, salty: 2 },
    'p4': { richness: 3, salty: 1 },
    'p9': { richness: 1 },
    'p15': { richness: 3, salty: 2, spicy: 1 },
    'p16': { salty: 2, richness: 1 },
    'p18': { salty: 1 },
    'p20': { salty: 2 },
    'p22': { sour: 2, salty: 1 },
    'p25': { sweet: 2, sour: 3 },
    'p29': { sweet: 3, richness: 2 },
  };
  const kcals: Record<string, number> = {
    'p1': 420, 'p2': 380, 'p3': 460, 'p4': 400, 'p5': 440, 'p6': 280, 'p7': 360, 'p8': 430,
    'p9': 180, 'p10': 170, 'p11': 220, 'p12': 200, 'p13': 380, 'p14': 210,
    'p15': 580, 'p16': 310, 'p17': 360, 'p18': 160, 'p19': 540,
    'p20': 80, 'p21': 180, 'p22': 120, 'p23': 140,
    'p24': 0, 'p25': 90, 'p26': 120, 'p27': 220, 'p28': 0,
    'p29': 280, 'p30': 360, 'p31': 240,
  };
  const protein: Record<string, number> = { 'p1': 24, 'p2': 22, 'p4': 18, 'p9': 16, 'p15': 35, 'p19': 40, 'p29': 4 };
  const fat: Record<string, number> = { 'p1': 18, 'p2': 16, 'p4': 22, 'p15': 28, 'p19': 18, 'p29': 12 };
  const carbs: Record<string, number> = { 'p1': 42, 'p2': 38, 'p4': 36, 'p6': 52, 'p15': 60, 'p29': 40 };
  return {
    ...p,
    isAvailable: p.available,
    tags: p.allergens,
    taste: tastes[p.id],
    kcal: kcals[p.id] ?? null,
    protein: protein[p.id] ?? null,
    fat: fat[p.id] ?? null,
    carbs: carbs[p.id] ?? null,
    allergenStatus: p.allergens.length > 0 ? 'listed' as const : 'none' as const,
  };
}

export const MOCK: Record<string, Handler> = {
  // ── Client ──
  'GET /public/theme/:slug': () => ({
    primaryColor: '#ea4f16',
    fontFamily: 'Inter',
    bgColor: '#121212',
    textColor: '#ffffff',
    logoUrl: null,
    frameAncestors: ['*'],
  }),

  'GET /public/menu/:slug': () => [
    { id: 'c1', name: 'Sushi Rolls', items: PRODUCTS.filter(p => p.categoryId === 'c1').map(enrichProduct) },
    { id: 'c2', name: 'Nigiri & Sashimi', items: PRODUCTS.filter(p => p.categoryId === 'c2').map(enrichProduct) },
    { id: 'c3', name: 'Hot Dishes', items: PRODUCTS.filter(p => p.categoryId === 'c3').map(enrichProduct) },
    { id: 'c4', name: 'Soups & Salads', items: PRODUCTS.filter(p => p.categoryId === 'c4').map(enrichProduct) },
    { id: 'c5', name: 'Drinks', items: PRODUCTS.filter(p => p.categoryId === 'c5').map(enrichProduct) },
    { id: 'c6', name: 'Desserts', items: PRODUCTS.filter(p => p.categoryId === 'c6').map(enrichProduct) },
  ],

  // legacy public/locations pattern (kept for backward compat)
  'GET /public/locations/:slug/menu': () => ({
    location: LOCATION,
    categories: CATEGORIES.map(c => ({ ...c, products: PRODUCTS.filter(p => p.categoryId === c.id) })),
  }),

  // CheckoutPage loads this to resolve locationId + map center; without it
  // locationId stays null and order placement silently no-ops (early return).
  'GET /public/locations/:slug/info': () => ({
    id: LOCATION.id,
    name: LOCATION.name,
    address: LOCATION.address,
    lat: LOCATION.lat,
    lng: LOCATION.lng,
  }),

  'POST /api/orders': () => ({
    id: 'ord_dev001', status: 'PENDING' as const, outcome: 'clean' as const,
  }),

  'GET /api/orders/:id': () => ({
    id: '2301',
    locationId: LOCATION.id,
    status: 'IN_DELIVERY' as const,
    type: 'delivery' as const,
    deliveryAddress: 'Rruga Barrikadave 22, Tiranë',
    subtotal: 1890,
    total: 1806,
    paymentMethod: 'cash' as const,
    paymentOutcome: 'pending' as const,
    createdAt: ago(22),
    timeoutAt: null,
    items: [
      { id: 'oi1', productId: 'p1',  nameSnapshot: 'Dragon Roll',      priceSnapshot: 850, quantity: 1 },
      { id: 'oi2', productId: 'p15', nameSnapshot: 'Chicken Ramen',     priceSnapshot: 680, quantity: 1 },
      { id: 'oi3', productId: 'p24', nameSnapshot: 'Sencha Green Tea',  priceSnapshot: 180, quantity: 2 },
    ],
  }),

  'POST /api/orders/:id/verify-otp': () => ({ ok: true }),
  'POST /api/orders/:id/confirm': () => ({ ok: true }),

  'GET /customer/orders/:id/status': () => ({
    id: '2301',
    status: 'PREPARING',
    createdAt: ago(15),
    elapsedSeconds: 900,
    items: [
      { name: 'Dragon Roll', quantity: 1, price: 850, kcal: 420, protein: 24, fat: 18, carbs: 42 },
      { name: 'Chicken Ramen', quantity: 1, price: 680, kcal: 580, protein: 35, fat: 28, carbs: 60 },
      { name: 'Sencha Green Tea', quantity: 2, price: 180, kcal: 0 },
    ],
    total: 1890,
    kcal_total: 1000,
    protein_mg_total: 59,
    fat_mg_total: 46,
    carb_mg_total: 102,
  }),

  // ── Admin (component-level API paths) ──
  'GET /owner/orders': () => [
    { id: 'o_1', status: 'PENDING', createdAt: ago(5), items: [{ name: 'Dragon Roll', quantity: 2 }, { name: 'Miso Soup', quantity: 1 }], total: 2180, customerName: 'Sara Mancini', customerPhone: '+355 69 876 543', shortId: '#2301', etaMinutes: null, elapsedSeconds: 300, courierName: null, itemCount: 3, itemsSummary: 'Dragon Roll x2, Miso Soup x1' },
    { id: 'o_2', status: 'PREPARING', createdAt: ago(15), items: [{ name: 'Tonkotsu Ramen', quantity: 1 }], total: 680, customerName: 'Alina Popa', customerPhone: '+355 69 432 187', shortId: '#2300', etaMinutes: null, elapsedSeconds: 900, courierName: 'Ardit', itemCount: 1, itemsSummary: 'Tonkotsu Ramen x1' },
    { id: 'o_3', status: 'CONFIRMED', createdAt: ago(8), items: [{ name: 'Sashimi Platter', quantity: 1 }], total: 1480, customerName: 'Bled Gjoni', customerPhone: '+355 69 321 654', shortId: '#2299', etaMinutes: null, elapsedSeconds: 480, courierName: null, itemCount: 1, itemsSummary: 'Sashimi Platter x1' },
    { id: 'o_4', status: 'IN_DELIVERY', createdAt: ago(22), items: [{ name: 'Philadelphia Roll', quantity: 2 }], total: 1520, customerName: 'Dorina Shehu', customerPhone: '+355 69 111 999', shortId: '#2298', etaMinutes: 14, elapsedSeconds: 1320, courierName: 'Ardit', itemCount: 2, itemsSummary: 'Philadelphia Roll x2' },
  ],

  'PATCH /owner/orders/:id/status': () => ({ ok: true }),

  'GET /owner/categories': () => CATEGORIES,
  'POST /owner/categories': (_url: string, _method: string, body?: unknown) => ({
    id: 'cNew', name: (body as { name?: string })?.name || 'New Category', sortOrder: 99, productCount: 0, imageKey: null, createdAt: now.toISOString(),
  }),
  'DELETE /owner/categories/:id': () => ({ ok: true }),
  'GET /owner/menu/categories': () => CATEGORIES,
  'POST /owner/menu/categories': (_url: string, _method: string, body?: unknown) => ({
    id: 'cNew', name: (body as { name?: string })?.name || 'New Category', sortOrder: 99, productCount: 0,
  }),
  'DELETE /owner/menu/categories/:id': () => ({ ok: true }),

  'GET /owner/products': (url: string) => {
    const qs = url.split('?')[1] || '';
    const catId = new URLSearchParams(qs).get('category_id');
    return catId ? PRODUCTS.filter(p => p.categoryId === catId) : PRODUCTS;
  },
  'POST /owner/products': () => ({
    id: 'pNew', categoryId: 'c1', name: 'New Product', description: null, price: 500, available: true, imageUrl: null, allergens: [] as string[], calories: 0, sortOrder: 99, createdAt: now.toISOString(),
  }),
  'PATCH /owner/products/:id': () => ({ ok: true }),
  'DELETE /owner/products/:id': () => ({ ok: true }),
  'GET /owner/menu/products': (url: string) => {
    const qs = url.split('?')[1] || '';
    const catId = new URLSearchParams(qs).get('category_id');
    return catId ? PRODUCTS.filter(p => p.categoryId === catId) : PRODUCTS;
  },
  'POST /owner/menu/products': () => ({
    id: 'pNew', categoryId: 'c1', name: 'New Product', description: null, price: 500, available: true, imageUrl: null, allergens: [] as string[], calories: 0, sortOrder: 99, createdAt: now.toISOString(),
  }),
  'PATCH /owner/menu/products/:id': () => ({ ok: true }),
  'DELETE /owner/menu/products/:id': () => ({ ok: true }),

  // ── Courier (component-level API paths) ──
  'GET /courier/me/assignments': () => ({ assignments: COURIER_ASSIGNMENTS }),

  'GET /courier/me': () => ({
    id: 'cu1', full_name: 'Ardit Kelmendi', masked_email: 'a****i@email.com',
    masked_phone: '+355 69 555 111', last_login_at: ago(10),
  }),

  'GET /courier/orders/:id': () => ({
    id: '2301',
    status: 'IN_DELIVERY',
    restaurant: { name: 'Burger King', address: 'Blloku, Tirana', lat: 41.328, lng: 19.812 },
    customer: { address: 'Rruga e Elbasanit 12', phone: '+355 69 123 4567', instructions: 'Call when near', lat: 41.337, lng: 19.825 },
    total: 1806,
    eta: '8 min',
  }),

  'POST /courier/orders/:id/status': () => ({ ok: true }),
  'POST /courier/shifts/transition': () => ({ ok: true }),

  'GET /courier/me/earnings': () => ({
    today: 5400, week: 29400, month: 98200,
    payouts: [
      { id: 'po1', amount: 12400, status: 'paid', createdAt: ago(1440) },
      { id: 'po2', amount: 9800, status: 'pending', createdAt: ago(2880) },
      { id: 'po3', amount: 11500, status: 'paid', createdAt: ago(10080) },
      { id: 'po4', amount: 8700, status: 'pending', createdAt: ago(20160) },
    ],
  }),

  'GET /courier/me/history': () => ({
    deliveries: [
      { id: 'd1', orderId: '2285', customerName: 'Gjergji Marku', restaurant: 'Dubin & Sushi', address: 'Rruga Kavajës 45', total: 2340, rating: 5, completedAt: ago(90) },
      { id: 'd2', orderId: '2278', customerName: 'Bled Gjoni', restaurant: 'Dubin & Sushi', address: 'Rruga Durrësit 12', total: 1850, rating: 4, completedAt: ago(180) },
      { id: 'd3', orderId: '2270', customerName: 'Sara Mancini', restaurant: 'Dubin & Sushi', address: 'Rruga Barrikadave 22', total: 3100, rating: 5, completedAt: ago(360) },
      { id: 'd4', orderId: '2265', customerName: 'Erion Berisha', restaurant: 'Dubin & Sushi', address: 'Bulevardi Zogu I', total: 1680, rating: 5, completedAt: ago(720) },
      { id: 'd5', orderId: '2260', customerName: 'Dorina Shehu', restaurant: 'Dubin & Sushi', address: 'Rruga Myslym Shyri', total: 2900, rating: 3, completedAt: ago(1440) },
      { id: 'd6', orderId: '2250', customerName: 'Fatbardha Koci', restaurant: 'Dubin & Sushi', address: 'Rruga e Kavajës 88', total: 1200, rating: 4, completedAt: ago(2880) },
    ],
  }),

  'GET /courier/me/shifts': () => ({
    shifts: [
      { id: 'sh1', date: '2026-06-04', startTime: '11:00', endTime: null, status: 'active' },
      { id: 'sh2', date: '2026-06-03', startTime: '11:00', endTime: '15:00', status: 'completed' },
      { id: 'sh3', date: '2026-06-02', startTime: '17:00', endTime: '22:00', status: 'completed' },
      { id: 'sh4', date: '2026-06-01', startTime: '11:00', endTime: '15:00', status: 'completed' },
    ],
  }),

  'POST /courier/me/shifts': () => ({ id: 'shNew', date: '2026-06-04', startTime: '11:00', endTime: null, status: 'active' }),

  'POST /courier/auth/login': () => ({
    token: 'eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.mock_courier_token',
    courier: { id: 'cu1', name: 'Ardit Kelmendi', status: 'offline' },
  }),

  // ── Admin additional (component-level API paths) ──
  'GET /owner/couriers': () => ({ couriers: COURIERS }),
  'GET /owner/analytics': () => ({
    revenue: { today: 87400, trend: '+18%' },
    orders: { today: 63, trend: '+11' },
    avgOrderValue: { value: 1387, trend: '+5%' },
    deliveryTime: { avg: 32, trend: '-8%' },
    chart: [
      { day: 'Mon', revenue: 52000 }, { day: 'Tue', revenue: 61000 },
      { day: 'Wed', revenue: 48000 }, { day: 'Thu', revenue: 71000 },
      { day: 'Fri', revenue: 95000 }, { day: 'Sat', revenue: 110000 },
      { day: 'Sun', revenue: 87400 },
    ],
    topProducts: [
      { name: 'Dragon Roll', orders: 28, revenue: 23800 },
      { name: 'Salmon Sashimi', orders: 22, revenue: 14960 },
    ],
  }),
  'GET /owner/customers': () => ({ customers: CUSTOMERS }),
  'POST /owner/customers/:id/reveal-contact': () => ({ phone: '+355 69 876 543', name: 'Sara Mancini' }),
  'GET /owner/settings': () => SN_LOCATION,
  'PATCH /owner/settings': () => ({ ok: true }),
  'POST /owner/onboarding': () => ({ ok: true, locationId: 'loc_new', slug: 'new-restaurant' }),
  'GET /owner/brand': () => ({ primaryColor: '#ea4f16', bgColor: '#121212', logoUrl: '' }),
  'PUT /owner/brand': () => ({ ok: true }),

  // ── Admin Dashboard (legacy patterns, kept for direct API access) ──
  'GET /api/owner/locations/current/dashboard/snapshot': () => ({
    serverTime: now.toISOString(),
    counts: {
      pending: 1, confirmed: 1, preparing: 1, ready: 1, inDelivery: 1,
      deliveredToday: 63, revenueToday: 87400, revenueTrend: '+18%',
      ordersToday: 63, ordersTrend: '+11',
      activeDeliveries: 1, couriersOnline: 2, avgDeliveryMin: 32,
    },
    activeDeliveries: 1, activeAlertCount: 2, activeSignalCount: 2,
    orders: [
      { id: '2301', shortId: '#2301', status: 'IN_DELIVERY' as const, customerName: 'Sara Mancini',     customerPhone: '+355 69 876 543', itemsSummary: 'Dragon Roll x1, Ramen x1, Tea x2',          itemCount: 4, total: 1806, createdAt: ago(22), elapsedSeconds: 1320, courierName: 'Ardit',  etaMinutes: 14 },
      { id: '2300', shortId: '#2300', status: 'PREPARING' as const,   customerName: 'Alina Popa',       customerPhone: '+355 69 432 187', itemsSummary: 'Sashimi Platter x1, Miso Soup x2',          itemCount: 3, total: 1960, createdAt: ago(15), elapsedSeconds: 900,  courierName: 'Blerim', etaMinutes: null },
      { id: '2299', shortId: '#2299', status: 'CONFIRMED' as const,   customerName: 'Bled Gjoni',       customerPhone: '+355 69 321 654', itemsSummary: 'Philadelphia Roll x2, Edamame x1',          itemCount: 3, total: 1800, createdAt: ago(8),  elapsedSeconds: 480,  courierName: null,     etaMinutes: null },
      { id: '2298', shortId: '#2298', status: 'PENDING' as const,     customerName: 'Dorina Shehu',     customerPhone: '+355 69 111 999', itemsSummary: 'Dragon Roll x1, Rainbow Roll x1',           itemCount: 2, total: 1770, createdAt: ago(2),  elapsedSeconds: 120,  courierName: null,     etaMinutes: null },
      { id: '2297', shortId: '#2297', status: 'READY' as const,       customerName: 'Erion Berisha',    customerPhone: '+355 69 777 333', itemsSummary: 'Salmon Sashimi x5, Sake x1',               itemCount: 2, total: 1160, createdAt: ago(28), elapsedSeconds: 1680, courierName: 'Ardit',  etaMinutes: null },
    ],
  }),

  // ── Admin Orders ──
  'POST /api/owner/locations/current/orders/:id/confirm': () => ({ ok: true }),
  'POST /api/owner/locations/current/orders/:id/prepare': () => ({ ok: true }),
  'POST /api/owner/locations/current/orders/:id/ready': () => ({ ok: true }),
  'POST /api/owner/locations/current/orders/:id/reject': () => ({ ok: true }),

  // ── Admin Menu ──
  'GET /api/owner/locations/current/categories': () => CATEGORIES,
  'POST /api/owner/locations/current/categories': (_url: string, _method: string, body?: unknown) => ({
    id: 'cNew', name: (body as { name?: string })?.name || 'New Category', sortOrder: 99, productCount: 0, imageKey: null, createdAt: now.toISOString(),
  }),
  'DELETE /api/owner/locations/current/categories/:id': () => ({ ok: true }),
  'GET /api/owner/locations/current/products': (url: string) => {
    const qs = url.split('?')[1] || '';
    const catId = new URLSearchParams(qs).get('category_id');
    return catId ? PRODUCTS.filter(p => p.categoryId === catId) : PRODUCTS;
  },
  'POST /api/owner/locations/current/products': () => ({
    id: 'pNew', categoryId: 'c1', name: 'New Product', description: null, price: 500, available: true, imageUrl: null, allergens: [] as string[], calories: 0, sortOrder: 99, createdAt: now.toISOString(),
  }),
  'PATCH /api/owner/locations/current/products/:id': () => ({ ok: true }),

  // ── Admin Couriers ──
  'GET /api/owner/locations/current/couriers': () => ({ couriers: COURIERS }),
  'PATCH /api/owner/locations/current/couriers/:id': () => ({ ok: true }),

  // ── Admin Signals ──
  'GET /api/owner/locations/current/signals': () => ({ signals: SIGNALS }),
  'POST /api/owner/locations/current/signals/:id/acknowledge': () => ({ ok: true }),
  'POST /api/owner/locations/current/signals/:id/dismiss': () => ({ ok: true }),

  // ── Admin Alerts ──
  'GET /api/owner/locations/current/alerts': () => ({ alerts: ALERTS }),
  'POST /api/owner/locations/current/alerts/:id/acknowledge': () => ({ ok: true }),
  'POST /api/owner/locations/current/alerts/acknowledge-all': () => ({ ok: true }),

  // ── Admin Branding ──
  'GET /api/owner/locations/current/theme': () => ({
    theme: { primaryColor: '#ea4f16', fontFamily: 'Inter', bgColor: '#121212', textColor: '#ffffff', logoUrl: null, frameAncestors: ['*'] },
  }),
  'PUT /api/owner/locations/current/theme': () => ({ ok: true }),

  // ── Admin Settings ──
  'GET /api/owner/locations/current': () => SN_LOCATION,
  'PATCH /api/owner/locations/current': () => ({ ok: true }),
  'GET /api/owner/locations/current/settings/fallback': () => FALLBACK,
  'PUT /api/owner/locations/current/settings/fallback': () => ({ ok: true }),
  'GET /api/owner/locations/current/degradation': () => ({ db: 'healthy' as const, redis: 'healthy' as const, workers: 'healthy' as const }),

  // ── Admin CRM ──
  'GET /api/owner/locations/current/customers': () => ({ customers: CUSTOMERS }),

  // ── Admin Promotions ──
  'GET /api/owner/locations/current/promotions': () => ({ promotions: PROMOTIONS }),
  'POST /api/owner/locations/current/promotions': (_url: string, _method: string, body?: unknown) => {
    const b = (body as Record<string, unknown>) || {};
    return { id: 'prNew', code: b.code || 'NEW', type: b.type || 'percentage', value: b.value || 10, description: b.description || '', active: true, uses: 0, maxUses: b.maxUses || null };
  },
  'PATCH /api/owner/locations/current/promotions/:id': () => ({ ok: true }),

  // ── Admin Staff ──
  'GET /api/owner/locations/current/staff': () => ({ staff: STAFF }),
  'PATCH /api/owner/locations/current/staff/:id': () => ({ ok: true }),
  'POST /api/owner/locations/current/staff/invite': () => ({ ok: true, invited: true }),

  // ── Admin Inventory ──
  'GET /api/owner/locations/current/inventory': () => ({ items: INVENTORY_ITEMS }),
  'PATCH /api/owner/locations/current/inventory/:id': () => ({ ok: true }),

  // ── Admin Payouts ──
  'GET /api/owner/locations/current/payouts': () => ({ payouts: PAYOUTS }),

  // ── Courier ──
  'GET /api/courier/me/assignments': () => ({ assignments: COURIER_ASSIGNMENTS }),
  'GET /api/courier/me': () => ({
    id: 'cu1', full_name: 'Ardit Kelmendi', masked_email: 'a****i@email.com',
    masked_phone: '+355 69 555 111', last_login_at: ago(10),
  }),
  'GET /api/courier/me/payouts': () => ({ today: 5400 }),
  'POST /api/courier/shifts/transition': () => ({ ok: true }),
  'POST /api/courier/assignments/:id/accept': () => ({ ok: true }),
  'POST /api/courier/assignments/:id/reject': () => ({ ok: true }),
  'POST /api/courier/assignments/:id/picked-up': () => ({ ok: true }),
  'POST /api/courier/assignments/:id/delivered': () => ({ ok: true }),
  'POST /api/courier/assignments/:id/cancel': () => ({ ok: true }),
  'GET /api/courier/me/earnings': () => COURIER_EARNINGS,
  'GET /api/courier/me/history': () => ({ deliveries: COURIER_HISTORY }),
  'GET /api/courier/me/shifts': () => ({ shifts: COURIER_SHIFTS }),
  'POST /api/courier/me/shifts': () => ({ id: 'shNew', date: '2026-06-04', startTime: '11:00', endTime: '15:00', status: 'scheduled' as const }),
};

function matchRoute(pattern: string, url: string): Record<string, string> | null {
  const patternParts = pattern.replace(/^[A-Z]+ /, '').split('/');
  const urlParts = url.replace(/\?.*$/, '').split('/');
  if (patternParts.length !== urlParts.length) return null;
  const params: Record<string, string> = {};
  for (let i = 0; i < patternParts.length; i++) {
    if (patternParts[i]!.startsWith(':')) {
      params[patternParts[i]!.slice(1)] = urlParts[i]!;
    } else if (patternParts[i] !== urlParts[i]) {
      return null;
    }
  }
  return params;
}

export function getMockResponse(method: string, url: string, body?: unknown): { ok: boolean; data: unknown } | null {
  const key = `${method.toUpperCase()} ${url.replace(/\?.*$/, '')}`;
  const direct = MOCK[key];
  if (direct) return { ok: true, data: direct(url, method, body) };

  for (const pattern of Object.keys(MOCK)) {
    const params = matchRoute(pattern, url);
    if (params && pattern.startsWith(method.toUpperCase())) {
      return { ok: true, data: MOCK[pattern]!(url, method, body) };
    }
  }

  return null;
}

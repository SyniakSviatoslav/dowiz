window.MOCK = {

  location: {
    id: "dubin-sushi-tirana",
    name: "Dubin & Sushi",
    slug: "dubin-sushi",
    phone: "+355 69 234 567",
    address: "Rruga Ismail Qemali 8, Tiranë",
    status: "open",
    closes_at: "23:00",
    rating: 4.9,
    rating_count: 218,
    delivery_time: "30–45 min",
    delivery_fee: 200,
    min_order: 800,
    hero_style: "cover",
    menu_layout: "grid",
    menu_version: 12,
  },

  categories: [
    { id: "c1", name: "Sushi Rolls",    position: 1, count: 8 },
    { id: "c2", name: "Nigiri & Sashimi", position: 2, count: 6 },
    { id: "c3", name: "Hot Dishes",      position: 3, count: 5 },
    { id: "c4", name: "Soups & Salads",  position: 4, count: 4 },
    { id: "c5", name: "Drinks",          position: 5, count: 5 },
    { id: "c6", name: "Desserts",        position: 6, count: 3 },
  ],

  products: [
    // SUSHI ROLLS
    { id: "p1",  cat: "c1", name: "Dragon Roll",        desc: "Shrimp tempura, avocado, cucumber, eel sauce, tobiko",            price: 850,  available: true,  allergens: ["gluten","seafood","eggs"],   calories: 420, position: 1 },
    { id: "p2",  cat: "c1", name: "Spicy Tuna Roll",    desc: "Fresh tuna, spicy mayo, cucumber, sesame seeds",                  price: 780,  available: true,  allergens: ["gluten","seafood"],          calories: 380, position: 2 },
    { id: "p3",  cat: "c1", name: "Rainbow Roll",       desc: "California roll topped with tuna, salmon, yellowtail, avocado",   price: 920,  available: false, unavailable_until: "21:00", allergens: ["gluten","seafood","eggs"], calories: 460, position: 3 },
    { id: "p4",  cat: "c1", name: "Philadelphia Roll",  desc: "Salmon, cream cheese, cucumber, sesame",                          price: 760,  available: true,  allergens: ["gluten","seafood","milk"],   calories: 400, position: 4 },
    { id: "p5",  cat: "c1", name: "Volcano Roll",       desc: "Crab, spicy scallop, masago, green onion",                        price: 890,  available: true,  allergens: ["gluten","seafood","eggs"],   calories: 440, position: 5 },
    { id: "p6",  cat: "c1", name: "Vegetable Maki",     desc: "Avocado, cucumber, pickled daikon, sesame",                       price: 560,  available: true,  allergens: ["gluten"],                    calories: 280, position: 6 },
    { id: "p7",  cat: "c1", name: "Salmon Avocado",     desc: "Norwegian salmon, ripe avocado, light cream sauce",               price: 720,  available: true,  allergens: ["gluten","seafood","milk"],   calories: 360, position: 7 },
    { id: "p8",  cat: "c1", name: "Tempura Prawn",      desc: "Tiger prawn tempura, spicy mayo, cucumber, tobiko",               price: 840,  available: true,  allergens: ["gluten","seafood","eggs"],   calories: 430, position: 8 },
    // NIGIRI & SASHIMI
    { id: "p9",  cat: "c2", name: "Salmon Nigiri ×2",   desc: "Hand-pressed sushi rice with fresh Atlantic salmon",              price: 420,  available: true,  allergens: ["seafood"],                   calories: 180, position: 1 },
    { id: "p10", cat: "c2", name: "Tuna Nigiri ×2",     desc: "Hand-pressed sushi rice with bluefin tuna",                      price: 480,  available: true,  allergens: ["seafood"],                   calories: 170, position: 2 },
    { id: "p11", cat: "c2", name: "Salmon Sashimi ×5",  desc: "5 slices of premium Norwegian salmon, wasabi, pickled ginger",   price: 680,  available: true,  allergens: ["seafood"],                   calories: 220, position: 3 },
    { id: "p12", cat: "c2", name: "Tuna Sashimi ×5",    desc: "5 slices of premium bluefin tuna",                               price: 760,  available: true,  allergens: ["seafood"],                   calories: 200, position: 4 },
    { id: "p13", cat: "c2", name: "Sashimi Platter",    desc: "10 pieces: salmon, tuna, yellowtail, octopus, sweet shrimp",     price: 1480, available: true,  allergens: ["seafood"],                   calories: 380, position: 5 },
    { id: "p14", cat: "c2", name: "Unagi Nigiri ×2",    desc: "Grilled freshwater eel, teriyaki glaze, sesame",                 price: 540,  available: true,  allergens: ["seafood","gluten"],           calories: 210, position: 6 },
    // HOT DISHES
    { id: "p15", cat: "c3", name: "Chicken Ramen",      desc: "Rich tonkotsu broth, chashu chicken, soft egg, nori, green onion", price: 680, available: true, allergens: ["gluten","eggs","milk"],      calories: 580, position: 1 },
    { id: "p16", cat: "c3", name: "Shrimp Gyoza ×6",    desc: "Pan-fried prawn and ginger dumplings, ponzu dipping sauce",      price: 480,  available: true,  allergens: ["gluten","seafood"],          calories: 310, position: 2 },
    { id: "p17", cat: "c3", name: "Yakitori Skewers ×4",desc: "Grilled chicken thigh skewers, tare sauce, spring onion",      price: 560,  available: true,  allergens: ["gluten"],                    calories: 360, position: 3 },
    { id: "p18", cat: "c3", name: "Edamame",            desc: "Steamed young soybeans with sea salt",                           price: 280,  available: true,  allergens: [],                            calories: 160, position: 4 },
    { id: "p19", cat: "c3", name: "Chicken Teriyaki",   desc: "Grilled chicken, homemade teriyaki glaze, steamed rice, salad",  price: 720,  available: true,  allergens: ["gluten"],                    calories: 540, position: 5 },
    // SOUPS & SALADS
    { id: "p20", cat: "c4", name: "Miso Soup",          desc: "Traditional dashi broth, silken tofu, wakame, green onion",     price: 240,  available: true,  allergens: ["gluten"],                    calories: 80,  position: 1 },
    { id: "p21", cat: "c4", name: "Tom Yum Soup",       desc: "Lemongrass broth, shrimp, mushrooms, chili, kaffir lime",       price: 480,  available: true,  allergens: ["seafood"],                   calories: 180, position: 2 },
    { id: "p22", cat: "c4", name: "Seaweed Salad",      desc: "Wakame, cucumber, sesame oil, rice vinegar, chili flakes",      price: 320,  available: true,  allergens: ["gluten"],                    calories: 120, position: 3 },
    { id: "p23", cat: "c4", name: "Kaiso Salad",        desc: "Mixed sea vegetables, ginger dressing, toasted sesame",         price: 360,  available: true,  allergens: ["gluten","seafood"],           calories: 140, position: 4 },
    // DRINKS
    { id: "p24", cat: "c5", name: "Sencha Green Tea",   desc: "Premium Japanese green tea, hot or iced",                       price: 180,  available: true,  allergens: [],                            calories: 0,   position: 1 },
    { id: "p25", cat: "c5", name: "Yuzu Lemonade",      desc: "Fresh yuzu citrus, sparkling water, honey, mint",               price: 240,  available: true,  allergens: [],                            calories: 90,  position: 2 },
    { id: "p26", cat: "c5", name: "Matcha Latte",       desc: "Ceremonial grade matcha, oat milk, light honey",                price: 280,  available: true,  allergens: ["milk"],                      calories: 120, position: 3 },
    { id: "p27", cat: "c5", name: "Sake (180ml)",       desc: "Junmai Daiginjo — floral, light, fruity finish",                price: 480,  available: true,  allergens: ["gluten"],                    calories: 220, position: 4 },
    { id: "p28", cat: "c5", name: "San Pellegrino 0.5l",desc: "Sparkling mineral water",                                     price: 120,  available: true,  allergens: [],                            calories: 0,   position: 5 },
    // DESSERTS
    { id: "p29", cat: "c6", name: "Mochi Ice Cream ×3", desc: "Strawberry, matcha, mango — rice cake filled with ice cream",   price: 380,  available: true,  allergens: ["milk","gluten"],             calories: 280, position: 1 },
    { id: "p30", cat: "c6", name: "Matcha Cheesecake",  desc: "Baked cheesecake with ceremonial matcha, white chocolate glaze", price: 340, available: true,  allergens: ["milk","eggs","gluten"],      calories: 360, position: 2 },
    { id: "p31", cat: "c6", name: "Taiyaki",            desc: "Fish-shaped waffle with red bean or custard filling, warm",     price: 260,  available: true,  allergens: ["gluten","eggs","milk"],      calories: 240, position: 3 },
  ],

  // Cart state — used in Cart, Checkout, OrderStatus
  cart: [
    { product_id: "p1",  name: "Dragon Roll",       price: 850, qty: 1 },
    { product_id: "p15", name: "Chicken Ramen",      price: 680, qty: 1 },
    { product_id: "p24", name: "Sencha Green Tea",   price: 180, qty: 2 },
  ],
  cart_subtotal: 1890,
  cart_delivery_fee: 200,
  cart_total: 2090,
  promo_code: "SUSHI15",
  promo_discount: 284,
  cart_total_with_promo: 1806,

  // Active order — used in OrderStatus, Dashboard, Courier screens
  order: {
    id: "2301",
    short_id: "#2301",
    status: "IN_DELIVERY",
    type: "delivery",
    customer: { name: "Sara Mancini", phone: "+355 69 876 543" },
    courier: {
      id: "cu1", name: "Ardit Kelmendi", phone: "+355 69 555 111",
      lat: 41.3290, lng: 19.8170, accuracy: 6,
    },
    address: "Rruga Barrikadave 22, Tiranë",
    lat: 41.3305, lng: 19.8195,
    items: [
      { name: "Dragon Roll",     qty: 1, price: 850 },
      { name: "Chicken Ramen",   qty: 1, price: 680 },
      { name: "Sencha Green Tea",qty: 2, price: 180 },
    ],
    subtotal: 1890,
    delivery_fee: 200,
    discount: 284,
    total: 1806,
    payment: "cash",
    created_at: "20:12",
    confirmed_at: "20:14",
    eta_minutes: 14,
    pickup_code: "DS-7734",
    distance_km: 1.8,
    rejection_reason: null,
    scheduled_at: null,
  },

  // All active orders — used in Dashboard and Orders kanban
  active_orders: [
    { id: "2301", status: "IN_DELIVERY", customer_phone: "+355 69 876 543", items_short: "Dragon Roll ×1, Ramen ×1, Tea ×2", total: 1806, created_ago: "22 min", courier_name: "Ardit",  eta: 14  },
    { id: "2300", status: "PREPARING",   customer_phone: "+355 69 432 187", items_short: "Sashimi Platter ×1, Miso Soup ×2", total: 1960, created_ago: "15 min", courier_name: "Blerim", eta: null },
    { id: "2299", status: "CONFIRMED",   customer_phone: "+355 69 321 654", items_short: "Philadelphia Roll ×2, Edamame ×1", total: 1800, created_ago: "8 min",  courier_name: null,     eta: null },
    { id: "2298", status: "PENDING",     customer_phone: "+355 69 111 999", items_short: "Dragon Roll ×1, Rainbow Roll ×1",  total: 1770, created_ago: "2 min",  courier_name: null,     eta: null, timeout_remaining: 487 },
    { id: "2297", status: "READY",       customer_phone: "+355 69 777 333", items_short: "Salmon Sashimi ×5, Sake ×1",       total: 1160, created_ago: "28 min", courier_name: "Ardit",  eta: null },
    { id: "2296", status: "SCHEDULED",   customer_phone: "+355 69 444 222", items_short: "Rainbow Roll ×2, Ramen ×1",        total: 2520, created_ago: null,     courier_name: null,     eta: null, scheduled_at: "22:00" },
  ],

  // Dashboard stats
  stats: {
    orders_today: 63, orders_trend: "+11",
    revenue_today: 87400, revenue_trend: "+18%",
    active_orders: 5,
    couriers_online: 2,
    avg_delivery_min: 32,
  },

  // Couriers
  couriers: [
    { id: "cu1", name: "Ardit Kelmendi",  status: "busy",   lat: 41.3290, lng: 19.8170, orders_today: 11, distance_today: 34.2, rating: 4.9, avg_delivery_min: 28, order_id: "2301" },
    { id: "cu2", name: "Blerim Hoxhaj",   status: "online", lat: 41.3320, lng: 19.8230, orders_today: 8,  distance_today: 25.1, rating: 4.8, avg_delivery_min: 31, order_id: "2300" },
    { id: "cu3", name: "Genci Dervishi",  status: "offline",lat: 41.3260, lng: 19.8140, orders_today: 7,  distance_today: 21.8, rating: 4.7, avg_delivery_min: 35, order_id: null   },
  ],

  // Analytics (7-day window)
  analytics: {
    revenue_7d:  [42100, 51800, 47300, 68200, 74900, 81400, 87400],
    orders_7d:   [28,    35,    31,    46,    52,    58,    63   ],
    labels:      ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
    top_products: [
      { name: "Dragon Roll",        orders: 124, revenue: 105400 },
      { name: "Salmon Sashimi ×5",  orders: 98,  revenue: 66640  },
      { name: "Chicken Ramen",      orders: 87,  revenue: 59160  },
      { name: "Philadelphia Roll",  orders: 76,  revenue: 57760  },
      { name: "Spicy Tuna Roll",    orders: 71,  revenue: 55380  },
    ],
    heatmap: [
      [4,20,100],[4,21,88],[5,20,96],[5,21,90],[6,13,70],[6,20,85],[6,21,78],
      [0,12,55], [1,12,60],[2,12,52],[3,12,66],[4,12,72],[5,12,80],
    ],
    ltv_chart: [
      { segment: "1 order",   count: 89,  avg_ltv: 1900  },
      { segment: "2–5 orders",count: 142, avg_ltv: 7400  },
      { segment: "6–15",      count: 67,  avg_ltv: 18200 },
      { segment: "16+",       count: 24,  avg_ltv: 42600 },
    ],
    order_latency: { p50: 38, p95: 72, p99: 118 }, // ms WS delivery
  },

  // CRM customers
  customers: [
    { id: "cu1", name: "Sara Mancini",     phone: "+355 69 876 543", orders: 18, ltv: 32400, last_order: "today",        aliases: [] },
    { id: "cu2", name: "Alina Popa",       phone: "+355 69 432 187", orders: 11, ltv: 19800, last_order: "yesterday",    aliases: [] },
    { id: "cu3", name: "Bled Gjoni",       phone: "+355 69 321 654", orders: 27, ltv: 51300, last_order: "today",        aliases: ["+355 69 321 655"] },
    { id: "cu4", name: "Dorina Shehu",     phone: "+355 69 111 999", orders: 4,  ltv: 6800,  last_order: "3 weeks ago",  aliases: [] },
    { id: "cu5", name: "Erion Berisha",    phone: "+355 69 777 333", orders: 14, ltv: 24200, last_order: "2 days ago",   aliases: [] },
    { id: "cu6", name: "Fatbardha Koci",   phone: "+355 69 444 222", orders: 1,  ltv: 2090,  last_order: "2 months ago", aliases: [] },
    { id: "cu7", name: "Gjergji Marku",    phone: "+355 69 555 888", orders: 32, ltv: 61400, last_order: "today",        aliases: [] },
  ],

  // Owner
  owner: { name: "Kenji Tanaka", initials: "KT", plan: "Pro" },

  // Dashboard alerts
  alerts: [
    { id: "a1", severity: "warning",  message: "Rainbow Roll has been on stop-list for 3 hours. Avg daily revenue: 1,840 ALL.", action: "Restore" },
    { id: "a2", severity: "info",     message: "Scheduled order #2296 activates at 21:45. Assign a courier in advance.", action: "Assign" },
  ],

  // Checkout timeslots
  timeslots: [
    { id: "ts1", label: "Today 21:00–21:30",  available: true  },
    { id: "ts2", label: "Today 21:30–22:00",  available: true  },
    { id: "ts3", label: "Today 22:00–22:30",  available: false },
    { id: "ts4", label: "Tomorrow 12:00–12:30", available: true },
    { id: "ts5", label: "Tomorrow 12:30–13:00", available: true },
  ],

  // Promotions
  promotions: [
    { id: "pr1", code: "SUSHI15",   type: "percentage",  value: 15,   desc: "15% off first order",                           active: true,  uses: 58,  max_uses: 200 },
    { id: "pr2", code: "ROLL2GET1", type: "buy_x_get_y", value: null, desc: "Buy 2 rolls — get 3rd free",                    active: true,  uses: 31,  max_uses: null },
    { id: "pr3", code: "LUNCH",     type: "happy_hour",  value: 20,   desc: "Mon–Fri 12:00–14:00 — 20% off hot dishes",      active: false, uses: 124, max_uses: null },
    { id: "pr4", code: "COMBO1",    type: "combo",       value: null, desc: "Dragon Roll + Miso Soup + Green Tea — 1,800 ALL", active: true, uses: 44,  max_uses: null },
  ],

  // API keys (Branding/Integrations tab)
  api_keys: [
    { id: "k1", prefix: "sk_live_dS8f", name: "Google Sheets export", scopes: ["orders:read","customers:read"], last_used: "2 hours ago", created: "14 days ago" },
  ],

  // Webhooks
  webhooks: [
    { id: "wh1", url: "https://hooks.zapier.com/hooks/catch/123456/abcdef/", events: ["order.created","order.status_changed"], active: true, failure_count: 0, last_triggered: "20 min ago" },
  ],
};

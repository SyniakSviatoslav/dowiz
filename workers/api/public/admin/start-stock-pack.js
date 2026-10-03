// THE SUSHI STARTER PACK (W-STOCK P1): about forty things a sushi kitchen in
// Durres buys, so a venue's shelf is filled in an hour instead of a week.
//
// EVERY NUMBER HERE IS A DEFAULT TO CHECK, not a fact about this venue: the
// losses (cleanPm / cookPm, per mille) are common kitchen yields, and the
// nutrition (per 100 g or ml, per ONE piece for a supply counted in pieces) is
// rounded from public tables (USDA FoodData Central). The sheet marks them so,
// and the owner edits every one on the ingredient's card. Costs are NOT here:
// a price is the venue's own paper.
//
// Fields: id (stable: a re-run updates, never duplicates), unit (g|ml|unit),
// kind, cat (a CATS key), clean / cook (per mille; 1000 = none lost),
// pack (what it is bought in, base units), wpu (grams per piece), nut
// [kcal, protein, fat, carbs], n (the name in each language).
//
// ASCII QUOTES ONLY as string delimiters.

export const CATS = {
  fish: { sq: 'Peshk dhe fruta deti', en: 'Fish and seafood', uk: 'Риба й морепродукти', ru: 'Рыба и морепродукты' },
  rice: { sq: 'Oriz dhe bazë', en: 'Rice and basics', uk: 'Рис і основа', ru: 'Рис и основа' },
  veg: { sq: 'Perime dhe fruta', en: 'Vegetables and fruit', uk: 'Овочі й фрукти', ru: 'Овощи и фрукты' },
  dairy: { sq: 'Bulmet', en: 'Dairy', uk: 'Молочне', ru: 'Молочное' },
  sauce: { sq: 'Salca', en: 'Sauces', uk: 'Соуси', ru: 'Соусы' },
  fry: { sq: 'Për skuqje', en: 'Frying', uk: 'Для фритюру', ru: 'Для фритюра' },
  other: { sq: 'Të tjera', en: 'Other', uk: 'Інше', ru: 'Прочее' },
  pack: { sq: 'Paketim', en: 'Packaging', uk: 'Упаковка', ru: 'Упаковка' },
  drinks: { sq: 'Pije', en: 'Drinks', uk: 'Напої', ru: 'Напитки' },
};

const F = 'food_ingredient', C = 'condiment', P = 'packaging', R = 'resale';

export const PACK = [
  { id: 'pack-salmon', unit: 'g', kind: F, cat: 'fish', clean: 900, cook: 1000, pack: 1000, nut: [208, 20.4, 13.4, 0], n: { sq: 'Salmon (fileto)', en: 'Salmon fillet', uk: 'Лосось (філе)', ru: 'Лосось (филе)' } },
  { id: 'pack-tuna', unit: 'g', kind: F, cat: 'fish', clean: 950, cook: 1000, pack: 1000, nut: [109, 24.4, 0.5, 0], n: { sq: 'Ton (fileto)', en: 'Tuna loin', uk: 'Тунець (філе)', ru: 'Тунец (филе)' } },
  { id: 'pack-eel', unit: 'g', kind: F, cat: 'fish', clean: 1000, cook: 1000, pack: 500, nut: [236, 23.7, 15, 0], n: { sq: 'Ngjalë e pjekur (unagi)', en: 'Grilled eel (unagi)', uk: 'Вугор запечений (унагі)', ru: 'Угорь жареный (унаги)' } },
  { id: 'pack-shrimp', unit: 'g', kind: F, cat: 'fish', clean: 1000, cook: 800, pack: 1000, nut: [85, 20.1, 0.5, 0], n: { sq: 'Karkaleca të qëruara', en: 'Shrimp, peeled', uk: 'Креветки очищені', ru: 'Креветки очищенные' } },
  { id: 'pack-surimi', unit: 'g', kind: F, cat: 'fish', clean: 1000, cook: 1000, pack: 500, nut: [99, 15.2, 0.9, 6.9], n: { sq: 'Shkopinj gaforre (surimi)', en: 'Crab sticks (surimi)', uk: 'Крабові палички (сурімі)', ru: 'Крабовые палочки (сурими)' } },
  { id: 'pack-tobiko', unit: 'g', kind: F, cat: 'fish', clean: 1000, cook: 1000, pack: 500, nut: [143, 22.3, 6.4, 1.5], n: { sq: 'Havjar peshku (tobiko)', en: 'Flying fish roe (tobiko)', uk: 'Ікра летючої риби (тобіко)', ru: 'Икра летучей рыбы (тобико)' } },
  { id: 'pack-rice', unit: 'g', kind: F, cat: 'rice', clean: 1000, cook: 2200, pack: 10000, nut: [360, 6.6, 0.6, 79], n: { sq: 'Oriz për sushi', en: 'Sushi rice', uk: 'Рис для суші', ru: 'Рис для суши' } },
  { id: 'pack-rice-vinegar', unit: 'ml', kind: C, cat: 'rice', clean: 1000, cook: 1000, pack: 1000, nut: [18, 0, 0, 0], n: { sq: 'Uthull orizi', en: 'Rice vinegar', uk: 'Рисовий оцет', ru: 'Рисовый уксус' } },
  { id: 'pack-sugar', unit: 'g', kind: C, cat: 'rice', clean: 1000, cook: 1000, pack: 1000, nut: [387, 0, 0, 100], n: { sq: 'Sheqer', en: 'Sugar', uk: 'Цукор', ru: 'Сахар' } },
  { id: 'pack-salt', unit: 'g', kind: C, cat: 'rice', clean: 1000, cook: 1000, pack: 1000, nut: [0, 0, 0, 0], n: { sq: 'Kripë', en: 'Salt', uk: 'Сіль', ru: 'Соль' } },
  { id: 'pack-nori', unit: 'unit', kind: F, cat: 'rice', clean: 1000, cook: 1000, pack: 50, wpu: 3, nut: [6, 1.2, 0.1, 0.7], n: { sq: 'Nori (fletë)', en: 'Nori sheets', uk: 'Норі (листи)', ru: 'Нори (листы)' } },
  { id: 'pack-avocado', unit: 'g', kind: F, cat: 'veg', clean: 700, cook: 1000, pack: 4000, nut: [160, 2, 14.7, 8.5], n: { sq: 'Avokado', en: 'Avocado', uk: 'Авокадо', ru: 'Авокадо' } },
  { id: 'pack-cucumber', unit: 'g', kind: F, cat: 'veg', clean: 850, cook: 1000, pack: 1000, nut: [15, 0.7, 0.1, 3.6], n: { sq: 'Kastravec', en: 'Cucumber', uk: 'Огірок', ru: 'Огурец' } },
  { id: 'pack-carrot', unit: 'g', kind: F, cat: 'veg', clean: 850, cook: 1000, pack: 1000, nut: [41, 0.9, 0.2, 9.6], n: { sq: 'Karotë', en: 'Carrot', uk: 'Морква', ru: 'Морковь' } },
  { id: 'pack-spring-onion', unit: 'g', kind: F, cat: 'veg', clean: 850, cook: 1000, pack: 500, nut: [32, 1.8, 0.2, 7.3], n: { sq: 'Qepë e njomë', en: 'Spring onion', uk: 'Зелена цибуля', ru: 'Зелёный лук' } },
  { id: 'pack-mango', unit: 'g', kind: F, cat: 'veg', clean: 650, cook: 1000, pack: 1000, nut: [60, 0.8, 0.4, 15], n: { sq: 'Mango', en: 'Mango', uk: 'Манго', ru: 'Манго' } },
  { id: 'pack-lettuce', unit: 'g', kind: F, cat: 'veg', clean: 800, cook: 1000, pack: 500, nut: [15, 1.4, 0.2, 2.9], n: { sq: 'Sallatë jeshile', en: 'Lettuce', uk: 'Салат листовий', ru: 'Салат листовой' } },
  { id: 'pack-sesame', unit: 'g', kind: F, cat: 'veg', clean: 1000, cook: 1000, pack: 1000, nut: [573, 17.7, 49.7, 23.4], n: { sq: 'Susam i bardhë', en: 'White sesame', uk: 'Кунжут білий', ru: 'Кунжут белый' } },
  { id: 'pack-sesame-black', unit: 'g', kind: F, cat: 'veg', clean: 1000, cook: 1000, pack: 500, nut: [573, 17.7, 49.7, 23.4], n: { sq: 'Susam i zi', en: 'Black sesame', uk: 'Кунжут чорний', ru: 'Кунжут чёрный' } },
  { id: 'pack-cream-cheese', unit: 'g', kind: F, cat: 'dairy', clean: 1000, cook: 1000, pack: 2000, nut: [342, 5.9, 34.2, 4.1], n: { sq: 'Djathë krem', en: 'Cream cheese', uk: 'Вершковий сир', ru: 'Сливочный сыр' } },
  { id: 'pack-soy', unit: 'ml', kind: C, cat: 'sauce', clean: 1000, cook: 1000, pack: 1000, nut: [53, 8.1, 0.6, 4.9], n: { sq: 'Salcë soje', en: 'Soy sauce', uk: 'Соєвий соус', ru: 'Соевый соус' } },
  { id: 'pack-wasabi', unit: 'g', kind: C, cat: 'sauce', clean: 1000, cook: 1000, pack: 1000, nut: [109, 4.8, 0.6, 23.5], n: { sq: 'Wasabi', en: 'Wasabi', uk: 'Васабі', ru: 'Васаби' } },
  { id: 'pack-ginger', unit: 'g', kind: C, cat: 'sauce', clean: 1000, cook: 1000, pack: 1000, nut: [20, 0.2, 0.1, 4.5], n: { sq: 'Xhenxhefil turshi', en: 'Pickled ginger', uk: 'Імбир маринований', ru: 'Имбирь маринованный' } },
  { id: 'pack-unagi-sauce', unit: 'ml', kind: C, cat: 'sauce', clean: 1000, cook: 1000, pack: 1000, nut: [200, 3, 0, 45], n: { sq: 'Salcë unagi', en: 'Unagi sauce', uk: 'Соус унагі', ru: 'Соус унаги' } },
  { id: 'pack-spicy-mayo', unit: 'ml', kind: C, cat: 'sauce', clean: 1000, cook: 1000, pack: 1000, nut: [600, 1, 65, 3], n: { sq: 'Majonezë pikante', en: 'Spicy mayo', uk: 'Гострий майонез', ru: 'Острый майонез' } },
  { id: 'pack-mayo', unit: 'ml', kind: C, cat: 'sauce', clean: 1000, cook: 1000, pack: 1000, nut: [680, 1, 75, 0.6], n: { sq: 'Majonezë', en: 'Mayonnaise', uk: 'Майонез', ru: 'Майонез' } },
  { id: 'pack-teriyaki', unit: 'ml', kind: C, cat: 'sauce', clean: 1000, cook: 1000, pack: 1000, nut: [89, 5.9, 0, 15.6], n: { sq: 'Salcë teriyaki', en: 'Teriyaki sauce', uk: 'Соус теріякі', ru: 'Соус терияки' } },
  { id: 'pack-tempura', unit: 'g', kind: F, cat: 'fry', clean: 1000, cook: 1000, pack: 1000, nut: [350, 9, 1, 75], n: { sq: 'Miell tempura', en: 'Tempura flour', uk: 'Борошно темпура', ru: 'Мука темпура' } },
  { id: 'pack-panko', unit: 'g', kind: F, cat: 'fry', clean: 1000, cook: 1000, pack: 1000, nut: [395, 13, 5, 72], n: { sq: 'Panko', en: 'Panko breadcrumbs', uk: 'Панко', ru: 'Панко' } },
  { id: 'pack-oil', unit: 'ml', kind: C, cat: 'fry', clean: 1000, cook: 1000, pack: 5000, nut: [884, 0, 100, 0], n: { sq: 'Vaj për skuqje', en: 'Frying oil', uk: 'Олія для фритюру', ru: 'Масло для фритюра' } },
  { id: 'pack-tofu', unit: 'g', kind: F, cat: 'other', clean: 1000, cook: 1000, pack: 1000, nut: [76, 8, 4.8, 1.9], n: { sq: 'Tofu', en: 'Tofu', uk: 'Тофу', ru: 'Тофу' } },
  { id: 'pack-miso', unit: 'g', kind: C, cat: 'other', clean: 1000, cook: 1000, pack: 1000, nut: [199, 12.8, 6, 26.5], n: { sq: 'Pastë miso', en: 'Miso paste', uk: 'Місо паста', ru: 'Мисо паста' } },
  { id: 'pack-wakame', unit: 'g', kind: F, cat: 'other', clean: 1000, cook: 1000, pack: 500, nut: [45, 3, 0.6, 9.1], n: { sq: 'Alga wakame', en: 'Wakame seaweed', uk: 'Водорості вакаме', ru: 'Водоросли вакаме' } },
  { id: 'pack-edamame', unit: 'g', kind: F, cat: 'other', clean: 1000, cook: 1000, pack: 1000, nut: [121, 11.9, 5.2, 8.9], n: { sq: 'Edamame', en: 'Edamame', uk: 'Едамаме', ru: 'Эдамаме' } },
  { id: 'pack-chicken', unit: 'g', kind: F, cat: 'other', clean: 900, cook: 750, pack: 2000, nut: [121, 19.7, 4.1, 0], n: { sq: 'Kofshë pule', en: 'Chicken thigh', uk: 'Куряче стегно', ru: 'Куриное бедро' } },
  { id: 'pack-box', unit: 'unit', kind: P, cat: 'pack', pack: 100, n: { sq: 'Kuti sushi', en: 'Sushi box', uk: 'Коробка для суші', ru: 'Коробка для суши' } },
  { id: 'pack-chopsticks', unit: 'unit', kind: P, cat: 'pack', pack: 100, n: { sq: 'Shkopinj ngrënieje', en: 'Chopsticks', uk: 'Палички для їжі', ru: 'Палочки для еды' } },
  { id: 'pack-bag', unit: 'unit', kind: P, cat: 'pack', pack: 100, n: { sq: 'Qese', en: 'Carry bag', uk: 'Пакет', ru: 'Пакет' } },
  { id: 'pack-napkins', unit: 'unit', kind: P, cat: 'pack', pack: 100, n: { sq: 'Peceta', en: 'Napkins', uk: 'Серветки', ru: 'Салфетки' } },
  { id: 'pack-water', unit: 'unit', kind: R, cat: 'drinks', pack: 24, n: { sq: 'Ujë 0,5 l', en: 'Water 0.5 l', uk: 'Вода 0,5 л', ru: 'Вода 0,5 л' } },
  { id: 'pack-tea', unit: 'unit', kind: F, cat: 'drinks', pack: 100, wpu: 2, nut: [0, 0, 0, 0], n: { sq: 'Çaj jeshil (qeskë)', en: 'Green tea bag', uk: 'Зелений чай (пакетик)', ru: 'Зелёный чай (пакетик)' } },
];

// THE KEYWORD TABLE: a dish's name, lower-cased and without accents, split
// into words; a rule fires when a word STARTS WITH one of its stems. Stems in
// the four languages and the Japanese words menus use. Deterministic: the same
// name always proposes the same lines, in PACK order.
//   add   pack ids the dish takes
//   base  a roll / nigiri / bowl: the rice (and nori) every one of them takes
//   drink no box is proposed for it
export const RULES = [
  { k: ['salmon', 'salmo', 'lachs', 'лосос', 'семг', 'сьомг'], add: ['pack-salmon'] },
  { k: ['tuna', 'tonno', 'ton', 'maguro', 'тунец', 'тунц', 'тунець'], add: ['pack-tuna'] },
  { k: ['eel', 'unagi', 'ngjal', 'вугор', 'вугр', 'угор', 'угр'], add: ['pack-eel', 'pack-unagi-sauce'] },
  { k: ['shrimp', 'prawn', 'ebi', 'karkalec', 'gamber', 'кревет'], add: ['pack-shrimp'] },
  { k: ['surimi', 'crab', 'kani', 'gaforr', 'краб'], add: ['pack-surimi'] },
  { k: ['tobiko', 'masago', 'тобико', 'тобіко', 'икр', 'ікр'], add: ['pack-tobiko'] },
  { k: ['avocad', 'avokad', 'авокад'], add: ['pack-avocado'] },
  { k: ['cucumber', 'kappa', 'kastravec', 'огур', 'огір'], add: ['pack-cucumber'] },
  { k: ['carrot', 'karot', 'морков', 'морк'], add: ['pack-carrot'] },
  { k: ['mango', 'манго'], add: ['pack-mango'] },
  { k: ['cheese', 'djath', 'cream', 'krem', 'сыр', 'сир'], add: ['pack-cream-cheese'] },
  { k: ['philadelph', 'filadelf', 'филадел', 'філадел'], add: ['pack-salmon', 'pack-cream-cheese'] },
  { k: ['californ', 'kaliforn', 'калифорн', 'каліфорн'], add: ['pack-surimi', 'pack-avocado', 'pack-cucumber', 'pack-tobiko', 'pack-mayo'] },
  { k: ['dragon', 'дракон'], add: ['pack-eel', 'pack-avocado', 'pack-unagi-sauce'] },
  { k: ['spicy', 'pikant', 'djeges', 'гостр', 'остр', 'пикант', 'пікант'], add: ['pack-spicy-mayo'] },
  { k: ['tempura', 'tempur', 'темпур'], add: ['pack-tempura', 'pack-oil'] },
  { k: ['teriyak', 'терияк', 'теріяк'], add: ['pack-teriyaki'] },
  { k: ['miso', 'мисо', 'місо'], add: ['pack-miso', 'pack-tofu', 'pack-wakame', 'pack-spring-onion'] },
  { k: ['edamame', 'эдамам', 'едамам'], add: ['pack-edamame', 'pack-salt'] },
  { k: ['wakame', 'chuka', 'вакаме', 'чука'], add: ['pack-wakame', 'pack-sesame'] },
  { k: ['chicken', 'pule', 'pul', 'кур'], add: ['pack-chicken'] },
  { k: ['tofu', 'тофу'], add: ['pack-tofu'] },
  { k: ['veg', 'vegan', 'vegjetar', 'perime', 'овоч'], add: ['pack-cucumber', 'pack-avocado', 'pack-carrot'] },
  { k: ['sesam', 'susam', 'кунжут'], add: ['pack-sesame'] },
  { k: ['roll', 'rol', 'maki', 'uramaki', 'futomaki', 'hosomaki', 'temaki', 'gunkan', 'рол', 'гункан'], add: ['pack-rice', 'pack-nori'], base: true },
  { k: ['nigiri', 'нигир', 'нігір', 'sushi', 'суши', 'суші'], add: ['pack-rice'], base: true },
  { k: ['poke', 'bowl', 'поке', 'боул'], add: ['pack-rice'], base: true },
  { k: ['water', 'uje', 'acqua', 'вода'], add: ['pack-water'], drink: true },
  { k: ['tea', 'caj', 'чай'], add: ['pack-tea'], drink: true },
];

/// The box every matched dish that is not a drink goes out in.
export const BOX = 'pack-box';

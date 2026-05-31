import os

dir_path = r"c:\Users\Dell5\Documents\delivery\src\screens"
files = [f for f in os.listdir(dir_path) if f.endswith('.html')]

translations = {
    # Product Names & Descriptions
    "Піца Маргарита": "Spicy Tuna Roll",
    "Томатний соус, сир моцарела, свіжий базилік.": "Fresh tuna, spicy mayo, scallions, sesame.",
    "Томатний соус, сир моцарела, свіжий базилік": "Fresh tuna, spicy mayo, scallions, sesame",
    "Піца Пепероні": "Salmon Avocado Roll",
    "Салямі пепероні, моцарела, томатний соус.": "Fresh salmon, avocado, cream cheese.",
    "Салямі пепероні, моцарела, томатний соус": "Fresh salmon, avocado, cream cheese",
    "Піца 4 Сири": "Dragon Roll",
    "Моцарела, пармезан, горгонзола, рікота.": "Eel, cucumber, avocado, eel sauce.",
    "Моцарела, пармезан, горгонзола, рікота": "Eel, cucumber, avocado, eel sauce",
    "Піца Прошуто": "Shrimp Tempura Roll",
    "Прошуто крудо, рукола, пармезан, моцарела.": "Crispy shrimp, avocado, spicy mayo.",
    "Піца Дябло": "Volcano Roll",
    "Гостра салямі, перець халапеньйо, моцарела.": "Spicy tuna, jalapeño, sriracha, tempura flakes.",
    "Піца Маринара": "Cucumber Roll",
    "Томатний соус, часник, орегано, оливкова олія.": "Fresh cucumber, sesame seeds, nori.",
    "Паста Карбонара": "Tonkotsu Ramen",
    "Спагеті, гуанчіале, пекоріно, яйця.": "Rich pork broth, chashu, soft egg, noodles.",
    "Паста Болоньєзе": "Spicy Miso Ramen",
    "М'ясний соус рагу, томати, пармезан.": "Miso broth, spicy ground pork, corn, scallions.",
    "Кола": "Matcha Tea",

    # Categories
    "Піца": "Sushi",
    "Паста": "Ramen",
    "Салати": "Bao Buns",
    "Напої": "Drinks",
    "Десерти": "Desserts",
    "Соуси": "Sauces",
    "глютен": "gluten",
    "молоко": "dairy",

    # Global UI Strings
    "Меню": "Menu",
    "Замовлення": "Orders",
    "Налаштування": "Settings",
    "Налашт.": "Settings",
    "Головна": "Home",
    "Курʼєри": "Couriers",
    "Кошик": "Cart",

    # Admin Menu Strings
    "Управління меню": "Menu Management",
    "Додати категорію": "Add Category",
    "Додати страву": "Add Dish",
    "Страва": "Dish",
    "Категорії": "Categories",
    "страв у цій категорії": "items in this category",
    "Пошук страви...": "Search items...",
    "В меню (доступно для замовлення)": "In Menu (available for order)",
    "В меню": "In Menu",
    "Приховано": "Hidden",
    "Стоп-лист": "Stop-list",
    "Редагувати страву": "Edit Dish",
    "Фотографія (4:3)": "Photo (4:3)",
    "Завантажити нове фото": "Upload new photo",
    "Назва": "Name",
    "Ціна": "Price",
    "Категорія": "Category",
    "Опис (складники)": "Description (ingredients)",
    "Клієнти бачать страву і можуть її замовити": "Customers can see and order this item",
    "Видалити": "Delete",
    "Скасувати": "Cancel",
    "Зберегти зміни": "Save changes",
    "Обрано:": "Selected:",
    "страви": "items",
    "Приховати всі": "Hide All",
    "Готово": "Done",

    # Client View Strings
    "Тимчасово недоступно": "Temporarily unavailable",
    "Заклад зачинено": "Restaurant closed",
    "Працюємо щодня 10:00–22:00": "Open daily 10:00–22:00",
    "Відкрито · до 22:00": "Open · until 22:00",
    "Відкрито": "Open",
    "до 22:00": "until 22:00",
    "відгуки": "reviews",

    # Dashboard Strings
    "Виручка за сьогодні": "Revenue Today",
    "Нові замовлення": "New Orders",
    "Середній чек": "Avg Order Value",
    "Очікують курʼєра": "Awaiting Courier",
    "Жива стрічка": "Live Feed",
    "Остання година": "Last hour",
    "Готується": "Preparing",
    "Доставляється": "Delivering",
    "Доставлено": "Delivered",
    "Скасовано": "Cancelled",

    # Courier Strings
    "Екран активний": "Screen Active",
    "Додаток у фоні — GPS може зупинитись": "App in background — GPS might stop",
    "Доставка для": "Delivery for",
    "Відкрити в Google Maps": "Open in Google Maps",
    "хвилин": "min",
    "Зателефонувати клієнту": "Call Customer",
    "Забрав замовлення": "Picked up order",
    "Фото підтвердження (необов'язково)": "Photo proof (optional)",
    "Повернутись до завдань": "Return to tasks",

    # Branding Settings Strings
    "Налаштування бренду": "Brand Settings",
    "Логотип та обкладинка": "Logo & Cover",
    "Завантажте лого (PNG, SVG)": "Upload logo (PNG, SVG)",
    "Завантажте обкладинку": "Upload cover",
    "Пресет теми": "Theme Preset",
    "Форма елементів": "Element Shape",
    "Прямі": "Square",
    "Округлі": "Rounded",
    "Пілюлі": "Pill",
    "Layout меню": "Menu Layout",
    "Сітка": "Grid",
    "Список": "List",
    "Налаштування збережено": "Settings saved",
    "ПОПЕРЕДНІЙ ПЕРЕГЛЯД": "PREVIEW",
    "Контраст": "Contrast",
    "Primary color": "Primary color"
}

for file in files:
    filepath = os.path.join(dir_path, file)
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    for k, v in translations.items():
        content = content.replace(k, v)

    # Specific border removal for clean UI (cards shouldn't have borders, just subtle backgrounds)
    # E.g. in 02-admin-dashboard.html -> `border border-brand-border` on QuickStats.
    if file == '02-admin-dashboard.html':
        content = content.replace('bg-brand-bg border border-brand-border', 'bg-brand-surface shadow-sm border-transparent')
        content = content.replace('shadow-md', 'shadow-sm')

    if file == '05-admin-menu.html':
        content = content.replace('border border-brand-border rounded-brand', 'bg-brand-surface rounded-brand border-transparent')
        # Fix category link styles
        content = content.replace('font-medium', 'font-semibold')

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

print("Translated UI strings and applied soft UI tweaks.")

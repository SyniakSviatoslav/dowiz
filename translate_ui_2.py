import os

dir_path = r"c:\Users\Dell5\Documents\delivery\src\screens"
files = [f for f in os.listdir(dir_path) if f.endswith('.html')]

translations = {
    "Settings бренду": "Brand Settings",
    "Томати, моцарела...": "Fresh tuna, spicy mayo...",
    "Салямі, перець...": "Fresh salmon, avocado...",
    "4 Сири": "Dragon Roll",
    "Моцарела, дорблю...": "Eel, cucumber, eel sauce...",
    "Settings збережено": "Settings saved",
    "Аналітика": "Analytics",
    "AI асистент": "AI Assistant",
    "Добрий вечір, Marco 👋": "Good evening, Arben 👋",
    "● ВІДКРИТО": "● OPEN",
    "⏱ Зайнятий режим": "⏱ Busy mode",
    "Курʼєр Ерліс не відповідає вже 5 min": "Courier Erlis unresponsive for 5 min",
    "Зателефонувати": "Call",
    "Відхилити &times;": "Dismiss &times;",
    "+8 vs вчора": "+8 vs yesterday",
    "Дохід": "Revenue",
    "+12% vs вчора": "+12% vs yesterday",
    "Активних": "Active",
    "Курʼєрів": "Couriers",
    "Ардіан": "Ardian",
    "Ерліс": "Erlis",
    "Резарт": "Rezart",
    "АК": "AK",
    "ЕК": "EK",
    "РМ": "RM",
    "Активні замовлення": "Active Orders",
    "Всі": "All",
    "Нові (2)": "New (2)",
    "Нові": "New",
    "В роботі": "In Progress",
    "Доставка": "Delivery",
    "хв тому": "min ago",
    "с тому": "s ago",
    "Нове": "New",
    "Відхилити": "Decline",
    "Підтвердити": "Accept",
    "Підтверджено": "Accepted",
    "Салат Цезар": "Spicy Edamame",
    "Призначити курʼєра": "Assign Courier",
    "Done до видачі": "Ready for pickup",
    "В доставці": "In Delivery",
    "від клієнта": "from customer",
    "Couriers на карті": "Couriers on map",
    "Оновлено 5с тому": "Updated 5s ago",
    "Піца Маргарита &times;2": "Spicy Tuna Roll &times;2",
    "Кола &times;1": "Matcha Tea &times;1",
    "Олена В.": "Edita K.",
    "Під'їзд 2, Поверх 4, Квартира 42": "Entrance 2, Floor 4, Apt 42",
    "хвилин": "min",
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
    "Primary color": "Primary color",
    "Налаштування": "Settings",
    "Налашт.": "Settings",
    "Головна": "Home",
    "Курʼєри": "Couriers",
    "Кошик": "Cart"
}

for file in files:
    filepath = os.path.join(dir_path, file)
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    for k, v in translations.items():
        content = content.replace(k, v)

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

print("Translated secondary UI strings.")

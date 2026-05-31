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
    "vs вчора": "vs yesterday",
    "Дохід": "Revenue",
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
    "Закрити заклад?": "Close restaurant?",
    "New Orders не прийматимуться. Active Orders потрібно буде завершити вручну.": "New Orders won't be accepted. Active Orders must be completed manually.",
    "Закрити заклад": "Close restaurant",
    "● ЗАЧИНЕНО": "● CLOSED",
    "км": "km",
    "Додати": "Add",
    "350г": "350g",
    "до 5MB": "up to 5MB",
    "8м": "8m",
    "Оновлено 5s ago": "Updated 5s ago"
}

for file in files:
    filepath = os.path.join(dir_path, file)
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    for k, v in translations.items():
        content = content.replace(k, v)

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

print("Translated final set of UI strings.")

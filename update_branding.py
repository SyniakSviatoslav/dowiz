import os

dir_path = r"c:\Users\Dell5\Documents\delivery\src\screens"
files = [f for f in os.listdir(dir_path) if f.endswith('.html')]

replacements = {
    "Sakura Asian Kitchen": "Dubin & Sushi",
    "Rruga Taulantia 12, Durrës": "Rruga Ismail Qemali 8, Tiranë",
    "Sakura": "Dubin",
    "Durrës": "Tiranë",
    "069 123 4567": "069 234 567",
    "+355 69 123 456": "+355 69 234 567",
    "+35569123456": "+35569234567"
}

for file in files:
    filepath = os.path.join(dir_path, file)
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # 1. Inject mock-data.js before the first <script> block, if not already there
    if '<script src="mock-data.js"></script>' not in content:
        content = content.replace('<script>', '<script src="mock-data.js"></script>\n  <script>', 1)

    # 2. Text Replacements
    for old, new in replacements.items():
        content = content.replace(old, new)

    # 3. Default to Ocean Fresh theme
    content = content.replace("const savedTheme = localStorage.getItem('dos_mockup_theme');", "let savedTheme = localStorage.getItem('dos_mockup_theme');\n    if (!savedTheme) { savedTheme = 'Ocean Fresh'; localStorage.setItem('dos_mockup_theme', 'Ocean Fresh'); }")

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

print("Updated existing files with Dubin & Sushi branding and mock-data.js")

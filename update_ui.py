import os
import re

dir_path = r"c:\Users\Dell5\Documents\delivery\src\screens"
files = [f for f in os.listdir(dir_path) if f.endswith('.html')]

font_link_old = r'<link href="https://fonts.googleapis.com/css2\?family=DM\+Serif\+Display:ital@0;1&family=DM\+Sans:opsz,wght@9\.\.40,400;9\.\.40,500;9\.\.40,600&family=Cormorant\+Garamond:wght@400;500;600&family=Playfair\+Display:wght@400;500;600&family=JetBrains\+Mono:wght@400;500&display=swap" rel="stylesheet">'
font_link_new = '<link href="https://fonts.googleapis.com/css2?family=Plus+Jakarta+Sans:wght@500;600;700;800&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">'

for file in files:
    filepath = os.path.join(dir_path, file)
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # 1. Fonts Link
    content = re.sub(font_link_old, font_link_new, content)
    
    # 2. Font Families
    content = content.replace("'DM Serif Display', serif", "'Plus Jakarta Sans', sans-serif")
    content = content.replace("'DM Sans', sans-serif", "'Inter', sans-serif")
    content = content.replace("'Cormorant Garamond', serif", "'Plus Jakarta Sans', sans-serif")
    content = content.replace("'Playfair Display', serif", "'Plus Jakarta Sans', sans-serif")

    # 3. Emojis and UI Updates
    content = content.replace('🎨 ', '<i class="ti ti-palette mr-1 text-[14px]"></i> ')
    content = content.replace(".textContent = '🎨 ' + ", ".innerHTML = '<i class=\"ti ti-palette mr-1 text-[14px]\"></i> ' + ")
    
    # 4. Global text replacements
    content = content.replace("Pizza Roma", "Sakura Asian Kitchen")
    content = content.replace("Rruga Myslym Shyri 45, Tiranë", "Rruga Taulantia 12, Durrës")
    content = content.replace("Олена В.", "Edita K.")
    content = content.replace("Marco Rossi", "Arben M.")
    
    # 5. Soften Shadows
    content = content.replace("12%, transparent", "6%, transparent")
    content = content.replace("6%, transparent", "3%, transparent") # This might match the previously replaced 12%->6%, so we use specific replacement
    # Let's be more specific with shadows
    content = content.replace("0 12px 28px color-mix(in srgb, var(--brand-primary) 12%, transparent)", "0 12px 28px color-mix(in srgb, var(--brand-primary) 6%, transparent)")
    content = content.replace("0 4px 8px color-mix(in srgb, var(--brand-primary) 6%, transparent)", "0 4px 8px color-mix(in srgb, var(--brand-primary) 3%, transparent)")
    content = content.replace("0 -8px 24px color-mix(in srgb, var(--brand-primary) 8%, transparent)", "0 -8px 24px color-mix(in srgb, var(--brand-primary) 4%, transparent)")

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

print("Updated base fonts, emojis, and global strings.")

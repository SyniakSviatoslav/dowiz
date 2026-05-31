import os

filepath = r"c:\Users\Dell5\Documents\delivery\src\screens\05-admin-menu.html"
with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Replace the label flex spacing
content = content.replace(
    '<label class="relative inline-flex items-center cursor-pointer">',
    '<label class="relative inline-flex items-center justify-center gap-2 cursor-pointer">'
)

# Remove the ml-2 from the spans
content = content.replace('span class="ml-2 text-[12px]', 'span class="text-[12px]')

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed spacing in 05-admin-menu.html")

import os
import re

dir_path = r"c:\Users\Dell5\Documents\delivery\src\screens"
files = [f for f in os.listdir(dir_path) if f.endswith('.html')]

css_to_add = """
    /* Global Interactions */
    button, .interactive-card {
      transition: transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1), background-color 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease;
    }
    button:active, .interactive-card:active {
      transform: scale(0.97);
    }
"""

for file in files:
    filepath = os.path.join(dir_path, file)
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # 1. JS Fixes
    content = content.replace("document.getElementById('theme-btn').textContent =", "document.getElementById('theme-btn').innerHTML =")

    # 2. Add Global CSS if not present
    if "/* Global Interactions */" not in content:
        content = content.replace("</style>", css_to_add + "\n  </style>")

    # 3. Fix Theme Button specific alignments
    content = content.replace('shadow-sm flex items-center font-medium embed-hidden', 'shadow-sm flex items-center justify-center gap-1.5 font-medium embed-hidden')
    content = content.replace('shadow-sm flex items-center font-semibold embed-hidden', 'shadow-sm flex items-center justify-center gap-1.5 font-semibold embed-hidden')
    # Remove mr-1 from theme-btn icon
    content = content.replace('<i class="ti ti-palette mr-1 text-[14px]"></i>', '<i class="ti ti-palette text-[14px]"></i>')
    content = content.replace('<i class="ti ti-palette mr-1"></i>', '<i class="ti ti-palette"></i>')

    # Add interactive-card to product cards
    content = content.replace('product-card bg-brand-bg border border-transparent', 'product-card interactive-card bg-brand-bg border border-transparent')
    content = content.replace('bg-brand-surface rounded-brand border-transparent p-[12px] min-h-[60px] flex items-center gap-3 sm:gap-4 hover:shadow-sm transition-shadow group', 'interactive-card bg-brand-surface rounded-brand border-transparent p-[12px] min-h-[60px] flex items-center gap-3 sm:gap-4 hover:shadow-md cursor-pointer group')

    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)

print("Applied global fixes.")

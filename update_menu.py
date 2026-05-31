import re

filepath = r"c:\Users\Dell5\Documents\delivery\src\screens\07-admin-menu.html"
with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update Toolbar with badges
toolbar_target = """<h1 class="text-[18px] font-semibold text-brand-text flex items-center gap-2">
        <i class="ti ti-tools-kitchen-2 text-brand-primary hidden sm:block"></i>
        Menu Management
      </h1>"""

toolbar_replacement = """<div class="flex items-center gap-3">
        <h1 class="text-[18px] font-semibold text-brand-text flex items-center gap-2">
          <i class="ti ti-tools-kitchen-2 text-brand-primary hidden sm:block"></i>
          Menu Management
        </h1>
        <div class="hidden md:flex items-center gap-2">
          <span class="px-2 py-0.5 bg-brand-surface border border-brand-border text-brand-text-muted text-[11px] font-bold rounded-sm uppercase tracking-wide" title="Version increments on every change">v.12</span>
          <span class="px-2 py-0.5 bg-semantic-success/10 text-semantic-success text-[11px] font-bold rounded-sm uppercase tracking-wide flex items-center gap-1">
            <i class="ti ti-check text-[12px]"></i> Checked today
          </span>
        </div>
      </div>"""
content = content.replace(toolbar_target, toolbar_replacement)

# 2. Add AIDescriptionGen button to products & PriceEditor
# For AIDescriptionGen: [✨ AI] purple outline 30px button
ai_btn = """
              <!-- AI Description -->
              <button onclick="generateAI(this, event)" class="w-[30px] h-[30px] rounded-brand-sm border border-[#A855F7] text-[#A855F7] hover:bg-[#A855F7]/10 flex items-center justify-center transition-colors ml-2 shrink-0" title="AI Generate Description">
                <i class="ti ti-sparkles text-[14px]"></i>
              </button>
"""

# Inject AI btn after description paragraph in the first dish
content = re.sub(r'(<p class="text-\[12px\] text-brand-text-muted truncate".*?>.*?</p>)', r'\1' + ai_btn, content)

# Price editor injection
# Instead of static price, make it click-to-edit
content = re.sub(r'<div class="text-\[14px\] sm:text-\[15px\] font-semibold text-brand-text">(\d+ ALL)</div>', r"""
              <div class="relative group/price">
                <div class="text-[14px] sm:text-[15px] font-semibold text-brand-text cursor-text border-b border-dashed border-transparent hover:border-brand-primary" onclick="editPrice(this)">\1</div>
                <input type="text" class="hidden absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[80px] h-[28px] text-center font-mono text-[13px] bg-brand-bg border border-brand-primary rounded-sm shadow-sm outline-none" value="\1" onblur="savePrice(this)" onkeydown="if(event.key === 'Enter') savePrice(this); if(event.key === 'Escape') cancelPrice(this)">
                <span class="hidden absolute top-full left-1/2 -translate-x-1/2 mt-1 text-[10px] text-semantic-success font-bold whitespace-nowrap bg-brand-surface px-1.5 py-0.5 rounded-sm">Saved ✓</span>
              </div>
""", content)

# 3. Add AllergenEditor below Dish 1
allergen_editor = """
          <!-- Allergen Editor (Expanded) -->
          <div class="bg-brand-surface rounded-brand-sm border border-brand-border p-4 mt-[-8px] mb-3 ml-[48px] sm:ml-[56px] slide-down">
            <div class="text-[12px] font-semibold text-brand-text-muted uppercase mb-2">Allergens</div>
            <div class="flex flex-wrap gap-2">
              <button class="px-3 py-1 text-[12px] font-medium rounded-full bg-brand-primary-light text-brand-primary border border-brand-primary transition-colors">Gluten</button>
              <button class="px-3 py-1 text-[12px] font-medium rounded-full bg-brand-primary-light text-brand-primary border border-brand-primary transition-colors">Seafood</button>
              <button class="px-3 py-1 text-[12px] font-medium rounded-full bg-brand-bg text-brand-text-muted border border-brand-border hover:bg-brand-surface transition-colors">Milk</button>
              <button class="px-3 py-1 text-[12px] font-medium rounded-full bg-brand-bg text-brand-text-muted border border-brand-border hover:bg-brand-surface transition-colors">Eggs</button>
              <button class="px-3 py-1 text-[12px] font-medium rounded-full bg-brand-bg text-brand-text-muted border border-brand-border hover:bg-brand-surface transition-colors">Nuts</button>
              <button class="px-3 py-1 text-[12px] font-medium rounded-full bg-brand-primary-light text-brand-primary border border-brand-primary transition-colors">Soy</button>
              <button class="px-3 py-1 text-[12px] font-medium rounded-full bg-brand-bg text-brand-text-muted border border-brand-border hover:bg-brand-surface transition-colors">Sesame</button>
              <button class="px-3 py-1 text-[12px] font-medium rounded-full bg-brand-bg text-brand-text-muted border border-brand-border hover:bg-brand-surface transition-colors">Mustard</button>
            </div>
          </div>
"""
# Insert after the first dish closing div
# Look for '</div>\n\n          <!-- Dish 2 -->'
content = content.replace('</div>\n\n          <!-- Dish 2 -->', '</div>\n' + allergen_editor + '\n          <!-- Dish 2 -->')

# 4. Make BulkEditBar visible and select 3 items (since we have 3 dishes)
content = content.replace("translate-y-full", "") # Show the bar
content = content.replace("Selected: <span class=\"font-bold\">2</span> items", "3 selected &middot; Change price &middot; Hide &middot; Move category &middot; Duplicate &middot;")
content = content.replace('<button class="h-[36px] px-4 rounded-brand-sm text-semantic-danger font-semibold text-[13px] hover:bg-semantic-danger/10 transition-colors">Hide All</button>', '')
content = content.replace('<button class="h-[36px] px-4 rounded-brand-sm bg-brand-primary text-white font-semibold text-[13px] hover:bg-brand-primary-hover transition-colors shadow-sm" onclick="document.getElementById(\'bulkEditBar\').classList.add(\'\')">Done</button>', '<button class="text-brand-text-muted hover:text-brand-text" onclick="document.getElementById(\'bulkEditBar\').style.display=\'none\'">Cancel &times;</button>')

# Pre-select checkboxes
content = content.replace('<input type="checkbox" class="sr-only peer">', '<input type="checkbox" checked class="sr-only peer">')

# 5. PositionDragHandle styles (opacity 0.3 -> 1, cursor grab -> grabbing)
style_additions = """
    .drag-handle { opacity: 0.3; cursor: grab; }
    .interactive-card:hover .drag-handle { opacity: 1; }
    .drag-handle:active { cursor: grabbing; }
"""
content = content.replace('</style>', style_additions + '\n  </style>')
content = content.replace('ti-grip-vertical text-brand-text-muted opacity-30 group-hover:opacity-100 cursor-grab transition-opacity', 'ti-grip-vertical text-brand-text-muted drag-handle transition-opacity')

# 6. JS logic for Price Editor and AI
js_logic = """
    // Price Editor Logic
    function editPrice(el) {
      el.classList.add('hidden');
      const input = el.nextElementSibling;
      input.classList.remove('hidden');
      input.dataset.oldVal = input.value;
      input.focus();
      input.select();
    }
    function savePrice(input) {
      input.classList.add('hidden');
      const display = input.previousElementSibling;
      display.textContent = input.value;
      display.classList.remove('hidden');
      
      if(input.value !== input.dataset.oldVal) {
        const toast = input.nextElementSibling;
        toast.classList.remove('hidden');
        setTimeout(() => toast.classList.add('hidden'), 1500);
      }
    }
    function cancelPrice(input) {
      input.classList.add('hidden');
      const display = input.previousElementSibling;
      input.value = input.dataset.oldVal;
      display.classList.remove('hidden');
    }

    // AI Generation Logic
    function generateAI(btn, e) {
      e.stopPropagation();
      const p = btn.previousElementSibling;
      const originalText = p.textContent;
      
      btn.innerHTML = '<i class="ti ti-loader animate-spin text-[14px]"></i>';
      btn.classList.add('opacity-50', 'cursor-not-allowed');
      
      setTimeout(() => {
        btn.innerHTML = '<i class="ti ti-sparkles text-[14px]"></i>';
        btn.classList.remove('opacity-50', 'cursor-not-allowed');
        
        const newText = "Artisan crafted roll with premium ocean-caught tuna, infused with a bold chili reduction, topped with crisp scallions and toasted sesame seeds.";
        p.textContent = '';
        
        let i = 0;
        const words = newText.split(' ');
        const interval = setInterval(() => {
          if(i < words.length) {
            p.textContent += (i > 0 ? ' ' : '') + words[i];
            i++;
          } else {
            clearInterval(interval);
          }
        }, 50);
      }, 1500);
    }
"""
content = content.replace('</script>\n</body>', js_logic + '\n  </script>\n</body>')

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Updated 07-admin-menu.html")

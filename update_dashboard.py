import re
import os

filepath = r"c:\Users\Dell5\Documents\delivery\src\screens\05-admin-dashboard.html"

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update Topbar (Busy mode, Alerts, Sound)
topbar_target = """<button class="hidden lg:flex h-[36px] px-4 items-center justify-center rounded-brand-btn border border-brand-border bg-transparent text-brand-text text-[13px] font-medium hover:bg-brand-surface transition-colors">
          ⏱ Busy mode
        </button>
        <button class="w-[36px] h-[36px] rounded-full flex items-center justify-center text-brand-text-muted hover:bg-brand-surface transition-colors relative">
          <i class="ti ti-bell text-[20px]"></i>
          <span class="absolute top-1 right-1 flex items-center justify-center min-w-[16px] h-[16px] bg-semantic-danger text-white text-[10px] font-bold rounded-full border-2 border-brand-bg">3</span>
        </button>
        <button class="w-[36px] h-[36px] rounded-full flex items-center justify-center text-brand-text-muted hover:bg-brand-surface transition-colors">
          <i class="ti ti-volume text-[20px]"></i>
        </button>"""

topbar_replacement = """<button id="busyModeToggle" onclick="toggleBusyMode()" class="hidden lg:flex h-[36px] px-4 items-center justify-center rounded-brand-btn border border-brand-border bg-transparent text-brand-text text-[13px] font-medium hover:bg-brand-surface transition-colors relative group">
          ⏱ Busy Mode
        </button>
        <div class="relative">
          <button onclick="document.getElementById('alertsDropdown').classList.toggle('hidden')" class="w-[36px] h-[36px] rounded-full flex items-center justify-center text-brand-text-muted hover:bg-brand-surface transition-colors relative">
            <i class="ti ti-bell text-[20px]"></i>
            <span class="absolute top-1 right-1 flex items-center justify-center min-w-[16px] h-[16px] bg-semantic-danger text-white text-[10px] font-bold rounded-full border-2 border-brand-bg">2</span>
          </button>
          <div id="alertsDropdown" class="absolute top-full right-0 mt-2 w-[320px] bg-brand-bg border border-brand-border rounded-brand shadow-lg hidden z-[99]">
             <div class="p-3 border-b border-brand-border flex items-center justify-between">
               <h3 class="text-[14px] font-bold text-brand-text">Alerts</h3>
             </div>
             <div class="flex flex-col">
               <div class="p-3 border-b border-brand-border flex gap-3 items-start">
                 <div class="w-2 h-2 rounded-full bg-semantic-warning mt-1.5 shrink-0"></div>
                 <div class="flex-1">
                   <p class="text-[13px] text-brand-text mb-2">Rainbow Roll has been on stop-list for 3 hours. Avg daily revenue: 1,840 ALL.</p>
                   <button class="text-[12px] font-bold text-brand-primary">Restore</button>
                 </div>
                 <button class="text-brand-text-muted hover:text-brand-text"><i class="ti ti-x"></i></button>
               </div>
               <div class="p-3 flex gap-3 items-start">
                 <div class="w-2 h-2 rounded-full bg-semantic-info mt-1.5 shrink-0"></div>
                 <div class="flex-1">
                   <p class="text-[13px] text-brand-text mb-2">Scheduled order #2296 activates at 21:45. Assign a courier in advance.</p>
                   <button class="text-[12px] font-bold text-brand-primary">Assign</button>
                 </div>
                 <button class="text-brand-text-muted hover:text-brand-text"><i class="ti ti-x"></i></button>
               </div>
             </div>
          </div>
        </div>
        <button onclick="toggleSound()" class="w-[36px] h-[36px] rounded-full flex items-center justify-center text-brand-text-muted hover:bg-brand-surface transition-colors">
          <i id="soundIcon" class="ti ti-volume text-[20px]"></i>
        </button>"""

content = content.replace(topbar_target, topbar_replacement)

# 2. Add sparklines to each QuickStat card
sparkline = """
          <svg class="w-[40px] h-[20px] mt-auto ml-auto absolute right-5 bottom-4" viewBox="0 0 100 30" preserveAspectRatio="none">
            <polyline points="0,25 15,15 30,20 45,10 60,15 75,5 100,0" fill="none" stroke="var(--brand-primary)" stroke-width="2"/>
          </svg>"""

# Find cards using regex and append relative positioning
content = re.sub(r'(<div class="bg-brand-surface shadow-sm border-transparent rounded-brand p-\[20px\] shadow-sm flex flex-col)(?! relative)', r'\1 relative', content)
content = re.sub(r'(<i class="ti ti-trending-up text-\[14px\]"></i>\s*<span>\+.*?vs yesterday</span>\s*</div>)', r'\1' + sparkline, content)

# 3. Add ScheduledOrdersSection
scheduled_section = """
            <!-- SCHEDULED SECTION -->
            <div class="mt-4 pt-4 border-t border-brand-border">
              <details class="group">
                <summary class="flex items-center justify-between text-[14px] font-medium text-brand-text cursor-pointer list-none mb-3">
                  <div class="flex items-center gap-2">
                    <i class="ti ti-calendar text-brand-primary"></i> Scheduled Today (1)
                  </div>
                  <i class="ti ti-chevron-down text-brand-text-muted transition-transform group-open:rotate-180"></i>
                </summary>
                <article class="bg-brand-surface shadow-sm border-transparent rounded-brand p-[14px] px-[16px] min-h-[110px] relative cursor-pointer border border-brand-border">
                  <div class="flex justify-between items-start mb-2.5">
                    <div class="flex items-center gap-2.5">
                      <span class="font-mono text-[13px] font-medium text-brand-text">#2296</span>
                      <span class="text-[12px] text-brand-text-muted font-bold">22:00</span>
                    </div>
                    <div class="px-2 py-0.5 bg-semantic-warning/20 text-semantic-warning text-[11px] font-bold rounded-sm uppercase tracking-wide">
                      Activates at 21:45
                    </div>
                  </div>
                  <div class="mb-3">
                    <div class="text-[13px] text-brand-text leading-relaxed">
                      Rainbow Roll ×2, Ramen ×1
                    </div>
                  </div>
                  <div class="flex items-center justify-between pt-3 border-t border-brand-border">
                    <span class="font-medium text-[15px] text-brand-text">2 520 ALL</span>
                    <button class="h-[32px] px-4 rounded-brand-sm bg-brand-surface border border-brand-border text-brand-text font-medium text-[13px] hover:bg-brand-border transition-colors">Pre-assign Courier</button>
                  </div>
                </article>
              </details>
            </div>
"""

# Insert ScheduledOrdersSection right after CARD 4: IN_DELIVERY closing </article> inside the left column
content = content.replace('</article>\n\n          </div>', '</article>' + scheduled_section + '\n          </div>')

# 4. Add JS logic
js_logic = """
    // Busy Mode Toggle
    let isBusyMode = localStorage.getItem('dos_busy_mode') === 'true';
    const busyToggle = document.getElementById('busyModeToggle');
    function applyBusyMode() {
      if (isBusyMode) {
        busyToggle.classList.add('bg-semantic-warning/20', 'text-semantic-warning', 'border-semantic-warning/30');
        busyToggle.classList.remove('bg-transparent', 'text-brand-text');
        busyToggle.setAttribute('title', '×2 timeout active');
      } else {
        busyToggle.classList.remove('bg-semantic-warning/20', 'text-semantic-warning', 'border-semantic-warning/30');
        busyToggle.classList.add('bg-transparent', 'text-brand-text');
        busyToggle.removeAttribute('title');
      }
    }
    function toggleBusyMode() {
      isBusyMode = !isBusyMode;
      localStorage.setItem('dos_busy_mode', isBusyMode);
      applyBusyMode();
    }
    applyBusyMode();

    // Sound Toggle
    let isSoundEnabled = localStorage.getItem('dos_sound_enabled') !== 'false';
    const soundIcon = document.getElementById('soundIcon');
    function applySound() {
      if (isSoundEnabled) {
        soundIcon.className = 'ti ti-volume text-[20px]';
      } else {
        soundIcon.className = 'ti ti-volume-off text-[20px]';
      }
    }
    function toggleSound() {
      // iOS audio context fix comment
      // if (!window.audioCtx) window.audioCtx = new AudioContext(); window.audioCtx.resume();
      isSoundEnabled = !isSoundEnabled;
      localStorage.setItem('dos_sound_enabled', isSoundEnabled);
      applySound();
    }
    applySound();
"""

content = content.replace("</script>\n</body>", js_logic + "\n  </script>\n</body>")

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Updated 05-admin-dashboard.html")

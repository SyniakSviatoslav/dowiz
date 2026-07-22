# PLAN — Interface Implementation

## T1: Three.js Neural Field Background
- Import Three.js from CDN via importmap
- Izhikevich particle system (2000 neurons)
- UnrealBloomPass post-processing
- Connection lines (synapses)
- Mouse interaction
- Audio sonification (spike → pentatonic)

## T2: Customer Views
- Menu with categories (pizza/pasta/salads/drinks/desserts)
- Product cards with photos, descriptions, prices
- Cart (full panel + FAB)
- Order history
- Item detail modal

## T3: Owner/Admin Views
- Dashboard (stats, revenue, orders count)
- Orders management (list with status)
- Menu management (CRUD items)
- Analytics (charts placeholder)
- Settings (restaurant info)
- Branding (theme customization)
- Promotions (discounts)
- Couriers (list)
- CRM (customer list)
- Supplies (inventory)
- Activation (plan/status)

## T4: Courier Views
- Home (online/offline status)
- Tasks (active deliveries)
- Shift (start/end)
- Earnings (daily/weekly/monthly)
- History (past deliveries)

## T5: Integration
- Bottom navigation for role switching
- Responsive (390px / 768px / 1280px)
- localStorage persistence
- Keyboard shortcuts

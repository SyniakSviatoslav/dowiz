// @ts-nocheck
import { getCart, saveCart, clearCart as clearCartStore, CartItem } from '../cart/store.js';

let currentStep = 0; // 0 = Cart, 1 = Contact, 2 = Delivery, 3 = Payment
let locationId = '';
let slug = '';
let menuData: any = null;

// Checkout State
let deliveryType = 'delivery';
let discountPercent = 0;
let tipPercent = 15;
let tipAmountCustom = 0;
let customerPhone = '69 123 456';
let customerName = '';
let deliveryAddress = '';
let deliveryInstructions = '';

document.addEventListener('DOMContentLoaded', async () => {
  const metaLocation = document.querySelector('meta[name="dos-location-id"]');
  const metaSlug = document.querySelector('meta[name="dos-slug"]');
  
  if (metaLocation) locationId = metaLocation.getAttribute('content') || '';
  if (metaSlug) slug = metaSlug.getAttribute('content') || '';

  if (!locationId) {
    document.getElementById('app')!.innerHTML = '<p>Error: No location data</p>';
    return;
  }

  const cart = getCart(locationId);
  if (cart.items.length === 0) {
    renderEmptyCart();
    return;
  }

  document.getElementById('app')!.innerHTML = `
    <div class="flex-1 flex flex-col items-center justify-center text-center py-16">
      <i class="ti ti-loader-2 spinner text-[56px] mb-4 opacity-40" style="color:var(--brand-primary); animation: spin 1s linear infinite;"></i>
      <p class="text-[14px]" style="color:var(--brand-text-muted)">Loading checkout...</p>
    </div>
  `;

  try {
    const res = await fetch(`/public/locations/${slug}/menu`);
    if (!res.ok) throw new Error('Failed to load menu');
    menuData = await res.json();
    
    // Inject styles for animations that the mockups rely on
    const style = document.createElement('style');
    style.textContent = `
      .spinner { animation: spin 1s linear infinite; }
      @keyframes spin { 100% { transform: rotate(360deg); } }
      input:focus, textarea:focus { outline: none; border-color: var(--brand-primary) !important; }
      button, .interactive-card { transition: transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1), background-color 0.2s ease, box-shadow 0.2s ease; }
      button:active, .interactive-card:active { transform: scale(0.97); }
    `;
    document.head.appendChild(style);

    renderApp();
  } catch (err) {
    // Surface the real error — this catch previously swallowed render-time
    // exceptions (e.g. schema-drift on product fields) as a misleading
    // "failed to load" message while the fetch had actually succeeded.
    console.error('[checkout] render failed:', err);
    document.getElementById('app')!.innerHTML = '<p>Something went wrong loading your cart. Please refresh.</p>';
  }
});

function getProductDetails(productId: string) {
  if (!menuData || !menuData.categories) return null;
  for (const cat of menuData.categories) {
    if (!cat.products) continue;
    const prod = cat.products.find((p: any) => p.id === productId);
    if (prod) return prod;
  }
  return null;
}

function calculateSubtotal() {
  const cart = getCart(locationId);
  let subtotal = 0;
  cart.items.forEach(item => {
    const prod = getProductDetails(item.productId);
    if (prod) {
      subtotal += prod.price * item.quantity;
    }
  });
  return subtotal;
}

function renderApp() {
  const app = document.getElementById('app')!;
  
  // Override #app container styles to match mockup
  app.className = "max-w-md mx-auto min-h-screen flex flex-col pb-28 relative";
  app.style.background = "var(--brand-bg)";

  if (currentStep === 0) {
    app.innerHTML = renderCartView();
  } else {
    app.innerHTML = renderCheckoutView();
    updateIndicators();
  }
}

// ==========================================
// CART VIEW (STEP 0)
// ==========================================
function renderCartView() {
  const cart = getCart(locationId);
  if (cart.items.length === 0) {
    setTimeout(renderEmptyCart, 0);
    return '';
  }

  let subtotal = calculateSubtotal();
  const deliveryFee = 200; // Hardcoded for now
  const discount = Math.round(subtotal * discountPercent / 100);
  const finalTotal = subtotal + deliveryFee - discount;

  let itemsHtml = '<div class="flex flex-col mb-6">';
  cart.items.forEach((item, idx) => {
    const prod = getProductDetails(item.productId);
    if (!prod) return;
    // /public/locations/:slug/menu returns flat, single-locale products
    // (name/description), NOT the all-locales available_names shape.
    const name = prod.name || 'Unknown Product';
    const desc = prod.description || '';
    
    itemsHtml += `
      <div class="py-4 border-b flex gap-3 last:border-0" style="border-color:var(--brand-border)">
        <div class="w-[56px] h-[56px] rounded-[8px] shrink-0 border flex items-center justify-center bg-cover bg-center" style="border-color:var(--brand-border); background-image: url('${prod.imageUrl || '/images/' + prod.image_key}')">
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex justify-between items-start mb-1">
            <h3 class="text-[14px] font-medium leading-snug pr-4" style="color:var(--brand-text)">${name}</h3>
            <button onclick="window.DowizCheckout.removeItem(${idx})" class="shrink-0 -mt-1 -mr-2 p-1 active:scale-90 transition-transform" style="color:var(--brand-text-muted)">
              <i class="ti ti-x text-[18px]"></i>
            </button>
          </div>
          <p class="text-[12px] truncate mb-2" style="color:var(--brand-text-muted)">${desc}</p>
          <div class="flex items-center justify-between mt-auto gap-2">
            <div class="flex items-center gap-1 sm:gap-3 rounded-full border px-1 py-1" style="background:var(--brand-surface-raised);border-color:var(--brand-border)">
              <button onclick="window.DowizCheckout.updateQty(${idx}, -1)" class="min-w-[44px] min-h-[44px] p-2 rounded-full flex items-center justify-center active:scale-90 transition-all" style="color:var(--brand-text)"><i class="ti ti-minus text-[14px]"></i></button>
              <span class="text-[14px] font-medium w-4 text-center" style="color:var(--brand-text)">${item.quantity}</span>
              <button onclick="window.DowizCheckout.updateQty(${idx}, 1)" class="min-w-[44px] min-h-[44px] p-2 rounded-full flex items-center justify-center active:scale-90 transition-all" style="color:var(--brand-text)"><i class="ti ti-plus text-[14px]"></i></button>
            </div>
            <div class="text-right">
              <div class="text-[14px] font-bold" style="color:var(--brand-primary)">${prod.price * item.quantity} ALL</div>
            </div>
          </div>
        </div>
      </div>
    `;
  });
  itemsHtml += '</div>';

  return `
    <header class="sticky top-0 z-50 h-[56px] border-b flex items-center justify-between px-4" style="background:var(--brand-surface);border-color:var(--brand-border)">
      <a href="/s/${slug}" class="min-w-[44px] min-h-[44px] -ml-2 rounded-full flex items-center justify-center transition-all active:scale-[0.97]" style="color:var(--brand-text)">
        <i class="ti ti-arrow-left text-[20px]"></i>
      </a>
      <h1 class="text-[16px] font-bold" style="color:var(--brand-text)">Cart</h1>
      <button onclick="window.DowizCheckout.clearCart()" class="text-[13px] font-medium px-2 transition-colors ml-auto" style="color:var(--brand-text-muted)">Clear all</button>
    </header>

    <main class="flex-1 px-4 pt-4 flex flex-col">
      ${itemsHtml}
      <div class="rounded-[12px] p-4 mb-8 border" style="background:var(--brand-surface);border-color:var(--brand-border)">
        <div class="flex justify-between text-[14px] mb-2" style="color:var(--brand-text)">
          <span>Subtotal</span><span>${subtotal} ALL</span>
        </div>
        <div class="flex justify-between text-[14px] mb-2" style="color:var(--brand-text)">
          <span>Delivery</span><span>${deliveryFee} ALL</span>
        </div>
        <div class="h-px w-full my-3" style="background:var(--brand-border)"></div>
        <div class="flex justify-between items-center">
          <span class="text-[16px] font-bold" style="color:var(--brand-text)">Total</span>
          <div class="text-right">
            <span class="text-[16px] font-bold block" style="color:var(--brand-primary)">${finalTotal} ALL</span>
          </div>
        </div>
      </div>
    </main>

    <div class="fixed bottom-0 left-0 right-0 max-w-md mx-auto border-t p-3 sm:p-4 z-40" style="background:var(--brand-surface);border-color:var(--brand-border)">
      <button onclick="window.DowizCheckout.setStep(1)" class="w-full h-[52px] flex items-center justify-between px-6 text-white font-medium active:scale-[0.97] transition-all" style="background:var(--brand-primary);border-radius:var(--brand-radius-btn)">
        <span class="text-[16px]">Checkout</span>
        <span class="text-[16px] font-bold">${finalTotal} ALL</span>
      </button>
    </div>
  `;
}

function renderEmptyCart() {
  const app = document.getElementById('app')!;
  app.className = "max-w-md mx-auto min-h-screen flex flex-col";
  app.innerHTML = `
    <header class="sticky top-0 z-50 h-[56px] border-b flex items-center justify-between px-4" style="background:var(--brand-surface);border-color:var(--brand-border)">
      <a href="/s/${slug}" class="min-w-[44px] min-h-[44px] -ml-2 rounded-full flex items-center justify-center transition-all active:scale-[0.97]" style="color:var(--brand-text)">
        <i class="ti ti-arrow-left text-[20px]"></i>
      </a>
      <h1 class="text-[16px] font-bold" style="color:var(--brand-text)">Cart</h1>
      <div class="w-[44px]"></div>
    </header>
    <div class="flex-1 flex flex-col items-center justify-center text-center py-16">
      <i class="ti ti-shopping-bag text-[56px] mb-4 opacity-40" style="color:var(--brand-text-muted)"></i>
      <h2 class="text-[18px] font-semibold mb-1" style="color:var(--brand-text)">Your cart is empty</h2>
      <p class="text-[14px] mb-6" style="color:var(--brand-text-muted)">Add items from the menu to get started</p>
      <a href="/s/${slug}" class="px-6 h-[44px] text-white font-medium flex items-center justify-center active:scale-[0.97] transition-all" style="background:var(--brand-primary);border-radius:var(--brand-radius-btn)">Browse Menu</a>
    </div>
  `;
}

// ==========================================
// CHECKOUT VIEW (STEPS 1, 2, 3)
// ==========================================
function renderCheckoutView() {
  let content = '';
  
  if (currentStep === 1) {
    content = `
      <h2 class="text-[20px] font-semibold mb-6" style="color:var(--brand-text);font-family:var(--brand-font-heading)">Contact info</h2>
      <div class="space-y-4 mb-8">
        <div>
          <label class="block text-[13px] font-medium mb-1" style="color:var(--brand-text)">Your phone number</label>
          <div class="relative flex items-center">
            <div class="absolute left-3 flex items-center gap-1 border-r pr-2" style="border-color:var(--brand-border)">
              <span class="text-[16px]">🇦🇱</span>
              <span class="text-[14px]" style="color:var(--brand-text-muted)">+355</span>
            </div>
            <input type="tel" id="contactPhone" value="${customerPhone}" placeholder="69 123 4567" class="w-full h-[48px] pl-[84px] pr-4 border transition-colors text-[14px] font-medium" style="background:var(--brand-surface-raised);border-color:var(--brand-border);border-radius:var(--brand-radius-sm);color:var(--brand-text)">
          </div>
        </div>
        <div>
          <label class="block text-[13px] font-medium mb-1" style="color:var(--brand-text)">Name <span class="font-normal" style="color:var(--brand-text-muted)">(optional)</span></label>
          <input type="text" id="contactName" value="${customerName}" placeholder="John Doe" class="w-full h-[48px] px-4 border transition-colors text-[14px] font-medium" style="background:var(--brand-surface-raised);border-color:var(--brand-border);border-radius:var(--brand-radius-sm);color:var(--brand-text)">
        </div>
      </div>
      <button onclick="window.DowizCheckout.setStep(2)" class="w-full h-[48px] text-white font-medium text-[15px] active:scale-[0.97] transition-all" style="background:var(--brand-primary);border-radius:var(--brand-radius-btn)">
        Continue
      </button>
    `;
  } else if (currentStep === 2) {
    const btnStyle = (type: string) => deliveryType === type
          ? `background:var(--brand-surface-raised);color:var(--brand-text);font-weight:600`
          : `background:transparent;color:var(--brand-text-muted)`;

    let typeContent = '';
    if (deliveryType === 'delivery') {
      typeContent = `
        <div class="mt-6">
          <label class="block text-[13px] font-medium mb-1" style="color:var(--brand-text)">Delivery address</label>
          <div class="relative flex items-center mb-2">
            <i class="ti ti-map-pin absolute left-4 text-[18px]" style="color:var(--brand-text-muted)"></i>
            <input type="text" id="deliveryAddress" value="${deliveryAddress}" placeholder="Enter full address..." class="w-full h-[48px] pl-10 pr-4 border text-[14px] font-medium" style="background:var(--brand-surface-raised);border-color:var(--brand-border);border-radius:var(--brand-radius-sm);color:var(--brand-text)">
          </div>
          <textarea id="deliveryInstructions" placeholder="E.g., Leave at door, ring bell, building 5, apartment 14..." maxlength="300" class="w-full h-[52px] border rounded-[12px] px-4 py-3 text-[14px] resize-none outline-none transition-colors mb-4" style="background:var(--brand-surface-raised);border-color:var(--brand-border);color:var(--brand-text)">${deliveryInstructions}</textarea>
          <div class="border rounded-[8px] p-3 flex items-center justify-between text-[13px] font-medium" style="background:var(--brand-surface);border-color:var(--brand-border)">
            <span style="color:var(--brand-text)">Delivery fee</span>
            <span style="color:var(--brand-primary)">200 ALL</span>
          </div>
        </div>
      `;
    } else {
      typeContent = `
        <div class="mt-6 border rounded-[12px] p-4" style="background:var(--brand-surface);border-color:var(--brand-border)">
          <h3 class="text-[14px] font-bold mb-1" style="color:var(--brand-text)">Pickup address</h3>
          <p class="text-[14px] mb-4" style="color:var(--brand-text-muted)">${menuData.location_name || ''}</p>
        </div>
      `;
    }

    content = `
      <h2 class="text-[20px] font-semibold mb-6" style="color:var(--brand-text);font-family:var(--brand-font-heading)">Receiving order</h2>
      <div class="flex p-1 rounded-[10px] mb-2 gap-0.5" style="background:var(--brand-surface);border-color:var(--brand-border)">
        <button onclick="window.DowizCheckout.changeDeliveryType('delivery')" class="flex-1 py-2 text-[13px] font-medium rounded-[8px] transition-all" style="${btnStyle('delivery')}">Delivery</button>
        <button onclick="window.DowizCheckout.changeDeliveryType('pickup')" class="flex-1 py-2 text-[13px] font-medium rounded-[8px] transition-all" style="${btnStyle('pickup')}">Pickup</button>
      </div>
      ${typeContent}
      <div class="mt-8">
        <button onclick="window.DowizCheckout.setStep(3)" class="w-full h-[48px] text-white font-medium text-[15px] active:scale-[0.97] transition-all" style="background:var(--brand-primary);border-radius:var(--brand-radius-btn)">
          Continue to Payment
        </button>
      </div>
    `;
  } else if (currentStep === 3) {
    const subtotal = calculateSubtotal();
    const deliveryFee = deliveryType === 'delivery' ? 200 : 0;
    const discount = Math.round(subtotal * discountPercent / 100);
    const tipAmount = tipPercent > 0 ? Math.round(subtotal * tipPercent / 100) : tipAmountCustom;
    const total = subtotal + deliveryFee - discount + tipAmount;

    content = `
      <h2 class="text-[20px] font-semibold mb-6" style="color:var(--brand-text);font-family:var(--brand-font-heading)">Review &amp; Pay</h2>
      
      <div class="border rounded-[12px] mb-6 overflow-hidden" style="background:var(--brand-surface);border-color:var(--brand-border)">
        <div class="px-4 py-4 text-[13px]" style="background:var(--brand-surface-raised);border-color:var(--brand-border)">
          <div class="flex justify-between mb-1 text-[12px]" style="color:var(--brand-text-muted)"><span>Subtotal</span><span>${subtotal} ALL</span></div>
          <div class="flex justify-between mb-1 text-[12px]" style="color:var(--brand-text-muted)"><span>Delivery fee</span><span>${deliveryFee} ALL</span></div>
          ${tipAmount > 0 ? `<div class="flex justify-between mb-1 text-[12px]" style="color:var(--brand-text-muted)"><span>Tip</span><span>${tipAmount} ALL</span></div>` : ''}
          <div class="flex justify-between font-bold pt-2 border-t mt-2" style="border-color:var(--brand-border);color:var(--brand-text)">
            <span>Total</span><span class="text-[15px]" style="color:var(--brand-primary)">${total} ALL</span>
          </div>
        </div>
      </div>

      <h3 class="text-[14px] font-bold mb-3" style="color:var(--brand-text)">Payment Method</h3>
      <div class="space-y-3 mb-6">
        <label class="flex items-center justify-between p-4 rounded-[12px] border-2 cursor-pointer" style="border-color:var(--brand-primary);background:var(--brand-primary-light)">
          <div class="flex items-center gap-3">
            <i class="ti ti-cash text-[24px]" style="color:var(--brand-primary)"></i>
            <span class="text-[14px] font-semibold" style="color:var(--brand-text)">Cash on ${deliveryType}</span>
          </div>
          <div class="w-5 h-5 rounded-full text-white flex items-center justify-center" style="background:var(--brand-primary)">
            <i class="ti ti-check text-[12px]"></i>
          </div>
        </label>
        <div class="flex items-center justify-between p-4 rounded-[12px] border opacity-50 cursor-not-allowed" style="border-color:var(--brand-border);background:var(--brand-surface)">
          <div class="flex items-center gap-3">
            <i class="ti ti-credit-card text-[24px]" style="color:var(--brand-text-muted)"></i>
            <span class="text-[14px] font-medium" style="color:var(--brand-text-muted)">Card (Visa/Mastercard)</span>
          </div>
          <span class="text-[10px] px-2 py-0.5 rounded-[4px] font-bold uppercase tracking-wider" style="background:var(--brand-surface-raised);color:var(--brand-text-muted)">Soon</span>
        </div>
      </div>

      <div class="mt-8">
        <button id="confirm-btn" onclick="window.DowizCheckout.confirmOrder()" class="w-full h-[52px] flex items-center justify-center text-white font-medium text-[16px] active:scale-[0.97] transition-all" style="background:var(--brand-primary);border-radius:var(--brand-radius-btn)">
          Confirm Order \u00b7 ${total} ALL
        </button>
      </div>
    `;
  }

  return `
    <header class="sticky top-0 z-50 border-b px-4 py-3" style="background:var(--brand-surface);border-color:var(--brand-border)">
      <div class="flex items-center mb-4">
        <button onclick="window.DowizCheckout.goBack()" class="min-w-[44px] min-h-[44px] -ml-2 rounded-full flex items-center justify-center transition-all active:scale-[0.97]" style="color:var(--brand-text)">
          <i class="ti ti-arrow-left text-[20px]"></i>
        </button>
        <h1 class="text-[16px] font-bold ml-1" style="color:var(--brand-text)">Checkout</h1>
      </div>

      <div class="flex items-center justify-between relative px-2">
        <div class="absolute top-1/2 left-6 right-6 h-[2px] -z-10 -translate-y-1/2" style="background:var(--brand-border)"></div>
        <div class="flex flex-col items-center gap-1 z-10 px-2" id="step1-indicator" style="background:var(--brand-surface)">
          <div class="w-6 h-6 rounded-full text-white flex items-center justify-center text-[12px] font-bold transition-colors" style="background:var(--brand-primary)">1</div>
          <span class="text-[10px] font-semibold" style="color:var(--brand-primary)">Contact</span>
        </div>
        <div class="flex flex-col items-center gap-1 z-10 px-2" id="step2-indicator" style="background:var(--brand-surface)">
          <div class="w-6 h-6 rounded-full border-2 flex items-center justify-center text-[12px] font-bold transition-colors" style="border-color:var(--brand-border);background:var(--brand-surface);color:var(--brand-text-muted)">2</div>
          <span class="text-[10px] font-semibold" style="color:var(--brand-text-muted)">Delivery</span>
        </div>
        <div class="flex flex-col items-center gap-1 z-10 px-2" id="step3-indicator" style="background:var(--brand-surface)">
          <div class="w-6 h-6 rounded-full border-2 flex items-center justify-center text-[12px] font-bold transition-colors" style="border-color:var(--brand-border);background:var(--brand-surface);color:var(--brand-text-muted)">3</div>
          <span class="text-[10px] font-semibold" style="color:var(--brand-text-muted)">Payment</span>
        </div>
      </div>
    </header>
    <main class="flex-1 px-4 py-6">${content}</main>
  `;
}

function updateIndicators() {
  if (currentStep === 0) return;
  const s1 = document.getElementById('step1-indicator')!;
  const s2 = document.getElementById('step2-indicator')!;
  const s3 = document.getElementById('step3-indicator')!;

  [s1, s2, s3].forEach(el => {
    (el.children[0] as HTMLElement).style.background = 'var(--brand-surface)';
    (el.children[0] as HTMLElement).style.borderWidth = '2px';
    (el.children[0] as HTMLElement).style.borderStyle = 'solid';
    (el.children[0] as HTMLElement).style.borderColor = 'var(--brand-border)';
    (el.children[0] as HTMLElement).style.color = 'var(--brand-text-muted)';
    (el.children[1] as HTMLElement).style.color = 'var(--brand-text-muted)';
    el.children[0].innerHTML = el.children[0].textContent!.trim();
  });

  if (currentStep > 1) {
    (s1.children[0] as HTMLElement).style.background = 'var(--color-success)';
    (s1.children[0] as HTMLElement).style.borderColor = 'var(--color-success)';
    (s1.children[0] as HTMLElement).style.color = '#fff';
    s1.children[0].innerHTML = '<i class="ti ti-check text-[14px]"></i>';
    (s1.children[1] as HTMLElement).style.color = 'var(--color-success)';
  } else if (currentStep === 1) {
    (s1.children[0] as HTMLElement).style.background = 'var(--brand-primary)';
    (s1.children[0] as HTMLElement).style.borderColor = 'var(--brand-primary)';
    (s1.children[0] as HTMLElement).style.color = '#fff';
    (s1.children[1] as HTMLElement).style.color = 'var(--brand-primary)';
  }

  if (currentStep > 2) {
    (s2.children[0] as HTMLElement).style.background = 'var(--color-success)';
    (s2.children[0] as HTMLElement).style.borderColor = 'var(--color-success)';
    (s2.children[0] as HTMLElement).style.color = '#fff';
    s2.children[0].innerHTML = '<i class="ti ti-check text-[14px]"></i>';
    (s2.children[1] as HTMLElement).style.color = 'var(--color-success)';
  } else if (currentStep === 2) {
    (s2.children[0] as HTMLElement).style.background = 'var(--brand-primary)';
    (s2.children[0] as HTMLElement).style.borderColor = 'var(--brand-primary)';
    (s2.children[0] as HTMLElement).style.color = '#fff';
    (s2.children[1] as HTMLElement).style.color = 'var(--brand-primary)';
  }

  if (currentStep === 3) {
    (s3.children[0] as HTMLElement).style.background = 'var(--brand-primary)';
    (s3.children[0] as HTMLElement).style.borderColor = 'var(--brand-primary)';
    (s3.children[0] as HTMLElement).style.color = '#fff';
    (s3.children[1] as HTMLElement).style.color = 'var(--brand-primary)';
  }
}

// ==========================================
// ACTIONS
// ==========================================

function goBack() {
  if (currentStep > 0) {
    if (currentStep === 1) {
      customerPhone = (document.getElementById('contactPhone') as HTMLInputElement)?.value || customerPhone;
      customerName = (document.getElementById('contactName') as HTMLInputElement)?.value || customerName;
    } else if (currentStep === 2 && deliveryType === 'delivery') {
      deliveryAddress = (document.getElementById('deliveryAddress') as HTMLInputElement)?.value || deliveryAddress;
      deliveryInstructions = (document.getElementById('deliveryInstructions') as HTMLTextAreaElement)?.value || deliveryInstructions;
    }
    currentStep--;
    renderApp();
  } else {
    window.location.href = `/s/${slug}`;
  }
}

function setStep(step: number) {
  if (currentStep === 1) {
    const phoneEl = document.getElementById('contactPhone') as HTMLInputElement;
    if (phoneEl) customerPhone = phoneEl.value;
    const nameEl = document.getElementById('contactName') as HTMLInputElement;
    if (nameEl) customerName = nameEl.value;
    
    if (step === 2 && !customerPhone) {
      alert("Please enter a phone number");
      return;
    }
  }
  
  if (currentStep === 2 && deliveryType === 'delivery') {
    const addrEl = document.getElementById('deliveryAddress') as HTMLInputElement;
    if (addrEl) deliveryAddress = addrEl.value;
    const instEl = document.getElementById('deliveryInstructions') as HTMLTextAreaElement;
    if (instEl) deliveryInstructions = instEl.value;
    
    if (step === 3 && !deliveryAddress) {
      alert("Please enter a delivery address");
      return;
    }
  }

  currentStep = step;
  renderApp();
}

function updateQty(index: number, change: number) {
  if (navigator.vibrate) navigator.vibrate(10);
  let cart = getCart(locationId);
  cart.items[index].quantity += change;
  if (cart.items[index].quantity <= 0) cart.items.splice(index, 1);
  saveCart(locationId, cart);
  renderApp();
}

function removeItem(index: number) {
  let cart = getCart(locationId);
  cart.items.splice(index, 1);
  saveCart(locationId, cart);
  renderApp();
}

function clearCart() {
  clearCartStore(locationId);
  renderApp();
}

function changeDeliveryType(type: string) {
  deliveryType = type;
  renderApp();
}

async function confirmOrder() {
  const btn = document.getElementById('confirm-btn') as HTMLButtonElement;
  btn.innerHTML = '<i class="ti ti-loader-2 spinner text-[20px]"></i>';
  btn.disabled = true;

  const cart = getCart(locationId);

  const orderData = {
    locationId,
    type: 'delivery',
    items: cart.items.map(i => ({
      product_id: i.productId,
      quantity: i.quantity,
      modifier_ids: i.modifierIds
    })),
    customer: {
      phone: '+355' + customerPhone.replace(/\D/g, ''),
      name: customerName || undefined
    },
    delivery: {
      pin: { lat: 41.3275, lng: 19.8187 },
      address_text: deliveryAddress || 'No address provided'
    },
    payment: {
      method: 'cash'
    },
    idempotency_key: crypto.randomUUID()
  };

  try {
    const res = await fetch('/api/orders', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(orderData)
    });

    if (!res.ok) {
      throw new Error('Failed to create order');
    }
    
    const json = await res.json();
    clearCartStore(locationId);
    window.location.href = `/s/${slug}/orders/${json.id}`;

  } catch (err) {
    console.error(err);
    alert('Failed to place order.');
    btn.innerHTML = 'Confirm Order';
    btn.disabled = false;
  }
}

// Expose handlers to window for inline HTML onclicks
(window as any).DowizCheckout = {
  goBack,
  setStep,
  updateQty,
  removeItem,
  clearCart,
  changeDeliveryType,
  confirmOrder
};

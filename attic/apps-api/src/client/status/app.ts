// @ts-nocheck
let locationId = '';
let slug = '';
let orderId = '';
let orderData: any = null;
let currentThemeIdx = 0;

document.addEventListener('DOMContentLoaded', async () => {
  const metaLocation = document.querySelector('meta[name="dos-location-id"]');
  const metaSlug = document.querySelector('meta[name="dos-slug"]');
  
  if (metaLocation) locationId = metaLocation.getAttribute('content') || '';
  if (metaSlug) slug = metaSlug.getAttribute('content') || '';
  
  // Extract orderId from URL: /s/:slug/orders/:orderId
  const pathParts = window.location.pathname.split('/');
  orderId = pathParts[pathParts.length - 1];

  if (!orderId) {
    document.getElementById('app')!.innerHTML = '<p>Error: No order ID found</p>';
    return;
  }

  document.getElementById('app')!.innerHTML = `
    <div class="flex-1 flex flex-col items-center justify-center text-center py-16">
      <i class="ti ti-loader-2 spinner text-[56px] mb-4 opacity-40" style="color:var(--brand-primary); animation: spin 1s linear infinite;"></i>
      <p class="text-[14px]" style="color:var(--brand-text-muted)">Loading your order...</p>
    </div>
  `;

  // Inject styles
  const style = document.createElement('style');
  style.textContent = `
    .spinner { animation: spin 1s linear infinite; }
    @keyframes spin { 100% { transform: rotate(360deg); } }
    button, .interactive-card { transition: transform 0.15s cubic-bezier(0.34, 1.56, 0.64, 1), background-color 0.2s ease; }
    button:active { transform: scale(0.97); }
    .pulse-dot { animation: pulse-ring 2s infinite; }
    @keyframes pulse-ring {
      0% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--brand-primary) 50%, transparent); }
      70% { box-shadow: 0 0 0 10px color-mix(in srgb, var(--brand-primary) 0%, transparent); }
      100% { box-shadow: 0 0 0 0 color-mix(in srgb, var(--brand-primary) 0%, transparent); }
    }
    .draw-line { stroke-dasharray: 1000; stroke-dashoffset: 1000; animation: dash 3s ease-in-out forwards; }
    @keyframes dash { to { stroke-dashoffset: 0; } }
    details summary::-webkit-details-marker { display: none; }
  `;
  document.head.appendChild(style);

  await fetchOrderData();
  
  // Poll for updates every 10 seconds
  setInterval(fetchOrderData, 10000);
});

async function fetchOrderData() {
  try {
    const res = await fetch(`/api/orders/${orderId}`);
    if (!res.ok) throw new Error('Failed to load order');
    orderData = await res.json();
    renderApp();
  } catch (err) {
    if (!orderData) {
      document.getElementById('app')!.innerHTML = '<p class="text-center py-10">Error loading order. Please refresh.</p>';
    }
  }
}

function renderApp() {
  const app = document.getElementById('app')!;
  app.className = "max-w-md mx-auto min-h-screen relative pb-28";
  
  // Status definitions
  const STATUSES = ['PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED'];
  const currentIndex = STATUSES.indexOf(orderData.status);
  const isCancelled = orderData.status === 'CANCELLED' || orderData.status === 'REJECTED';
  
  let statusText = "Pending Confirmation";
  if (orderData.status === 'CONFIRMED') statusText = "Order Confirmed";
  if (orderData.status === 'PREPARING') statusText = "Preparing Food";
  if (orderData.status === 'READY') statusText = "Ready for Pickup/Delivery";
  if (orderData.status === 'IN_DELIVERY') statusText = "Courier is on the way";
  if (orderData.status === 'DELIVERED') statusText = "Delivered";
  if (isCancelled) statusText = orderData.status;

  // Timeline UI
  let timelineHtml = '<div class="flex justify-between relative z-10 px-2">';
  for (let i = 0; i < 5; i++) {
    let icon = 'ti-check';
    let bg = 'var(--brand-surface)';
    let color = 'var(--brand-text-muted)';
    let border = '2px solid var(--brand-border)';
    let extraClass = '';
    
    if (i < currentIndex) {
      bg = 'var(--color-success)';
      color = 'white';
      border = 'none';
    } else if (i === currentIndex && !isCancelled) {
      bg = 'var(--brand-primary)';
      color = 'white';
      border = 'none';
      extraClass = 'shadow-md pulse-dot';
      icon = i === 4 ? 'ti-home' : (i === 3 ? 'ti-bike' : 'ti-loader');
    } else if (isCancelled) {
      bg = 'var(--color-danger)';
      color = 'white';
      border = 'none';
      icon = 'ti-x';
    } else {
      if (i === 4) icon = 'ti-home';
      else if (i === 3) icon = 'ti-bike';
      else icon = 'ti-point';
    }

    timelineHtml += `
      <div class="flex flex-col items-center gap-2 relative">
        <div class="w-8 h-8 rounded-full flex items-center justify-center ${extraClass}" style="background:${bg}; color:${color}; border:${border}">
          <i class="ti ${icon} text-[16px]"></i>
        </div>
      </div>
    `;
  }
  timelineHtml += '</div>';

  let itemsHtml = '';
  if (orderData.items && orderData.items.length > 0) {
    orderData.items.forEach((item: any) => {
      itemsHtml += `
        <div class="flex justify-between mb-2 text-[13px]" style="color:var(--brand-text)">
          <span>${item.nameSnapshot || 'Product'} ×${item.quantity}</span>
          <span>${item.priceSnapshot * item.quantity} ALL</span>
        </div>
      `;
    });
  }

  app.innerHTML = `
    <header class="sticky top-0 z-50 h-[56px] border-b flex items-center px-4" style="background:var(--brand-surface);border-color:var(--brand-border)">
      <a href="/s/${slug}" class="min-w-[44px] min-h-[44px] -ml-2 rounded-full flex items-center justify-center transition-all active:scale-[0.97]" style="color:var(--brand-text)">
        <i class="ti ti-arrow-left text-[20px]"></i>
      </a>
      <h1 class="text-[18px] font-bold" style="color:var(--brand-text);font-family:var(--brand-font-heading)">Order Status</h1>
      <span class="ml-2 text-[12px] mt-1 font-mono" style="color:var(--brand-text-muted)">#${orderData.id.split('-')[0]}</span>
    </header>

    <main class="px-4 pt-6">
      <div class="text-center mb-6">
        <p class="text-[12px] font-medium uppercase tracking-widest mb-1" style="color:var(--brand-text-muted)">Current Status</p>
        <h2 class="text-[28px] font-semibold leading-none" style="color:var(--brand-primary);font-family:var(--brand-font-heading)">${statusText}</h2>
      </div>

      <div class="mb-8 overflow-x-auto hide-scrollbar relative -mx-4 px-4">
        <div class="relative" style="min-width:460px">
          <div class="absolute top-4 left-4 right-4 h-1 z-0" style="background:var(--brand-border)"></div>
          <div class="absolute top-4 left-4 h-1 z-0 transition-all duration-1000" style="width:${Math.max(0, currentIndex * 25)}%;background:var(--brand-primary)"></div>
          ${timelineHtml}
        </div>
      </div>

      <div class="border rounded-[12px] mb-6 overflow-hidden" style="background:var(--brand-surface);border-color:var(--brand-border)">
        <details class="group" open>
          <summary class="flex items-center justify-between p-4 cursor-pointer list-none font-medium text-[14px]" style="color:var(--brand-text)">
            <span>Order Summary</span>
            <i class="ti ti-chevron-down transition-transform group-open:rotate-180" style="color:var(--brand-text-muted)"></i>
          </summary>
          <div class="px-4 pb-4 border-t pt-3" style="background:var(--brand-surface-raised);border-color:var(--brand-border)">
            ${itemsHtml}
            <div class="flex justify-between font-bold pt-3 border-t mt-2" style="border-color:var(--brand-border);color:var(--brand-text)">
              <span>Total</span><span style="color:var(--brand-primary)">${orderData.total} ALL</span>
            </div>
            <div class="text-[12px] mt-2" style="color:var(--brand-text-muted)">
               Payment Method: ${orderData.paymentMethod}
            </div>
          </div>
        </details>
      </div>

      ${orderData.status === 'DELIVERED' ? `
        <div id="feedback-form" class="border rounded-[12px] p-5 mb-6 transition-opacity duration-500" style="background:var(--brand-surface);border-color:var(--brand-border)">
          <h3 class="text-[20px] font-semibold mb-4 text-center" style="color:var(--brand-text);font-family:var(--brand-font-heading)">How was your experience?</h3>
          <textarea placeholder="Leave a comment (optional)..." class="w-full h-20 p-3 border text-[13px] outline-none resize-none mb-4" style="background:var(--brand-surface-raised);border-color:var(--brand-border);border-radius:var(--brand-radius-sm);color:var(--brand-text)"></textarea>
          <button onclick="window.DowizStatus.submitFeedback()" class="w-full h-[44px] text-white font-medium text-[14px] active:scale-[0.97] transition-all" style="background:var(--brand-primary);border-radius:var(--brand-radius-btn)">
            Submit Rating
          </button>
        </div>
      ` : ''}
    </main>
  `;
}

function submitFeedback() {
  alert('Thank you for your feedback!');
}

(window as any).DowizStatus = {
  submitFeedback
};

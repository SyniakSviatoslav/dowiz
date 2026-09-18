// Profile — Figma node 1:9321 (dark) / 1:19760 (light).
//
// Eleven rows and a portrait. The rows are DATA because nine other screens in
// the kit are the same list with different entries, and because a row whose
// destination is written into its markup is a row that outlives the screen it
// points at.

import { icon, esc } from '/kit/app.js';
import { topBar, plate, menuList, navbar } from '/kit/parts.js';

const ME = { name: 'Jennifer Aaker' };

const ROWS = [
  { icon: 'user',            label: 'Your profile',    to: 'your-profile' },
  { icon: 'pin-24',          label: 'Manager Address', to: 'manage-address' },
  { icon: 'card',            label: 'Payment Methods', to: 'payment-methods' },
  { icon: 'calendar',        label: 'My Bookings',     to: 'my-booking' },
  { icon: 'clipboard-text',  label: 'My Orders',       to: 'my-orders' },
  { icon: 'ticket',          label: 'My Coupons',      to: 'coupon' },
  { icon: 'wallet',          label: 'My Wallet',       to: 'my-wallet' },
  { icon: 'setting',         label: 'Settings',        to: 'settings' },
  { icon: 'info-circle',     label: 'Help Center',     to: 'help-faq' },
  { icon: 'lock',            label: 'Privacy Policy',  to: 'privacy-policy' },
  { icon: 'logout',          label: 'Log out',         to: 'logout', tone: 'danger' },
];

export function render(){
  return `
  ${topBar('Profile')}

  <div class="wrap">
    <div class="k-me">
      <div class="k-me-photo">
        ${plate(ME.name)}
        <img class="k-me-img" id="meImg" alt="" hidden>
        <button class="k-me-edit" type="button" data-go="your-profile"
                aria-label="Змінити фото">${icon('edit-2')}</button>
      </div>
      <h2 class="k-me-name">${esc(ME.name)}</h2>
    </div>

    <div class="k-block">${menuList(ROWS)}</div>
  </div>

  ${navbar('profile')}`;
}

export function bind(root){
  // The portrait chosen on Your Profile is kept on this device, so this screen
  // shows it too: two screens drawing the same person with two different faces
  // is how a customer stops believing either of them.
  showAvatar(root);
}

function showAvatar(root){
  const img = root.querySelector('#meImg');
  if (!img) return;
  try {
    const saved = localStorage.getItem('dw_kit_avatar');
    if (saved){ img.src = saved; img.hidden = false; }
  } catch { /* private window: the plate stands */ }
}

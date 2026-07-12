// push.js — courier out-of-app dispatch push subscription helper.
//
// Pairs with public/sw.js (the courier push handler) so couriers receive
// dispatch alerts even when the app/phone is locked or backgrounded — closing
// the gap documented in the roadmap (legacy React tracking UI had no
// service-worker push path). Plain JS module, no deps.
//
// This helper does NOT contain any courier scoring/rating logic (forbidden by
// the kernel guard in kernel/src/domain.rs). It only requests notification
// permission and subscribes to web-push.

// TODO(real-key): replace with the deployment-injected VAPID public key.
// This placeholder is intentionally invalid so a subscription can never
// silently "succeed" against a missing server key.
const VAPID_APP_PUBLIC_KEY = 'TODO_REPLACE_WITH_REAL_VAPID_PUBLIC_KEY';

// Register the courier push Service Worker (public/sw.js).
// Throws if Service Workers are unsupported in this context.
export async function registerCourierServiceWorker() {
  if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) {
    throw new Error('serviceWorker unsupported');
  }
  // '/sw.js' is served from web root; must be reachable (e.g. web/public/sw.js).
  return navigator.serviceWorker.register('/sw.js');
}

// Request notification permission + subscribe via pushManager, then POST the
// subscription JSON to the backend so it can target this courier.
//
// No server is built in this wave — the POST endpoint below is scaffolded with
// a TODO. The call is structured exactly as production will use it.
export async function registerCourierPush() {
  if (typeof navigator === 'undefined' || !('serviceWorker' in navigator)) {
    throw new Error('Web Push unsupported: serviceWorker missing');
  }
  if (typeof window === 'undefined' || !('PushManager' in window)) {
    throw new Error('Web Push unsupported: PushManager missing');
  }
  if (!('Notification' in window)) {
    throw new Error('Notifications unsupported in this browser');
  }

  if (VAPID_APP_PUBLIC_KEY.startsWith('TODO')) {
    throw new Error(
      'VAPID_APP_PUBLIC_KEY is a placeholder — set the real key before subscribing'
    );
  }

  const permission = await Notification.requestPermission();
  if (permission !== 'granted') {
    throw new Error('Notification permission denied: ' + permission);
  }

  // Ensure the SW is registered + active before subscribing.
  await registerCourierServiceWorker();
  const sw = await navigator.serviceWorker.ready;

  const subscription = await sw.pushManager.subscribe({
    userVisibleOnly: true,
    applicationServerKey: urlBase64ToUint8Array(VAPID_APP_PUBLIC_KEY),
  });

  // TODO(server): implement POST /api/courier/push/subscribe on the backend.
  // Scaffolded call only — no server wired in this wave.
  await fetch('/api/courier/push/subscribe', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(subscription),
  });

  return subscription;
}

// Convert a VAPID public key (URL-safe base64) to a Uint8Array for
// pushManager.subscribe. Standard Web Push boilerplate.
function urlBase64ToUint8Array(base64String) {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding)
    .replace(/-/g, '+')
    .replace(/_/g, '/');
  const raw = atob(base64);
  const out = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
  return out;
}

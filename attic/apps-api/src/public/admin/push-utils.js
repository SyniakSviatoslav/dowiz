// Shared push subscription utilities for DeliveryOS admin pages
// VAPID_PUBLIC_KEY is injected via env or meta tag

async function urlBase64ToUint8Array(base64String) {
  const padding = '='.repeat((4 - base64String.length % 4) % 4);
  const base64 = (base64String + padding).replace(/\-/g, '+').replace(/_/g, '/');
  const rawData = atob(base64);
  const outputArray = new Uint8Array(rawData.length);
  for (let i = 0; i < rawData.length; ++i) {
    outputArray[i] = rawData.charCodeAt(i);
  }
  return outputArray;
}

async function getVapidPublicKey() {
  // Try meta tag first, fall back to global
  const meta = document.querySelector('meta[name="vapid-public-key"]');
  if (meta) return meta.getAttribute('content');
  if (window.VAPID_PUBLIC_KEY) return window.VAPID_PUBLIC_KEY;
  // Fallback: fetch from server
  try {
    const res = await fetch('/api/push/vapid-public-key');
    const data = await res.json();
    return data.publicKey;
  } catch {
    return null;
  }
}

async function registerPushSubscription(token, locationId) {
  if (!('serviceWorker' in navigator) || !('PushManager' in window)) {
    console.log('[Push] Not supported');
    return null;
  }

  try {
    const registration = await navigator.serviceWorker.register('/sw.js');
    await navigator.serviceWorker.ready;

    const vapidKey = await getVapidPublicKey();
    if (!vapidKey) {
      console.error('[Push] VAPID key not available');
      return null;
    }

    const subscription = await registration.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: await urlBase64ToUint8Array(vapidKey),
    });

    // Send to server
    const res = await fetch(`/api/owner/locations/${locationId}/push/subscribe`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
      body: JSON.stringify({ subscription }),
    });

    if (!res.ok) throw new Error('Failed to register push subscription');
    console.log('[Push] Subscribed successfully');
    return subscription;
  } catch (err) {
    console.error('[Push] Subscription failed:', err);
    return null;
  }
}

async function unregisterPushSubscription(token, locationId) {
  try {
    const res = await fetch(`/api/owner/locations/${locationId}/push/unsubscribe`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${token}`,
      },
    });
    if (!res.ok) throw new Error('Failed to unregister');
    console.log('[Push] Unsubscribed');
    return true;
  } catch (err) {
    console.error('[Push] Unsubscribe failed:', err);
    return false;
  }
}

async function getPushState(token, locationId) {
  try {
    const res = await fetch(`/api/owner/locations/${locationId}/push/state`, {
      headers: { 'Authorization': `Bearer ${token}` },
    });
    return await res.json();
  } catch {
    return { subscribed: false };
  }
}

import { apiClient } from '../../../lib/index.js';

export async function requestPushPermission(_slug: string) {
  if (typeof window === 'undefined' || !('Notification' in window) || !('serviceWorker' in navigator) || !('PushManager' in window)) return;
  if (Notification.permission === 'granted') return;
  if (Notification.permission === 'denied') return;
  const result = await Notification.requestPermission();
  if (result !== 'granted') return;
  try {
    const reg = await navigator.serviceWorker.register('/sw.js');
    const publicKeyRes: any = await apiClient('/push/vapid-public-key');
    const publicKey: string | undefined = publicKeyRes?.publicKey;
    if (!publicKey) return;
    const sub = await reg.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: urlBase64ToUint8Array(publicKey) as any,
    });
    const p256dhKey = sub.getKey('p256dh');
    const authKey = sub.getKey('auth');
    if (!p256dhKey || !authKey) return;
    await apiClient('/customer/push/subscribe', {
      method: 'POST',
      body: {
        endpoint: sub.endpoint,
        keys: { p256dh: btoa(String.fromCharCode(...new Uint8Array(p256dhKey))), auth: btoa(String.fromCharCode(...new Uint8Array(authKey))) },
        opted_in: true,
      },
    });
  } catch (err) {
    console.debug('[CheckoutPage] push subscription failed:', err);
  }
}

function urlBase64ToUint8Array(base64String: string): Uint8Array {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
  const rawData = atob(base64);
  return Uint8Array.from(rawData.split('').map((c) => c.charCodeAt(0)));
}

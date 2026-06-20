import React, { useState, useEffect, useRef } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Button, MapWithPin, useI18n, StickyActionBar, PriceDisplay, useCurrency, OTPModal } from '@deliveryos/ui';
import { CURRENCIES } from '@deliveryos/shared-types';
import type { LngLatLike } from '@deliveryos/ui';
import { PHONE_E164_REGEX } from '@deliveryos/shared-types';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

// Albania has no other realistic country here, so accept how people actually type
// their number — local "069...", "0 69 ...", "00355...", bare "69..." — and coerce
// to the E.164 (+355...) the backend requires, instead of silently rejecting it.
function normalizeAlbanianPhone(raw: string): string {
  const compact = (raw || '').replace(/[\s()\-.]/g, '');
  if (!compact) return raw;
  if (compact.startsWith('+')) return compact;
  let digits = compact.replace(/\D/g, '');
  if (digits.startsWith('00')) return '+' + digits.slice(2);
  if (digits.startsWith('355')) return '+' + digits;
  if (digits.startsWith('0')) digits = digits.slice(1); // drop the national trunk 0
  return digits ? '+355' + digits : raw;
}

const OrderCreateResponse = z.object({
  id: z.string(),
  authToken: z.string().optional(),
}).passthrough();

// The /orders endpoint returns 200 with this body (NO order created) when the
// location requires phone verification and it hasn't been satisfied yet.
const PreflightResponse = z.object({
  outcome: z.enum(['clean', 'soft_confirm', 'hard_block']),
  requiresOtp: z.boolean().optional(),
  reasons: z.array(z.object({ code: z.string() }).passthrough()).optional(),
}).passthrough();

const OtpSendResponse = z.object({ otp_token: z.string() }).passthrough();
const OtpVerifyResponse = z.object({ verified_token: z.string() }).passthrough();
import { useSharedCart } from '../../lib/CartProvider.js';

const isDevMode = () => typeof window !== 'undefined' && sessionStorage.getItem('dos_dev') === '1';

async function requestPushPermission(_slug: string) {
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

type DeliveryType = 'delivery' | 'pickup' | 'scheduled';

interface MacroData { label: string; grams: number; color: string; kcalPer: number }
function NutritionRing({ kcal, protein, fat, carbs }: { kcal: number; protein: number; fat: number; carbs: number }) {
  const CX = 52, R = 38, SW = 10;
  const macros: MacroData[] = [
    { label: 'Protein', grams: protein, color: '#3b82f6', kcalPer: 4 },
    { label: 'Carbs',   grams: carbs,   color: '#22c55e', kcalPer: 4 },
    { label: 'Fat',     grams: fat,     color: '#f59e0b', kcalPer: 9 },
  ];
  const totalEnergy = macros.reduce((s, m) => s + m.grams * m.kcalPer, 0);
  let cumulative = 0;
  const arcs = macros
    .filter(m => m.grams > 0)
    .map(m => {
      const fraction = (m.grams * m.kcalPer) / totalEnergy;
      const startOffset = cumulative;
      cumulative += fraction;
      return { ...m, fraction, startOffset };
    });
  const hasMacros = arcs.length > 0;
  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-40px' }}
      transition={{ duration: 0.35, ease: [0.16, 1, 0.3, 1] }}
      className="rounded-[12px] p-4 border shadow-sm"
      style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}
    >
      <p className="text-[11px] font-semibold uppercase tracking-wide mb-3" style={{ color: 'var(--brand-text-muted)' }}>
        <i className="ti ti-flame mr-1" />Order Nutrition
      </p>
      <div className="flex items-center gap-5">
        <div className="relative shrink-0" style={{ width: CX * 2, height: CX * 2 }}>
          <svg width={CX * 2} height={CX * 2} viewBox={`0 0 ${CX * 2} ${CX * 2}`} aria-hidden="true" style={{ transform: 'rotate(-90deg)' }}>
            <circle cx={CX} cy={CX} r={R} fill="none" stroke="var(--brand-border)" strokeWidth={SW} />
            {hasMacros ? arcs.map((arc, i) => (
              <motion.circle
                key={arc.label}
                cx={CX} cy={CX} r={R}
                fill="none"
                stroke={arc.color}
                strokeWidth={SW}
                strokeLinecap="butt"
                initial={{ pathLength: 0, pathOffset: arc.startOffset }}
                animate={{ pathLength: arc.fraction, pathOffset: arc.startOffset }}
                transition={{ duration: 0.9, delay: 0.15 + i * 0.18, ease: [0.16, 1, 0.3, 1] }}
              />
            )) : (
              <motion.circle
                cx={CX} cy={CX} r={R}
                fill="none"
                stroke="var(--brand-primary)"
                strokeWidth={SW}
                strokeLinecap="round"
                initial={{ pathLength: 0, pathOffset: 0 }}
                animate={{ pathLength: 0.78, pathOffset: 0 }}
                transition={{ duration: 1.1, ease: [0.16, 1, 0.3, 1] }}
              />
            )}
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
            <motion.span
              className="text-[18px] font-bold leading-none"
              style={{ color: 'var(--brand-text)' }}
              initial={{ opacity: 0, scale: 0.7 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ delay: 0.55, type: 'spring', stiffness: 220, damping: 18 }}
            >
              {Math.round(kcal)}
            </motion.span>
            <span className="text-[9px] uppercase tracking-widest mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>kcal</span>
          </div>
        </div>
        <div className="flex-1 space-y-2.5">
          {hasMacros ? macros.filter(m => m.grams > 0).map((m, i) => (
            <motion.div
              key={m.label}
              className="flex items-center gap-2"
              initial={{ opacity: 0, x: -8 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: 0.35 + i * 0.1 }}
            >
              <div className="w-2.5 h-2.5 rounded-full shrink-0" style={{ background: m.color }} />
              <span className="text-[12px] flex-1" style={{ color: 'var(--brand-text-muted)' }}>{m.label}</span>
              <span className="text-[13px] font-bold tabular-nums" style={{ color: 'var(--brand-text)' }}>{Math.round(m.grams)}g</span>
            </motion.div>
          )) : (
            <motion.p className="text-[12px]" style={{ color: 'var(--brand-text-muted)' }} initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.5 }}>
              ~{Math.round(kcal)} kcal total
            </motion.p>
          )}
        </div>
      </div>
    </motion.div>
  );
}

export function CheckoutPage() {
  const { slug } = useParams<{ slug: string }>();
  const navigate = useNavigate();
  const { items, clearCart } = useSharedCart();
  const { t } = useI18n();
  const { currency: activeCurrency } = useCurrency();
  const currencySymbol = CURRENCIES[activeCurrency]?.symbol ?? activeCurrency;

  const [deliveryType, setDeliveryType] = useState<DeliveryType>('delivery');
  const [address, setAddress] = useState('');
  const [phone, setPhone] = useState('');
  const [customerName, setCustomerName] = useState('');
  // UX-2 optional messenger contact (Telegram/WhatsApp/Viber) so the courier can text.
  const [messengerKind, setMessengerKind] = useState('');
  const [messengerHandle, setMessengerHandle] = useState('');
  // UX-3 optional entry-anchor photo (uploaded to R2 before the order exists).
  const [entryPhotoKey, setEntryPhotoKey] = useState('');
  const [entryPhotoPreview, setEntryPhotoPreview] = useState('');
  const [photoUploading, setPhotoUploading] = useState(false);
  const entryFileRef = useRef<HTMLInputElement>(null);
  const uploadEntryPhoto = async (file: File) => {
    if (!file) return;
    setPhotoUploading(true);
    try {
      const form = new FormData();
      form.append('file', file);
      const res = await apiClient<any>('/public/entry-photo', { method: 'POST', body: form, timeout: 60000 });
      setEntryPhotoKey(res.key);
      setEntryPhotoPreview(res.url || '');
    } catch { /* optional — leave unset on failure */ } finally { setPhotoUploading(false); }
  };
  const [pinLocation, setPinLocation] = useState<LngLatLike | null>(null);
  const [locationId, setLocationId] = useState<string | null>(null);
  const [locationCenter, setLocationCenter] = useState<LngLatLike>([19.456, 41.324]); // Durrës default
  const [notes, setNotes] = useState('');
  const [cashAmount, setCashAmount] = useState<number>(0);
  const [tipAmount, setTipAmount] = useState<number>(0); // UX-4 optional courier tip
  const [orderError, setOrderError] = useState('');
  // #4 — restaurant phone for the failure fallback, cached on mount so the "call the
  // restaurant" CTA never depends on a network fetch made under the same load that
  // caused the failure. Null = no CTA (fail-soft to the generic toast).
  const [fallbackPhone, setFallbackPhone] = useState<string | null>(null);
  const [showPhoneFallback, setShowPhoneFallback] = useState(false);
  const [instructionOption, setInstructionOption] = useState<string>('');
  const [instructionCustom, setInstructionCustom] = useState<string>('');
  const [entrance, setEntrance] = useState('');
  const [apartment, setApartment] = useState('');
  const [phoneError, setPhoneError] = useState('');
  const [entranceError, setEntranceError] = useState('');
  const [apartmentError, setApartmentError] = useState('');
  const [placing, setPlacing] = useState(false);
  const [showConfirmation, setShowConfirmation] = useState(false);
  const [cityFact, setCityFact] = useState<string | null>(null);
  const [currencyCode, setCurrencyCode] = useState<string>('ALL');
  // Phone-verification (OTP) state — only engaged when the backend signals
  // `requiresOtp` on a soft_confirm. Otherwise checkout behaves exactly as before.
  const [otpOpen, setOtpOpen] = useState(false);
  const [otpToken, setOtpToken] = useState<string | null>(null);

    useEffect(() => {
    if (!slug) return;
    fetch(`/public/locations/${slug}/info`).then(r => r.json())
      .then((info: any) => {
        setLocationId(info.id);
        if (info.currency_code) setCurrencyCode(info.currency_code);
        if (info.lng && info.lat) setLocationCenter([info.lng, info.lat]);
        if (info.address) {
          const parts = info.address.split(',');
          const city = parts.length > 1 ? parts[parts.length - 1].trim() : parts[0].trim();
          if (city) {
            fetch(`https://en.wikipedia.org/api/rest_v1/page/summary/${encodeURIComponent(city)}`)
              .then(r => r.json())
              .then((wiki: any) => {
                if (wiki.extract) setCityFact(wiki.extract.split('.')[0] + '.');
              })
              .catch(() => {/* silently ignore */});
          }
        }
      })
      .catch((err) => {
        console.debug('[CheckoutPage] failed to load location info:', err);
      });
    // Cache the restaurant phone NOW (on mount) for the order-failure fallback, so
    // the CTA is available even when the order POST fails under DB/load pressure.
    fetch(`/api/public/locations/${slug}/fallback-config`).then(r => r.json())
      .then((cfg: any) => {
        if (cfg && cfg.showPhoneOnError !== false && cfg.phone) setFallbackPhone(cfg.phone);
      })
      .catch(() => {/* fail-soft: no CTA, generic toast only */});
    try {
      const saved = localStorage.getItem(`dos_last_delivery_${slug}`);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.lat && parsed.lng) {
          setPinLocation([parsed.lng, parsed.lat]);
        }
        if (parsed.address) {
          setAddress(parsed.address);
        }
        if (parsed.entrance) {
          setEntrance(parsed.entrance);
        }
        if (parsed.apartment) {
          setApartment(parsed.apartment);
        }
      }
    } catch (err) { console.debug('[CheckoutPage] corrupt localStorage:', err); }
    try {
      const draft = localStorage.getItem(`dos_checkout_draft_${slug}`);
      if (draft) {
        const d = JSON.parse(draft);
        if (d.phone) setPhone(d.phone);
        if (d.customerName) setCustomerName(d.customerName);
        if (d.deliveryType) setDeliveryType(d.deliveryType);
        if (d.instructionOption) setInstructionOption(d.instructionOption);
        if (d.instructionCustom) setInstructionCustom(d.instructionCustom);
        if (d.cashAmount) setCashAmount(d.cashAmount);
        // UX-3/UX-2 follow-up: remember the entrance photo + messenger per device.
        if (d.entryPhotoKey) { setEntryPhotoKey(d.entryPhotoKey); if (d.entryPhotoPreview) setEntryPhotoPreview(d.entryPhotoPreview); }
        if (d.messengerKind) setMessengerKind(d.messengerKind);
        if (d.messengerHandle) setMessengerHandle(d.messengerHandle);
      }
    } catch {}
  }, [slug]);

  useEffect(() => {
    if (!slug) return;
    try {
      localStorage.setItem(`dos_checkout_draft_${slug}`, JSON.stringify({
        phone, customerName, deliveryType, instructionOption, instructionCustom, cashAmount,
        entryPhotoKey, entryPhotoPreview, messengerKind, messengerHandle,
      }));
    } catch {}
  }, [slug, phone, customerName, deliveryType, instructionOption, instructionCustom, cashAmount, entryPhotoKey, entryPhotoPreview, messengerKind, messengerHandle]);

  const subtotal = items.reduce((sum, item) => sum + item.price * item.quantity, 0);
  const deliveryFee = deliveryType === 'delivery' ? 200 : 0;
  const total = subtotal + deliveryFee;

  const hasNutrition = items.some((item: any) => (item as any).kcal != null);
  const nutritionTotal = items.reduce((acc, item: any) => ({
    kcal: acc.kcal + ((item as any).kcal ?? 0) * item.quantity,
    protein: acc.protein + ((item as any).protein ?? 0) * item.quantity,
    fat: acc.fat + ((item as any).fat ?? 0) * item.quantity,
    carbs: acc.carbs + ((item as any).carbs ?? 0) * item.quantity,
  }), { kcal: 0, protein: 0, fat: 0, carbs: 0 });

  const orderItems = items.map(i => ({
    product_id: i.productId,
    quantity: i.quantity,
    modifier_ids: Object.values(i.options || {}).flat() as string[],
  }));

    const handlePlaceOrder = async (e: React.FormEvent) => {
    e.preventDefault();
    setOrderError('');
    if (items.length === 0 || !slug || !locationId) return;
    const e164 = normalizeAlbanianPhone(phone);
    if (!e164 || !PHONE_E164_REGEX.test(e164)) {
      setPhoneError(t('checkout.phone_invalid', 'Enter a valid phone number (+355...)'));
      return;
    }
    if (e164 !== phone) setPhone(e164); // reflect the normalized value back in the field
    setPhoneError('');
    
    // Validate entrance and apartment for delivery orders
    if (deliveryType === 'delivery') {
      if (!entrance.trim()) {
        setEntranceError(t('checkout.entrance_required', 'Entrance is required'));
        return;
      }
      if (!apartment.trim()) {
        setApartmentError(t('checkout.apartment_required', 'Apartment is required'));
        return;
      }
    }
    setEntranceError('');
    setApartmentError('');
    
    if (deliveryType === 'delivery' && !notes.trim()) {
      setOrderError(t('checkout.notes_required', 'Please describe how to find your location'));
      return;
    }
    setPlacing(true);
    await submitOrder();
  };

  // Submits the order. When `verifiedToken` is provided it is passed back to the
  // backend (header + acknowledged code) so a require-OTP location lets it through.
  // Returns true if the order was created (or OTP flow was started), false on hard error.
  const submitOrder = async (verifiedToken?: string) => {
    if (!slug || !locationId) { setPlacing(false); return false; }
    setOrderError('');
    setShowPhoneFallback(false);
    try {
      const idempotencyKey = crypto.randomUUID();
      const pinLat = (pinLocation as [number, number])?.[1] || (locationCenter as [number, number])?.[1] || 41.324;
      const pinLng = (pinLocation as [number, number])?.[0] || (locationCenter as [number, number])?.[0] || 19.456;
      const raw = await apiClient<typeof PreflightResponse>('/orders', {
        method: 'POST',
        headers: verifiedToken ? { 'x-otp-verified': verifiedToken } : undefined,
        body: {
          locationId: locationId,
          type: deliveryType === 'pickup' ? 'pickup' : 'delivery',
          items: orderItems,
          customer: {
            phone: normalizeAlbanianPhone(phone),
            name: customerName || undefined,
            ...(messengerKind && messengerHandle.trim()
              ? { messenger_kind: messengerKind, messenger_handle: messengerHandle.trim() }
              : {}),
          },
          ...(entryPhotoKey ? { delivery_photo_key: entryPhotoKey } : {}),
          ...(tipAmount > 0 ? { tip_amount: tipAmount } : {}),
          // Pickup orders carry no delivery pin/address (no delivery fee).
          ...(deliveryType === 'pickup' ? {} : {
            delivery: {
              pin: { lat: pinLat, lng: pinLng },
              address_text: address || undefined,
              notes: notes.trim() || undefined,
            },
          }),
          payment: { method: 'cash' },
          cash_pay_with: cashAmount > 0 ? cashAmount : undefined,
          idempotency_key: idempotencyKey,
          // Acknowledge the OTP soft-reason once we've verified the phone.
          acknowledged_codes: verifiedToken ? ['otp_required'] : [],
          prefs: {
            dropoff: {
              entrance: entrance.trim(),
              apartment: apartment.trim(),
            }
          },
          delivery_instructions: instructionOption
            ? instructionCustom
              ? `${instructionOption}: ${instructionCustom}`
              : instructionOption
            : undefined,
        },
      });

      // Phone verification required but not yet satisfied → start the OTP flow.
      const pre = PreflightResponse.safeParse(raw);
      if (pre.success && pre.data.outcome === 'soft_confirm' && pre.data.requiresOtp) {
        await beginOtpFlow();
        return true;
      }

      const orderRes = OrderCreateResponse.parse(raw);
      try {
        localStorage.setItem(`dos_last_delivery_${slug}`, JSON.stringify({
          lat: pinLat,
          lng: pinLng,
          address,
          entrance,
          apartment,
        }));
      } catch (err) { console.debug('[CheckoutPage] localStorage write failed:', err); }
      try { localStorage.removeItem(`dos_checkout_draft_${slug}`); } catch {}
      if (orderRes.authToken) {
        localStorage.setItem('dos_access_token', orderRes.authToken);
      }
      requestPushPermission(slug!);
      clearCart();
      setOtpOpen(false);
      setPlacing(false);
      setShowConfirmation(true);
      setTimeout(() => navigate(`/s/${slug}/order/${orderRes.id}`), 1500);
      return true;
    } catch (err: any) {
      setPlacing(false);
      // Dev convenience ONLY — compile-time gated so Vite dead-strips this entire
      // branch from any production build (import.meta.env.DEV is statically false),
      // making the o_mock_123 fake-success physically absent from the prod bundle.
      // A real session's dos_dev flag can no longer spoof success on a real failure.
      if (import.meta.env.DEV && isDevMode()) { clearCart(); navigate(`/s/${slug}/order/o_mock_123`); return false; }
      if (err?.status === 422 && err?.data?.code === 'MIN_ORDER_NOT_MET') {
        setOrderError(t('checkout.min_order_error', 'Minimum order is {{min}} {{currency}}. Your total is {{subtotal}}.', {
          min: err.data.details?.min_order_value,
          currency: currencyCode,
          subtotal: err.data.details?.subtotal,
        }));
        return false;
      }
      // Non-422 (5xx / network / timeout) is not customer-fixable → offer the
      // out-of-band path ("call the restaurant"). 422 business errors carry an
      // actionable message and the customer can fix the cart, so no phone CTA.
      // Cart is preserved either way (clearCart only runs on success).
      if (err?.status !== 422) setShowPhoneFallback(true);
      setOrderError(t('checkout.order_failed', 'Failed to place order. Please try again.'));
      return false;
    }
  };

  // hex(JSON.stringify(items)) — the backend re-hashes this to bind the OTP to the cart.
  const orderIntentHashHex = () => {
    const intentItems = orderItems.map(i => ({ product_id: i.product_id, quantity: i.quantity }));
    const json = JSON.stringify(intentItems);
    let hex = '';
    for (let i = 0; i < json.length; i++) hex += json.charCodeAt(i).toString(16).padStart(2, '0');
    return hex;
  };

  // Send the first code and reveal the verification modal.
  const beginOtpFlow = async () => {
    try {
      await sendOtp();
      setOtpOpen(true);
    } catch (err: any) {
      setOrderError(err?.message || t('otp.send_failed', 'Couldn’t send the verification code. Try again.'));
    } finally {
      setPlacing(false);
    }
  };

  const sendOtp = async () => {
    const res = await apiClient<typeof OtpSendResponse>(`/customer/locations/${slug}/otp/send`, {
      method: 'POST',
      schema: OtpSendResponse,
      body: {
        phone: normalizeAlbanianPhone(phone),
        order_intent: {
          items: orderItems.map(i => ({ product_id: i.product_id, quantity: i.quantity })),
          total,
          currency: currencyCode,
        },
      },
    });
    setOtpToken(res.otp_token);
  };

  const verifyOtp = async (code: string) => {
    if (!otpToken) throw new Error(t('otp.send_failed', 'Couldn’t send the verification code. Try again.'));
    const res = await apiClient<typeof OtpVerifyResponse>(`/customer/locations/${slug}/otp/verify`, {
      method: 'POST',
      schema: OtpVerifyResponse,
      body: {
        phone: normalizeAlbanianPhone(phone),
        code,
        otp_token: otpToken,
        order_intent_hash: orderIntentHashHex(),
      },
    });
    // Re-submit the order with the verified token; OTPModal closes on success.
    setPlacing(true);
    const ok = await submitOrder(res.verified_token);
    if (!ok) throw new Error(t('otp.verify_then_failed', 'Verified, but the order could not be placed. Try again.'));
  };

  if (items.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[60vh] text-center p-6 gap-3">
        <i className="ti ti-shopping-cart text-4xl" style={{ color: 'var(--brand-text-muted)', opacity: 0.4 }} />
        <h2 className="text-xl font-bold">{t('cart.empty')}</h2>
        <Button onClick={() => navigate(`/s/${slug}`)}>{t('common.back')}</Button>
      </div>
    );
  }

  const btnStyle = (type: DeliveryType) => deliveryType === type
    ? { background: 'var(--brand-surface-raised)', color: 'var(--brand-text)', fontWeight: 600 }
    : { background: 'transparent', color: 'var(--brand-text-muted)' } as const;

  return (
    <div className="max-w-xl mx-auto p-4 md:py-8 space-y-6 pb-32">
      <div className="flex items-center gap-3 mb-6">
        <motion.button onClick={() => navigate(-1)} whileTap={{ scale: 0.95 }} aria-label={t('common.back', 'Go back')} className="w-10 h-10 rounded-full flex items-center justify-center border transition-colors active:scale-95" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
          <i className="ti ti-arrow-left" aria-hidden="true" />
        </motion.button>
        <h1 className="text-[24px] font-bold" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.title')}</h1>
      </div>

      <form id="checkout-form" onSubmit={handlePlaceOrder} className="space-y-6">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-40px' }}
          transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1] }}
          className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <h2 className="text-[20px] font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.contact_info', 'Contact Info')}</h2>
          <div className="space-y-3">
            <div>
              <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.name', 'Name')}</label>
              <div className="relative">
                <i className="ti ti-user absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                <input required value={customerName} onChange={e => setCustomerName(e.target.value)} placeholder={t('checkout.name_placeholder', 'Your name')} autoComplete="name" className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
              </div>
            </div>
            <div>
              <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.phone', 'Phone')}</label>
              <div className="relative">
                <i className="ti ti-phone absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                <input required value={phone} onChange={e => { setPhone(e.target.value); setPhoneError(''); }} onBlur={() => setPhone(p => normalizeAlbanianPhone(p))} placeholder="+355 6X XXX XXXX" title="+355 followed by 7-14 digits" type="tel" inputMode="tel" autoComplete="tel" data-testid="checkout-phone" className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: phoneError ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
                {phoneError && <p role="alert" className="text-[12px] mt-1" style={{ color: 'var(--color-danger)' }}>{phoneError}</p>}
              </div>
            </div>
            {/* UX-2: optional messenger so the courier can text instead of call */}
            <div>
              <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.messenger', 'Messenger (optional)')}</label>
              <div className="flex gap-2">
                <select value={messengerKind} onChange={e => setMessengerKind(e.target.value)} data-testid="checkout-messenger-kind"
                  className="h-[48px] px-2 outline-none text-[14px] border rounded-[8px]" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
                  <option value="">{t('checkout.messenger_none', '—')}</option>
                  <option value="telegram">Telegram</option>
                  <option value="whatsapp">WhatsApp</option>
                  <option value="viber">Viber</option>
                </select>
                <input value={messengerHandle} onChange={e => setMessengerHandle(e.target.value)} disabled={!messengerKind}
                  required={!!messengerKind}
                  placeholder={messengerKind === 'telegram' ? '@username' : '+355 6X XXX XXXX'} data-testid="checkout-messenger-handle"
                  className="flex-1 h-[48px] px-3 outline-none text-[14px] border rounded-[8px] disabled:opacity-50" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
              </div>
            </div>
            {/* UX-3: optional entrance photo (delivery only) — camera or gallery */}
            {deliveryType !== 'pickup' && (
              <div>
                <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.entry_photo', 'Entrance photo (optional)')}</label>
                <div className="flex items-center gap-3">
                  <button type="button" onClick={() => entryFileRef.current?.click()} disabled={photoUploading}
                    className="inline-flex items-center gap-2 px-4 py-2 border rounded-[8px] cursor-pointer text-sm disabled:opacity-60"
                    style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
                    <i className="ti ti-camera" aria-hidden="true" />
                    {photoUploading ? t('checkout.uploading', 'Uploading…') : (entryPhotoKey ? t('checkout.change_photo', 'Change photo') : t('checkout.add_photo', 'Add photo'))}
                  </button>
                  <input ref={entryFileRef} type="file" accept="image/*" className="hidden" data-testid="entry-photo-input" disabled={photoUploading}
                    onChange={e => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) void uploadEntryPhoto(f); }} />
                  {entryPhotoPreview && (
                    <img src={entryPhotoPreview} alt={t('checkout.entry_photo', 'Entrance photo')} data-testid="entry-photo-preview" className="h-12 w-12 object-cover rounded-[8px] border" style={{ borderColor: 'var(--brand-border)' }} />
                  )}
                </div>
                <p className="text-[12px] mt-1" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.entry_photo_hint', 'Helps the courier find your entrance.')}</p>
              </div>
            )}
          </div>
        </motion.div>
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-40px' }}
          transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1], delay: 0.05 }}
          className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <h2 className="text-[20px] font-semibold mb-6" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.delivery_address')}</h2>
          <div className="flex p-1 rounded-[10px] mb-6 gap-0.5" role="tablist" aria-label={t('checkout.delivery_type', 'Delivery type')} style={{ background: 'var(--brand-surface)' }}>
            <motion.button type="button" role="tab" whileTap={{ scale: 0.97 }} aria-selected={deliveryType === 'delivery'} onClick={() => setDeliveryType('delivery')} className="flex-1 py-2 text-[13px] rounded-[8px] transition-all" style={btnStyle('delivery')}>{t('courier.deliver')}</motion.button>
            <motion.button type="button" role="tab" whileTap={{ scale: 0.97 }} aria-selected={deliveryType === 'pickup'} onClick={() => setDeliveryType('pickup')} className="flex-1 py-2 text-[13px] rounded-[8px] transition-all" style={btnStyle('pickup')}>{t('courier.pickup')}</motion.button>
            {/* Scheduled is scaffold (not yet implemented end-to-end) — hidden until supported. */}
          </div>

          {deliveryType === 'delivery' && (
            <div className="space-y-4">
              <div>
                <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.delivery_address')}</label>
                <MapWithPin className="h-48 w-full rounded-lg" initialCenter={locationCenter} onPinChange={setPinLocation} confirmLabel={t('common.confirm')} placeholder={t('checkout.delivery_address')} />
              </div>
              <div>
                <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.delivery_address')}</label>
                <div className="relative">
                  <i className="ti ti-map-pin absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                  <input required value={address} onChange={e => setAddress(e.target.value)} placeholder={t('checkout.delivery_address')} className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                </div>
              </div>
              <div className="space-y-4">
                <div>
                  <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.entrance')}</label>
                  <div className="relative">
                    <i className="ti ti-door-open absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                    <input required value={entrance} onChange={e => setEntrance(e.target.value)} data-testid="checkout-entrance" placeholder={t('checkout.entrance_placeholder', 'Entrance number or name')} className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  </div>
                  {entranceError && <p role="alert" className="text-[12px] mt-1" style={{ color: 'var(--color-danger)' }}>{entranceError}</p>}
                </div>
                <div>
                  <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.apartment')}</label>
                  <div className="relative">
                    <i className="ti ti-apartment absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                    <input required value={apartment} onChange={e => setApartment(e.target.value)} data-testid="checkout-apartment" placeholder={t('checkout.apartment_placeholder', 'Apartment or unit number')} className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  </div>
                  {apartmentError && <p role="alert" className="text-[12px] mt-1" style={{ color: 'var(--color-danger)' }}>{apartmentError}</p>}
                </div>
              </div>
              <div>
                <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>
                  {t('checkout.notes', 'How to find you')} <span style={{ color: 'var(--color-danger)' }}>*</span>
                </label>
                <div className="relative">
                  <i className="ti ti-map-2 absolute left-3 top-3 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                  <textarea
                    required
                    value={notes}
                    onChange={e => setNotes(e.target.value)}
                    rows={3}
                    placeholder={t('checkout.notes_placeholder', 'Describe how to find the exact place: floor, building color, nearby landmark, gate code...')}
                    className="w-full pl-10 pr-3 pt-2.5 pb-2 outline-none text-[14px] border rounded-[8px] transition-colors resize-none"
                    style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
                  />
                </div>
              </div>
              <div>
                <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.dropoff_instructions', 'Dropoff instructions')}</label>
                <div className="flex flex-wrap gap-2 mb-2" role="group" aria-label={t('checkout.dropoff_instructions', 'Dropoff instructions')}>
                  {[
                    { key: 'checkout.dropoff_door', val: 'Leave at door' },
                    { key: 'checkout.dropoff_call', val: 'Call on arrival' },
                    { key: 'checkout.dropoff_ring', val: 'Ring bell' },
                    { key: 'checkout.dropoff_hand', val: 'Hand to me' },
                    { key: 'checkout.dropoff_text', val: 'Text on arrival' },
                  ].map((opt) => (
                    <motion.button
                      key={opt.key}
                      type="button"
                      whileTap={{ scale: 0.95 }}
                      aria-pressed={instructionOption === opt.val}
                      onClick={() => setInstructionOption(instructionOption === opt.val ? '' : opt.val)}
                      className="px-3 py-1.5 text-[12px] rounded-[20px] border transition-all active:scale-95"
                      style={{
                        background: instructionOption === opt.val ? 'var(--brand-primary-light)' : 'var(--brand-surface-raised)',
                        borderColor: instructionOption === opt.val ? 'var(--brand-primary)' : 'var(--brand-border)',
                        color: instructionOption === opt.val ? 'var(--brand-text)' : 'var(--brand-text)',
                      }}
                    >{t(opt.key, opt.val)}</motion.button>
                  ))}
                </div>
                {instructionOption && (
                  <div className="relative">
                    <i className="ti ti-edit absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                    <input value={instructionCustom} onChange={e => setInstructionCustom(e.target.value)} placeholder={t('checkout.extra_notes', 'Extra notes...')} className="w-full h-[44px] pl-10 pr-3 outline-none text-[13px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  </div>
                )}
              </div>
            </div>
          )}

          {deliveryType === 'pickup' && (
            <div className="space-y-4">
              <div className="border rounded-[12px] p-4" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
                <h3 className="text-[14px] font-bold mb-1" style={{ color: 'var(--brand-text)' }}>{t('courier.pickup')}</h3>
                <p className="text-[14px] mb-4" style={{ color: 'var(--brand-text-muted)' }}>Dubin & Sushi, Rruga Sami Frasheri 12, Tirana</p>
                <div className="w-full h-[120px] rounded-[8px] relative overflow-hidden border flex items-center justify-center" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)' }}>
                  <div className="absolute inset-0 opacity-10" style={{ backgroundImage: 'linear-gradient(var(--brand-border) 1px, transparent 1px), linear-gradient(90deg, var(--brand-border) 1px, transparent 1px)', backgroundSize: '20px 20px' }} />
                  <i className="ti ti-building-store text-3xl relative z-10" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                </div>
              </div>
              <div className="flex items-center gap-3 p-3 rounded-[8px] border" style={{ background: 'var(--color-info-light)', borderColor: 'var(--color-info)', color: 'var(--color-info)' }}>
                <i className="ti ti-info-circle" aria-hidden="true" />
                <p className="text-[13px] font-medium">{t('checkout.phone_hint')}</p>
              </div>
            </div>
          )}

          {deliveryType === 'scheduled' && (
            <div className="flex items-center gap-3 p-4 rounded-[12px] border" style={{ background: 'var(--color-warning-light, rgba(217,119,6,0.1))', borderColor: 'var(--color-warning, #D97706)' }}>
              <i className="ti ti-clock text-lg shrink-0" style={{ color: 'var(--color-warning, #D97706)' }} />
              <p className="text-[13px] font-medium" style={{ color: 'var(--brand-text)' }}>
                {t('checkout.scheduled_coming_soon', 'Scheduled delivery coming soon. Please select Delivery or Pickup.')}
              </p>
            </div>
          )}
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-40px' }}
          transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1], delay: 0.1 }}
          className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <h2 className="text-[20px] font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.payment_method')}</h2>
          <div className="border rounded-[8px] p-3 mb-3" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-primary)' }}>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <i className="ti ti-cash text-xl" aria-hidden="true" style={{ color: 'var(--brand-primary)' }} />
                <div>
                  <div className="text-[14px] font-bold" style={{ color: 'var(--brand-text)' }}>{t('checkout.cash')}</div>
                  <div className="text-[12px]" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.place_order')}</div>
                </div>
              </div>
              <i className="ti ti-check" aria-hidden="true" style={{ color: 'var(--brand-primary)' }} />
            </div>
            <div className="mt-3 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
              <label htmlFor="cash-amount" className="text-[12px] font-semibold mb-1.5 block" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.cash_amount', 'Cash amount')}</label>
              <div className="flex gap-2">
                <div className="relative flex-1">
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[14px] font-bold" style={{ color: 'var(--brand-text-muted)' }}>{currencySymbol}</span>
                  <input
                    id="cash-amount"
                    type="number"
                    inputMode="decimal"
                    min={total}
                    value={cashAmount || ''}
                    onChange={e => setCashAmount(parseInt(e.target.value) || 0)}
                    className="w-full h-[44px] pl-11 pr-3 outline-none text-[14px] font-bold border rounded-[8px] transition-colors"
                    style={{ background: 'var(--brand-surface)', borderColor: cashAmount > 0 && cashAmount < total ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }}
                    placeholder={String(total)}
                  />
                </div>
              </div>
              {/* UX-4: optional courier tip (single amount, replaces %-badges) */}
              <div className="mt-3 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
                <label htmlFor="tip-amount" className="text-[12px] font-semibold mb-1.5 block" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.tip_amount', 'Tip for courier (optional)')}</label>
                <div className="relative">
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[14px] font-bold" style={{ color: 'var(--brand-text-muted)' }}>{currencySymbol}</span>
                  <input
                    id="tip-amount"
                    type="number"
                    inputMode="decimal"
                    min={0}
                    max={1000000}
                    value={tipAmount || ''}
                    data-testid="checkout-tip"
                    onChange={e => setTipAmount(Math.min(1000000, Math.max(0, parseInt(e.target.value) || 0)))}
                    className="w-full h-[44px] pl-11 pr-3 outline-none text-[14px] font-bold border rounded-[8px]"
                    style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
                    placeholder="0"
                  />
                </div>
                <p className="text-[12px] mt-1" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.tip_hint', 'Goes entirely to your courier, in cash on delivery.')}</p>
              </div>
              {cashAmount > 0 && (
                <div className="flex justify-between text-[13px] mt-2 px-1">
                  {cashAmount >= total ? (
                    <>
                      <span style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.change', 'Change')}</span>
                      <span className="font-bold" style={{ color: 'var(--brand-primary)' }}><PriceDisplay amount={cashAmount - total} /></span>
                    </>
                  ) : (
                    <span style={{ color: 'var(--color-danger)' }}>{t('checkout.cash_amount_too_low', 'Amount must be at least')} <PriceDisplay amount={total} /></span>
                  )}
                </div>
              )}
            </div>
          </div>
        </motion.div>

        {cityFact && (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
            className="rounded-[12px] p-4 border" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)' }}>
            <div className="flex items-start gap-3">
              <span className="text-xl shrink-0 mt-0.5">🌍</span>
              <div>
                <p className="text-[11px] font-semibold uppercase tracking-wide mb-1" style={{ color: 'var(--brand-text-muted)' }}>Did you know?</p>
                <p className="text-[13px] leading-relaxed" style={{ color: 'var(--brand-text)' }}>{cityFact}</p>
              </div>
            </div>
          </motion.div>
        )}

        {hasNutrition && (
          <NutritionRing
            kcal={nutritionTotal.kcal}
            protein={nutritionTotal.protein}
            fat={nutritionTotal.fat}
            carbs={nutritionTotal.carbs}
          />
        )}

        <motion.div
          initial={{ opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-40px' }}
          transition={{ duration: 0.25, ease: [0.16, 1, 0.3, 1], delay: 0.15 }}
          className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <h2 className="text-[20px] font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('order.title')}</h2>
          <div className="space-y-3 mb-4">
            <div className="flex justify-between text-[14px]">
              <span style={{ color: 'var(--brand-text-muted)' }}>{t('cart.subtotal')}</span>
                  <PriceDisplay amount={subtotal} />
            </div>
            {deliveryType === 'delivery' && (
              <div className="flex justify-between text-[14px]">
                <span style={{ color: 'var(--brand-text-muted)' }}>{t('cart.delivery_fee')}</span>
                  <PriceDisplay amount={deliveryFee} />
              </div>
            )}
            {tipAmount > 0 && (
              <div className="flex justify-between text-[14px]" data-testid="checkout-tip-line">
                <span style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.tip_for_courier', 'Tip for courier (cash)')}</span>
                <PriceDisplay amount={tipAmount} />
              </div>
            )}
            {hasNutrition && (
              <div className="flex justify-between text-[12px]">
                <span style={{ color: 'var(--brand-text-muted)' }}>≈ {t('menu.nutrition')}</span>
                <span className="font-medium" style={{ color: 'var(--brand-text-muted)' }}>~{nutritionTotal.kcal} kcal</span>
              </div>
            )}
          </div>
          <div className="pt-4 border-t flex justify-between items-center" style={{ borderColor: 'var(--brand-border)' }}>
            <span className="text-[16px] font-bold" style={{ color: 'var(--brand-text)' }}>{t('cart.total')}</span>
                  <span data-testid="checkout-total"><PriceDisplay amount={total} size="lg" /></span>
          </div>
          {tipAmount > 0 && (
            <div className="flex justify-between items-center text-[13px] mt-2" style={{ color: 'var(--brand-text-muted)' }} data-testid="checkout-cash-due">
              <span>{t('checkout.cash_to_courier', 'Cash to courier (incl. tip)')}</span>
              <PriceDisplay amount={total + tipAmount} />
            </div>
          )}
        </motion.div>
      </form>

      {orderError && (
        <div role="alert" className="p-4 rounded-xl border text-sm flex items-start gap-3" style={{ background: 'var(--color-danger-light)', borderColor: 'var(--color-danger)', color: 'var(--color-danger)' }}>
          <i className="ti ti-alert-triangle text-lg shrink-0 mt-0.5" />
          <div>
            <p className="font-semibold mb-1">Order cannot be placed</p>
            <p>{orderError}</p>
            {showPhoneFallback && fallbackPhone && (
              <a
                href={`tel:${fallbackPhone}`}
                data-testid="checkout-call-restaurant"
                className="inline-flex items-center gap-2 mt-3 px-4 py-2 rounded-full font-semibold text-sm"
                style={{ background: 'var(--brand-primary-strong)', color: '#fff', minHeight: 'var(--tap-min)' }}
              >
                <i className="ti ti-phone" />
                {t('checkout.call_restaurant', 'Call the restaurant')}: {fallbackPhone}
              </a>
            )}
          </div>
        </div>
      )}

      {/* #5 — privacy notice at the point of consent. Warm, plain sq/en/uk; states
          what we collect, who sees it (this restaurant + its courier — truthful to
          tenant isolation), and that identifying details are removed on request via
          the restaurant (anonymize-not-delete; no self-service button, no hard
          retention number the runtime can't yet positively prove). */}
      <div data-testid="checkout-privacy-notice" className="px-4 py-3 rounded-xl border text-xs leading-relaxed" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-muted)' }}>
        <p className="font-semibold mb-1" style={{ color: 'var(--brand-text)' }}>
          <i className="ti ti-lock mr-1" />{t('checkout.privacy.title', 'Your data')}
        </p>
        <p>{t('checkout.privacy.what', 'To deliver your order we collect your name, phone, address and (if you add it) a door photo.')}</p>
        <p className="mt-1">{t('checkout.privacy.who', 'Only this restaurant and its courier can see it — no other restaurant.')}</p>
        <p className="mt-1">{t('checkout.privacy.removal', 'We keep it only as long as needed for your orders and remove the details that identify you on request — contact the restaurant.')}</p>
      </div>

      <StickyActionBar>
        <motion.button
          type="submit"
          form="checkout-form"
          data-testid="order-confirm-button"
          disabled={placing}
          whileTap={{ scale: placing ? 1 : 0.97 }}
          className="w-full h-14 rounded-full bg-[var(--brand-primary-strong)] text-white font-bold text-base shadow-xl transition-all active:scale-[0.97] flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
          style={{ minHeight: 'var(--tap-critical)' }}
        >
          {placing ? (
            <span className="inline-flex items-center gap-2">
              <svg className="animate-spin h-5 w-5" viewBox="0 0 24 24" fill="none">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
              {t('checkout.placing_order', 'Placing order...')}
            </span>
          ) : (
            <>{t('checkout.place_order')} &bull; <PriceDisplay amount={total} size="sm" /></>
          )}
        </motion.button>
      </StickyActionBar>

      <OTPModal
        open={otpOpen}
        onClose={() => { setOtpOpen(false); setPlacing(false); }}
        phone={phone}
        alreadySent
        onResend={sendOtp}
        onVerify={verifyOtp}
      />

      {showConfirmation && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="fixed inset-0 z-[999] flex items-center justify-center pointer-events-none"
          style={{ background: 'color-mix(in srgb, var(--brand-bg) 60%, transparent)' }}
        >
          <motion.div
            initial={{ scale: 0.5, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ type: 'spring', stiffness: 260, damping: 24 }}
            className="flex flex-col items-center gap-3"
          >
            <svg className="w-16 h-16 text-[var(--color-success)]" viewBox="0 0 24 24" fill="none">
              <motion.circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="2" initial={{ pathLength: 0 }} animate={{ pathLength: 1 }} transition={{ duration: 0.4 }} />
              <motion.path d="M8 12l3 3 5-5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" initial={{ pathLength: 0 }} animate={{ pathLength: 1 }} transition={{ delay: 0.2, duration: 0.3 }} />
            </svg>
            <span className="text-2xl font-bold" style={{ color: 'var(--brand-text)' }}>{t('checkout.order_placed', 'Order placed!')} ✓</span>
          </motion.div>
        </motion.div>
      )}

    </div>
  );
}

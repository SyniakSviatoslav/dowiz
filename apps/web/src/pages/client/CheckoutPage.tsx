import { safeStorage } from '../../lib/safeStorage.js';
import React, { useState, useEffect, useRef } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { motion, useReducedMotion } from 'framer-motion';
import { Button, MapWithPin, useI18n, StickyActionBar, PriceDisplay, useCurrency, OTPModal, ease, duration, Select, Textarea, estimateOrderTotal, type OrderTotalConfig } from '@deliveryos/ui';
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
  const { t } = useI18n();
  const prefersReducedMotion = useReducedMotion();
  const CX = 52, R = 38, SW = 10;
  const macros: MacroData[] = [
    { label: t('nutrition.protein', 'Protein'), grams: protein, color: 'var(--chart-protein)', kcalPer: 4 },
    { label: t('nutrition.carbs', 'Carbs'),     grams: carbs,   color: 'var(--chart-carbs)',   kcalPer: 4 },
    { label: t('nutrition.fat', 'Fat'),         grams: fat,     color: 'var(--chart-fat)',     kcalPer: 9 },
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
      initial={prefersReducedMotion ? false : { opacity: 0, y: 8 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-40px' }}
      transition={{ duration: 0.35, ease: ease.out }}
      className="rounded-[var(--brand-radius)] p-4 border"
      style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }}
    >
      <p className="text-step-2xs font-semibold uppercase tracking-wide mb-3" style={{ color: 'var(--brand-text-muted)' }}>
        <i className="ti ti-flame mr-1" aria-hidden="true" />{t('checkout.order_nutrition', 'Order Nutrition')}
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
                transition={{ duration: 0.9, delay: 0.15 + i * 0.18, ease: ease.out }}
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
                transition={{ duration: 1.1, ease: ease.out }}
              />
            )}
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
            <motion.span
              className="text-step-lg font-bold leading-none"
              style={{ color: 'var(--brand-text)' }}
              initial={{ opacity: 0, scale: 0.7 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ delay: 0.55, type: 'spring', stiffness: 220, damping: 18 }}
            >
              {Math.round(kcal)}
            </motion.span>
            <span className="text-step-2xs uppercase tracking-widest mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>kcal</span>
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
              <span className="text-step-xs flex-1" style={{ color: 'var(--brand-text-muted)' }}>{m.label}</span>
              <span className="text-step-sm font-bold tabular-nums" style={{ color: 'var(--brand-text)' }}>{Math.round(m.grams)}g</span>
            </motion.div>
          )) : (
            <motion.p className="text-step-xs" style={{ color: 'var(--brand-text-muted)' }} initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.5 }}>
              ~{Math.round(kcal)} kcal total
            </motion.p>
          )}
        </div>
      </div>
    </motion.div>
  );
}

// §1 flow-simplification: CheckoutPage renders BOTH as the /checkout route (legacy/no-JS) and inside the
// bottom-sheet over the menu (the primary flow). In sheet mode, `onClose` is provided so Back / empty-state
// close the panel (cart intact, no page nav, no trap) instead of navigating the router.
export function CheckoutPage({ onClose }: { onClose?: () => void } = {}) {
  const { slug } = useParams<{ slug: string }>();
  const navigate = useNavigate();
  const { items, clearCart } = useSharedCart();
  const { t } = useI18n();
  const { currency: activeCurrency } = useCurrency();
  const currencySymbol = CURRENCIES[activeCurrency]?.symbol ?? activeCurrency;
  const prefersReducedMotion = useReducedMotion();

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
  // Pickup card shows the REAL venue (name + address) from /info — never a hardcoded address.
  const [pickupName, setPickupName] = useState('');
  const [pickupAddress, setPickupAddress] = useState('');
  // BUG-1: when /info fails, locationId stays null and every Place-Order submit is a
  // silent no-op behind an active-looking button. Track the failure so we can DISABLE
  // the button and show a humane, retryable message instead of failing silently.
  const [locationLoadFailed, setLocationLoadFailed] = useState(false);
  const [locationCenter, setLocationCenter] = useState<LngLatLike>([19.456, 41.324]); // Durrës default
  const [notes, setNotes] = useState('');
  const [cashAmount, setCashAmount] = useState<number>(0);
  const [tipAmount, setTipAmount] = useState<number>(0); // UX-4 optional courier tip
  // Delivery-fee inputs from /info → drives the client total MIRROR (ADR-0005). Defaults degrade
  // safely: until /info loads (or for distance-tiered venues) the fee is "unknown" and we never
  // pre-quote an exact total/cash figure — the server total + the cash-422 backstop are authoritative.
  const [feeInputs, setFeeInputs] = useState<{ deliveryFeeFlat: number | null; freeDeliveryThreshold: number | null; minOrderValue: number | null; taxRate: number; priceIncludesTax: boolean; hasDistanceTiers: boolean } | null>(null);
  const [orderError, setOrderError] = useState('');
  const orderErrorRef = useRef<HTMLDivElement>(null);
  // On a submit failure the form scrolls to top; the error banner sits at the bottom and was
  // off-screen on mobile (read as a "silent" failure). Bring it into view whenever it's set.
  useEffect(() => {
    if (orderError) orderErrorRef.current?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }, [orderError]);
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
    setLocationLoadFailed(false);
    fetch(`/public/locations/${slug}/info`)
      .then(r => { if (!r.ok) throw new Error(`info ${r.status}`); return r.json(); })
      .then((info: any) => {
        if (!info?.id) throw new Error('info: missing id');
        setLocationLoadFailed(false);
        setLocationId(info.id);
        if (info.name) setPickupName(info.name);
        if (info.address) setPickupAddress(info.address);
        if (info.currency_code) setCurrencyCode(info.currency_code);
        setFeeInputs({
          deliveryFeeFlat: info.deliveryFeeFlat ?? null,
          freeDeliveryThreshold: info.freeDeliveryThreshold ?? null,
          minOrderValue: info.minOrderValue ?? null,
          taxRate: typeof info.taxRate === 'number' ? info.taxRate : 0,
          priceIncludesTax: info.priceIncludesTax !== false,
          hasDistanceTiers: info.hasDistanceTiers === true,
        });
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
        setLocationId(null);
        setLocationLoadFailed(true);
      });
    // Cache the restaurant phone NOW (on mount) for the order-failure fallback, so
    // the CTA is available even when the order POST fails under DB/load pressure.
    fetch(`/api/public/locations/${slug}/fallback-config`).then(r => r.json())
      .then((cfg: any) => {
        if (cfg && cfg.showPhoneOnError !== false && cfg.phone) setFallbackPhone(cfg.phone);
      })
      .catch(() => {/* fail-soft: no CTA, generic toast only */});
    try {
      const saved = safeStorage.get(`dos_last_delivery_${slug}`);
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
      const draft = safeStorage.get(`dos_checkout_draft_${slug}`);
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
      safeStorage.set(`dos_checkout_draft_${slug}`, JSON.stringify({
        phone, customerName, deliveryType, instructionOption, instructionCustom, cashAmount,
        entryPhotoKey, entryPhotoPreview, messengerKind, messengerHandle,
      }));
    } catch {}
  }, [slug, phone, customerName, deliveryType, instructionOption, instructionCustom, cashAmount, entryPhotoKey, entryPhotoPreview, messengerKind, messengerHandle]);

  const subtotal = items.reduce((sum, item) => sum + item.price * item.quantity, 0);
  // Client total MIRROR of the server order math (ADR-0005, Approach M). For flat-fee/free/pickup
  // venues this equals the server-charged total to the cent (proven by the parity guardrail); for
  // distance-tiered venues (or before /info loads) feeKnown is false → we show "fee at checkout" and
  // never pre-quote an exact cash figure. The server total + the cash-422 backstop stay authoritative.
  const isPickup = deliveryType === 'pickup';
  const estimateCfg: OrderTotalConfig = {
    isPickup,
    deliveryFeeFlat: feeInputs?.deliveryFeeFlat ?? null,
    freeDeliveryThreshold: feeInputs?.freeDeliveryThreshold ?? null,
    minOrderValue: feeInputs?.minOrderValue ?? null,
    taxRate: feeInputs?.taxRate ?? 0,
    priceIncludesTax: feeInputs?.priceIncludesTax ?? true,
    // Until /info resolves we don't know the fee → treat as unknown (degrade), never a stale hardcode.
    hasDistanceTiers: feeInputs ? feeInputs.hasDistanceTiers : true,
  };
  const estimate = estimateOrderTotal(subtotal, estimateCfg);
  const feeKnown = estimate.feeKnown;
  const deliveryFee = estimate.deliveryFee ?? 0;
  const taxTotal = estimate.taxTotal;
  // When the fee is known, `total` is authoritative-by-construction. When unknown, fall back to a
  // lower-bound (subtotal+tax) used only for display; the cash-422 backstop catches any under-quote.
  const total = estimate.total ?? subtotal + taxTotal;

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
    
    // §3 contextually-required door detail (council-ratified): entrance/apartment are REQUIRED only when the
    // map-pin is LOW-confidence (the customer never placed a precise pin → pinLocation null), because that is
    // exactly when the courier needs the door detail + a clarifying call the least-served customer can't take.
    // When the pin IS placed (high confidence) they are optional — friction lands only where omission causes a
    // failed delivery, never taxing the confident user. Server-tolerant; no contract change.
    const pinIsLowConfidence = pinLocation == null;
    if (deliveryType === 'delivery' && pinIsLowConfidence) {
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
  // `ackCodes` carries soft-signal reason codes we auto-acknowledge on a single silent
  // retry (e.g. a velocity soft-confirm) — a soft speed-bump must never add customer friction.
  // Returns true if the order was created (or OTP flow was started), false on hard error.
  const submitOrder = async (verifiedToken?: string, ackCodes: string[] = []) => {
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
            },
          }),
          payment: { method: 'cash' },
          cash_pay_with: cashAmount > 0 ? cashAmount : undefined,
          idempotency_key: idempotencyKey,
          // Acknowledge the OTP soft-reason once we've verified the phone, plus any soft-signal
          // reasons we auto-acknowledge on the frictionless retry (e.g. velocity).
          acknowledged_codes: [...(verifiedToken ? ['otp_required'] : []), ...ackCodes],
          prefs: {
            dropoff: {
              entrance: entrance.trim(),
              apartment: apartment.trim(),
            }
          },
          // The required "how to find you" notes are location-finding detail for the courier, so
          // they ride in delivery_instructions (the persisted, courier-visible field) — the server
          // delivery schema is .strict() and rejects an unknown `notes` key. Dropoff chip appended.
          delivery_instructions: [
            notes.trim(),
            instructionOption ? (instructionCustom ? `${instructionOption}: ${instructionCustom}` : instructionOption) : '',
          ].filter(Boolean).join(' · ').slice(0, 500) || undefined,
        },
      });

      // Phone verification required but not yet satisfied → start the OTP flow.
      const pre = PreflightResponse.safeParse(raw);
      if (pre.success && pre.data.outcome === 'soft_confirm' && pre.data.requiresOtp) {
        await beginOtpFlow();
        return true;
      }
      // A non-OTP soft_confirm (e.g. the velocity speed-bump for a frequent customer) is a SOFT
      // signal, not a block — auto-acknowledge its reason codes and resubmit ONCE, silently. This
      // keeps a legitimate regular frictionless instead of dead-ending on the generic "order failed"
      // banner. requiresOtp is handled above (a real security gate, never auto-acked); hard_block below.
      // `ackCodes.length === 0` bounds it to a single retry (no loop). S26/anti-friction.
      if (pre.success && pre.data.outcome === 'soft_confirm' && !pre.data.requiresOtp && ackCodes.length === 0) {
        const codes = (pre.data.reasons ?? []).map((r) => r.code).filter(Boolean);
        return submitOrder(verifiedToken, codes.length ? codes : ['velocity']);
      }
      // A 200-body hard_block (item sold out / price changed since the cart was built) must show
      // the designed "review your cart" message — not fall through Zod-parse into the generic
      // "failed to place order" (the customer is never ambushed by a silent change). S11/S14.
      if (pre.success && pre.data.outcome === 'hard_block') {
        const reason = (pre.data as any).reasons?.[0]?.message;
        setOrderError(reason || t('checkout.item_unavailable_error', 'Something in your cart just changed (an item sold out or its price updated). Please review your cart and try again.'));
        return false;
      }

      const orderRes = OrderCreateResponse.parse(raw);
      try {
        safeStorage.set(`dos_last_delivery_${slug}`, JSON.stringify({
          lat: pinLat,
          lng: pinLng,
          address,
          entrance,
          apartment,
        }));
      } catch (err) { console.debug('[CheckoutPage] localStorage write failed:', err); }
      try { safeStorage.remove(`dos_checkout_draft_${slug}`); } catch {}
      if (orderRes.authToken) {
        safeStorage.set('dos_access_token', orderRes.authToken);
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
      // Make the known business rejections feel DESIGNED, not a cold "failed" — and never let a
      // price/availability change ambush the customer at submit (S11/S14 surprise). Server-read-only.
      if (err?.status === 422 || err?.status === 409) {
        const code = err?.data?.code;
        if (code === 'NOT_DELIVERABLE') {
          setOrderError(t('checkout.not_deliverable_error', 'This address is outside the delivery area. Try pickup or a different address.'));
          return false;
        }
        // Door-handover-parity backstop (ADR-0005): the cash the customer pledged is below the
        // server-authoritative total (e.g. a distance-tiered fee we couldn't pre-quote). Never let a
        // wrong cash amount through — ask for the correct one rather than collect the wrong sum.
        if (code === 'CASH_AMOUNT_TOO_LOW') {
          setOrderError(t('checkout.cash_too_low_error', 'The cash amount is below the final total (including the delivery fee). Please increase it and try again.'));
          return false;
        }
        if (code === 'item_unavailable' || err?.data?.outcome === 'hard_block') {
          setOrderError(t('checkout.item_unavailable_error', 'Something in your cart just changed (an item sold out or its price updated). Please review your cart and try again.'));
          return false;
        }
        // The server often phrases the rejection humanely already — surface that over a generic.
        const serverMsg = err?.data?.reasons?.[0]?.message || (typeof err?.data?.message === 'string' ? err.data.message : null);
        if (serverMsg) { setShowPhoneFallback(true); setOrderError(serverMsg); return false; }
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
      <motion.div
        initial={prefersReducedMotion ? false : { opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.25, ease: ease.out }}
        className="flex flex-col items-center justify-center min-h-[60vh] text-center px-6 py-12 gap-4 max-w-sm mx-auto"
      >
        <div
          className="flex items-center justify-center w-20 h-20 rounded-full"
          style={{ background: 'var(--brand-surface-raised)', boxShadow: 'var(--elev-1)' }}
        >
          <i className="ti ti-shopping-cart-off text-3xl" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
        </div>
        <h2 className="text-xl font-bold" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>
          {t('checkout.empty_title', 'Your cart is empty')}
        </h2>
        <p className="text-sm leading-relaxed" style={{ color: 'var(--brand-text-muted)' }}>
          {t('checkout.empty_body', 'Add a few dishes to get started — your order will appear here.')}
        </p>
        <Button onClick={() => onClose ? onClose() : navigate(`/s/${slug}`)} className="mt-1">
          <i className="ti ti-arrow-left mr-2" aria-hidden="true" />
          {t('checkout.browse_menu', 'Browse menu')}
        </Button>
      </motion.div>
    );
  }

  return (
    <div className="max-w-xl mx-auto p-4 md:py-8 space-y-6 pb-32">
      {!onClose && (
        <div className="flex items-center gap-3 mb-6">
          <motion.button onClick={() => navigate(-1)} whileTap={{ scale: 0.95 }} aria-label={t('common.back', 'Go back')} className="w-11 h-11 shrink-0 rounded-full flex items-center justify-center border transition-[transform,box-shadow,background-color] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-95 hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
            <i className="ti ti-arrow-left" aria-hidden="true" />
          </motion.button>
          <h1 className="text-step-2xl font-bold min-w-0 truncate" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.title')}</h1>
        </div>
      )}

      <form id="checkout-form" onSubmit={handlePlaceOrder} className="space-y-6">
        <motion.div
          initial={prefersReducedMotion ? false : { opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-40px' }}
          transition={{ duration: 0.25, ease: ease.out }}
          className="rounded-[var(--brand-radius)] p-4 border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }}>
          <h2 className="text-step-xl font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.contact_info', 'Contact Info')}</h2>
          <div className="space-y-3">
            <div>
              <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.name', 'Name')}</label>
              <div className="relative">
                <i className="ti ti-user absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                <input required value={customerName} onChange={e => setCustomerName(e.target.value)} placeholder={t('checkout.name_placeholder', 'Your name')} autoComplete="name" className="w-full h-[48px] pl-10 pr-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
              </div>
            </div>
            <div>
              <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.phone', 'Phone')}</label>
              <div className="relative">
                <i className="ti ti-phone absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                <input required value={phone} onChange={e => { setPhone(e.target.value); setPhoneError(''); }} onBlur={() => setPhone(p => normalizeAlbanianPhone(p))} placeholder="+355 6X XXX XXXX" title="+355 followed by 7-14 digits" type="tel" inputMode="tel" autoComplete="tel" data-testid="checkout-phone" className="w-full h-[48px] pl-10 pr-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface-raised)', borderColor: phoneError ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
                {phoneError && <p role="alert" className="text-step-xs mt-1" style={{ color: 'var(--color-danger)' }}>{phoneError}</p>}
              </div>
            </div>
            {/* UX-2: optional messenger so the courier can text instead of call */}
            <div>
              <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.messenger', 'Messenger (optional)')}</label>
              <div className="flex gap-2">
                <Select value={messengerKind} onChange={e => setMessengerKind(e.target.value)} aria-label={t('checkout.messenger', 'Messenger (optional)')} data-testid="checkout-messenger-kind">
                  <option value="">{t('checkout.messenger_none', '—')}</option>
                  <option value="telegram">Telegram</option>
                  <option value="whatsapp">WhatsApp</option>
                  <option value="viber">Viber</option>
                </Select>
                <input value={messengerHandle} onChange={e => setMessengerHandle(e.target.value)} disabled={!messengerKind}
                  required={!!messengerKind}
                  aria-label={t('checkout.messenger_handle_label', 'Your messenger handle')}
                  placeholder={messengerKind === 'telegram' ? '@username' : '+355 6X XXX XXXX'} data-testid="checkout-messenger-handle"
                  className="flex-1 min-w-0 h-[48px] px-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)] disabled:opacity-50" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
              </div>
            </div>
            {/* UX-3: optional entrance photo (delivery only) — camera or gallery */}
            {deliveryType !== 'pickup' && (
              <div>
                <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.entry_photo', 'Entrance photo (optional)')}</label>
                <div className="flex items-center gap-3">
                  <button type="button" onClick={() => entryFileRef.current?.click()} disabled={photoUploading}
                    className="inline-flex items-center gap-2 min-h-[44px] px-4 py-2 border rounded-[var(--brand-radius-sm)] cursor-pointer text-sm transition-[background-color,box-shadow,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 disabled:opacity-60"
                    style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
                    <i className="ti ti-camera" aria-hidden="true" />
                    {photoUploading ? t('checkout.uploading', 'Uploading…') : (entryPhotoKey ? t('checkout.change_photo', 'Change photo') : t('checkout.add_photo', 'Add photo'))}
                  </button>
                  <input ref={entryFileRef} type="file" accept="image/*" className="hidden" data-testid="entry-photo-input" disabled={photoUploading}
                    onChange={e => { const f = (e.target as HTMLInputElement).files?.[0]; if (f) void uploadEntryPhoto(f); }} />
                  {entryPhotoPreview && (
                    <img src={entryPhotoPreview} alt={t('checkout.entry_photo', 'Entrance photo')} data-testid="entry-photo-preview" className="h-12 w-12 object-cover rounded-[var(--brand-radius-sm)] border" style={{ borderColor: 'var(--brand-border)' }} />
                  )}
                </div>
                <p className="text-step-xs mt-1" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.entry_photo_hint', 'Helps the courier find your entrance.')}</p>
              </div>
            )}
          </div>
        </motion.div>
        <motion.div
          initial={prefersReducedMotion ? false : { opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-40px' }}
          transition={{ duration: 0.25, ease: ease.out, delay: 0.05 }}
          className="rounded-[var(--brand-radius)] p-4 border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }}>
          <h2 className="text-step-xl font-semibold mb-6" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.delivery_address')}</h2>
          {/* §4 flow-simplification: order-type switch removed — delivery is the only live type (pickup/scheduled
              deferred). deliveryType stays 'delivery' (the switch + pickup branches restore with the capability),
              the payload still sends a valid type → no order-contract change. */}

          {deliveryType === 'delivery' && (
            <div className="space-y-4">
              <div>
                <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.pin_on_map', 'Drag the pin to your location')}</label>
                <MapWithPin className="h-48 w-full rounded-[var(--brand-radius-sm)]" initialCenter={locationCenter} onPinChange={setPinLocation} confirmLabel={t('common.confirm')} placeholder={t('checkout.pin_on_map', 'Drag the pin to your location')} />
              </div>
              <div>
                <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.street_address', 'Street address')}</label>
                <div className="relative">
                  <i className="ti ti-map-pin absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                  <input required value={address} onChange={e => setAddress(e.target.value)} data-testid="checkout-address" placeholder={t('checkout.street_address', 'Street address')} className="w-full h-[48px] pl-10 pr-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                </div>
              </div>
              <div className="space-y-4">
                <div>
                  <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.entrance')}{pinLocation != null && <span className="font-normal ml-1" style={{ color: 'var(--brand-text-muted)' }}>{t('common.optional', '(optional)')}</span>}</label>
                  <div className="relative">
                    <i className="ti ti-door-open absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                    <input required={pinLocation == null} value={entrance} onChange={e => setEntrance(e.target.value)} data-testid="checkout-entrance" placeholder={t('checkout.entrance_placeholder', 'Entrance number or name')} className="w-full h-[48px] pl-10 pr-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  </div>
                  {entranceError && <p role="alert" className="text-step-xs mt-1" style={{ color: 'var(--color-danger)' }}>{entranceError}</p>}
                </div>
                <div>
                  <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.apartment')}{pinLocation != null && <span className="font-normal ml-1" style={{ color: 'var(--brand-text-muted)' }}>{t('common.optional', '(optional)')}</span>}</label>
                  <div className="relative">
                    <i className="ti ti-apartment absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                    <input required={pinLocation == null} value={apartment} onChange={e => setApartment(e.target.value)} data-testid="checkout-apartment" placeholder={t('checkout.apartment_placeholder', 'Apartment or unit number')} className="w-full h-[48px] pl-10 pr-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  </div>
                  {apartmentError && <p role="alert" className="text-step-xs mt-1" style={{ color: 'var(--color-danger)' }}>{apartmentError}</p>}
                </div>
              </div>
              <div>
                <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>
                  {t('checkout.notes', 'How to find you')} <span style={{ color: 'var(--color-danger)' }}>*</span>
                </label>
                <div className="relative">
                  <i className="ti ti-map-2 absolute left-3 top-3 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                  <Textarea
                    required
                    value={notes}
                    onChange={e => setNotes(e.target.value)}
                    rows={3}
                    placeholder={t('checkout.notes_placeholder', 'Describe how to find the exact place: floor, building color, nearby landmark, gate code...')}
                    className="pl-10"
                  />
                </div>
              </div>
              <div>
                <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.dropoff_instructions', 'Dropoff instructions')}</label>
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
                      className="px-3 py-1.5 text-step-xs rounded-[var(--brand-radius-btn)] border transition-[background-color,border-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1"
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
                    <input value={instructionCustom} onChange={e => setInstructionCustom(e.target.value)} placeholder={t('checkout.extra_notes', 'Extra notes...')} className="w-full h-[44px] pl-10 pr-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  </div>
                )}
              </div>
            </div>
          )}

          {deliveryType === 'pickup' && (
            <div className="space-y-4">
              <div className="border rounded-[var(--brand-radius)] p-4" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
                <h3 className="text-step-sm font-bold mb-1" style={{ color: 'var(--brand-text)' }}>{t('courier.pickup')}</h3>
                <p className="text-step-sm mb-4" style={{ color: 'var(--brand-text-muted)' }}>
                  {pickupName && <span className="block font-semibold" style={{ color: 'var(--brand-text)' }}>{pickupName}</span>}
                  {pickupAddress || t('checkout.pickup_addr_tbd', 'Address shown after the restaurant confirms.')}
                </p>
                <div className="w-full h-[120px] rounded-[var(--brand-radius-sm)] relative overflow-hidden border flex items-center justify-center" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)' }}>
                  <div className="absolute inset-0 opacity-10" style={{ backgroundImage: 'linear-gradient(var(--brand-border) 1px, transparent 1px), linear-gradient(90deg, var(--brand-border) 1px, transparent 1px)', backgroundSize: '20px 20px' }} />
                  <i className="ti ti-building-store text-3xl relative z-10" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                </div>
              </div>
              <div className="flex items-center gap-3 p-3 rounded-[var(--brand-radius-sm)] border" style={{ background: 'var(--color-info-light)', borderColor: 'var(--color-info)', color: 'var(--color-info)' }}>
                <i className="ti ti-info-circle" aria-hidden="true" />
                <p className="text-step-sm font-medium">{t('checkout.phone_hint')}</p>
              </div>
            </div>
          )}

          {deliveryType === 'scheduled' && (
            <div className="flex items-center gap-3 p-4 rounded-[var(--brand-radius)] border" style={{ background: 'var(--color-warning-light)', borderColor: 'var(--color-warning)' }}>
              <i className="ti ti-clock text-lg shrink-0" aria-hidden="true" style={{ color: 'var(--color-warning)' }} />
              <p className="text-step-sm font-medium" style={{ color: 'var(--brand-text)' }}>
                {t('checkout.scheduled_coming_soon', 'Scheduled delivery coming soon. Please select Delivery or Pickup.')}
              </p>
            </div>
          )}
        </motion.div>

        <motion.div
          initial={prefersReducedMotion ? false : { opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-40px' }}
          transition={{ duration: 0.25, ease: ease.out, delay: 0.1 }}
          className="rounded-[var(--brand-radius)] p-4 border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }}>
          <h2 className="text-step-xl font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.payment_method')}</h2>
          <div className="border rounded-[var(--brand-radius-sm)] p-3 mb-3" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-primary)' }}>
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <i className="ti ti-cash text-xl" aria-hidden="true" style={{ color: 'var(--brand-primary)' }} />
                <div>
                  <div className="text-step-sm font-bold" style={{ color: 'var(--brand-text)' }}>{t('checkout.cash')}</div>
                  <div className="text-step-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.place_order')}</div>
                </div>
              </div>
              <i className="ti ti-check" aria-hidden="true" style={{ color: 'var(--brand-primary)' }} />
            </div>
            <div className="mt-3 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
              <label htmlFor="cash-amount" className="text-step-xs font-semibold mb-1.5 block" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.cash_amount', 'Cash amount')}</label>
              <div className="flex gap-2">
                <div className="relative flex-1">
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-step-sm font-bold" style={{ color: 'var(--brand-text-muted)' }}>{currencySymbol}</span>
                  <input
                    id="cash-amount"
                    type="number"
                    inputMode="decimal"
                    min={total}
                    value={cashAmount || ''}
                    onChange={e => setCashAmount(parseInt(e.target.value) || 0)}
                    className="w-full h-[44px] pl-11 pr-3 outline-none text-step-sm font-bold border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]"
                    style={{ background: 'var(--brand-surface)', borderColor: cashAmount > 0 && cashAmount < total ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }}
                    placeholder={String(total)}
                  />
                </div>
              </div>
              {/* UX-4: optional courier tip (single amount, replaces %-badges) */}
              <div className="mt-3 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
                <label htmlFor="tip-amount" className="text-step-xs font-semibold mb-1.5 block" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.tip_amount', 'Tip for courier (optional)')}</label>
                <div className="relative">
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-step-sm font-bold" style={{ color: 'var(--brand-text-muted)' }}>{currencySymbol}</span>
                  <input
                    id="tip-amount"
                    type="number"
                    inputMode="decimal"
                    min={0}
                    max={1000000}
                    value={tipAmount || ''}
                    data-testid="checkout-tip"
                    onChange={e => setTipAmount(Math.min(1000000, Math.max(0, parseInt(e.target.value) || 0)))}
                    className="w-full h-[44px] pl-11 pr-3 outline-none text-step-sm font-bold border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]"
                    style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
                    placeholder="0"
                  />
                </div>
                <p className="text-step-xs mt-1" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.tip_hint', 'Goes entirely to your courier, in cash on delivery.')}</p>
              </div>
              {cashAmount > 0 && (
                <div className="flex justify-between text-step-sm mt-2 px-1">
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
            initial={prefersReducedMotion ? false : { opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: duration.slow, ease: ease.out }}
            className="rounded-[var(--brand-radius)] p-4 border" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)' }}>
            <div className="flex items-start gap-3">
              <span className="text-xl shrink-0 mt-0.5">🌍</span>
              <div>
                <p className="text-step-2xs font-semibold uppercase tracking-wide mb-1" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.did_you_know', 'Did you know?')}</p>
                <p className="text-step-sm leading-relaxed" style={{ color: 'var(--brand-text)' }}>{cityFact}</p>
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
          initial={prefersReducedMotion ? false : { opacity: 0, y: 12 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: '-40px' }}
          transition={{ duration: 0.25, ease: ease.out, delay: 0.15 }}
          className="rounded-[var(--brand-radius)] p-4 border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }}>
          <h2 className="text-step-xl font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('order.title')}</h2>
          <div className="space-y-3 mb-4">
            <div className="flex justify-between items-baseline gap-3 text-step-sm">
              <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('cart.subtotal')}</span>
              <span className="shrink-0 tabular-nums"><PriceDisplay amount={subtotal} /></span>
            </div>
            {deliveryType === 'delivery' && (
              <div className="flex justify-between items-baseline gap-3 text-step-sm">
                <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('cart.delivery_fee')}</span>
                {feeKnown ? (
                  <span className="shrink-0 tabular-nums"><PriceDisplay amount={deliveryFee} /></span>
                ) : (
                  // Distance-tiered venue — the fee depends on the delivery address and is finalised by
                  // the server. We never invent a number we can't collect at the door (ADR-0005).
                  <span className="shrink-0 text-step-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.fee_at_checkout', 'Calculated at checkout')}</span>
                )}
              </div>
            )}
            {taxTotal > 0 && (
              <div className="flex justify-between items-baseline gap-3 text-step-sm">
                <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('cart.tax', 'Tax')}</span>
                <span className="shrink-0 tabular-nums"><PriceDisplay amount={taxTotal} /></span>
              </div>
            )}
            {tipAmount > 0 && (
              <div className="flex justify-between items-baseline gap-3 text-step-sm" data-testid="checkout-tip-line">
                <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.tip_for_courier', 'Tip for courier (cash)')}</span>
                <span className="shrink-0 tabular-nums"><PriceDisplay amount={tipAmount} /></span>
              </div>
            )}
            {hasNutrition && (
              <div className="flex justify-between items-baseline gap-3 text-step-xs">
                <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>≈ {t('menu.nutrition')}</span>
                <span className="shrink-0 font-medium tabular-nums" style={{ color: 'var(--brand-text-muted)' }}>~{nutritionTotal.kcal} kcal</span>
              </div>
            )}
          </div>
          <div className="pt-4 border-t flex justify-between items-center gap-3" style={{ borderColor: 'var(--brand-border)' }}>
            <span className="text-step-base font-bold min-w-0 truncate" style={{ color: 'var(--brand-text)' }}>{t('cart.total')}</span>
            <motion.span
              key={total}
              data-testid="checkout-total"
              className="shrink-0 tabular-nums"
              initial={prefersReducedMotion ? false : { opacity: 0.4, y: -4 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: duration.base, ease: ease.out }}
            >
              <PriceDisplay amount={total} size="lg" />
            </motion.span>
          </div>
          {deliveryType === 'delivery' && !feeKnown && (
            <p className="text-step-xs mt-1 text-right" style={{ color: 'var(--brand-text-muted)' }}>
              {t('checkout.plus_delivery_fee', '+ delivery fee, calculated at checkout')}
            </p>
          )}
          {tipAmount > 0 && (
            <div className="flex justify-between items-center gap-3 text-step-sm mt-2" style={{ color: 'var(--brand-text-muted)' }} data-testid="checkout-cash-due">
              <span className="min-w-0 truncate">{t('checkout.cash_to_courier', 'Cash to courier (incl. tip)')}</span>
              <span className="shrink-0 tabular-nums"><PriceDisplay amount={total + tipAmount} /></span>
            </div>
          )}
          {/* Pre-order ETA — there is no order yet, so this is a deliberately WIDE
              approximate range that refines once the order is placed (the status page
              then shows the honest server range). Delivery only; pickup has no ETA. */}
          {deliveryType === 'delivery' && (
            <div className="mt-4 pt-4 border-t flex items-start gap-2.5" style={{ borderColor: 'var(--brand-border)' }} data-testid="checkout-eta-estimate">
              <i className="ti ti-clock text-lg shrink-0 mt-0.5" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
              <div className="min-w-0">
                <div className="text-step-sm font-semibold tabular-nums" style={{ color: 'var(--brand-text)' }}>
                  {t('order.eta_range', '{{low}}–{{high}} min', { low: 25, high: 45 })}
                </div>
                <p className="text-step-xs leading-snug" style={{ color: 'var(--brand-text-muted)' }}>
                  {t('checkout.eta_estimate', 'Estimated time — refines after you place the order')}
                </p>
              </div>
            </div>
          )}
        </motion.div>
      </form>

      {locationLoadFailed && (
        <div role="alert" data-testid="checkout-location-load-failed" className="p-4 rounded-[var(--brand-radius)] border text-sm flex items-start gap-3" style={{ background: 'var(--color-danger-light)', borderColor: 'var(--color-danger)', color: 'var(--color-danger)' }}>
          <i className="ti ti-alert-triangle text-lg shrink-0 mt-0.5" />
          <div className="flex-1">
            <p>{t('checkout.location_load_failed', "We couldn't load this restaurant — please refresh and try again.")}</p>
            <button
              type="button"
              onClick={() => window.location.reload()}
              data-testid="checkout-location-retry"
              className="inline-flex items-center gap-2 mt-3 px-4 py-2 rounded-full font-semibold text-sm transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
              style={{ background: 'var(--brand-primary-strong)', color: 'var(--brand-bg)', minHeight: 'var(--tap-min)' }}
            >
              <i className="ti ti-refresh" aria-hidden="true" />
              {t('common.refresh', 'Refresh')}
            </button>
          </div>
          <button
            type="button"
            onClick={() => setLocationLoadFailed(false)}
            aria-label={t('common.dismiss', 'Dismiss')}
            className="shrink-0 w-8 h-8 -mt-1 -mr-1 flex items-center justify-center rounded-full transition-[transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-danger)] focus-visible:ring-offset-1"
            style={{ color: 'var(--color-danger)' }}
          >
            <i className="ti ti-x text-lg" aria-hidden="true" />
          </button>
        </div>
      )}

      {orderError && (
        <div ref={orderErrorRef} role="alert" className="p-4 rounded-[var(--brand-radius)] border text-sm flex items-start gap-3" style={{ background: 'var(--color-danger-light)', borderColor: 'var(--color-danger)', color: 'var(--color-danger)' }}>
          <i className="ti ti-alert-triangle text-lg shrink-0 mt-0.5" aria-hidden="true" />
          <div className="min-w-0">
            <p className="font-semibold mb-1">{t('checkout.cannot_place', 'Order cannot be placed')}</p>
            <p className="break-words">{orderError}</p>
            {showPhoneFallback && fallbackPhone && (
              <a
                href={`tel:${fallbackPhone}`}
                data-testid="checkout-call-restaurant"
                className="inline-flex items-center gap-2 mt-3 px-4 py-2 rounded-full font-semibold text-sm transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                style={{ background: 'var(--brand-primary-strong)', color: 'var(--brand-bg)', minHeight: 'var(--tap-min)' }}
              >
                <i className="ti ti-phone" aria-hidden="true" />
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
      <div data-testid="checkout-privacy-notice" className="px-4 py-3 rounded-[var(--brand-radius)] border text-xs leading-relaxed" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
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
          disabled={placing || !locationId}
          whileTap={{ scale: (placing || !locationId) ? 1 : 0.97 }}
          className="w-full h-14 rounded-full bg-[var(--brand-primary-strong)] text-[var(--brand-bg)] font-bold text-base flex items-center justify-center gap-2 transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.97] hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed disabled:translate-y-0"
          style={{ minHeight: 'var(--tap-critical)', boxShadow: 'var(--elev-3)' }}
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
            initial={prefersReducedMotion ? { opacity: 0 } : { scale: 0.5, opacity: 0 }}
            animate={prefersReducedMotion ? { opacity: 1 } : { scale: 1, opacity: 1 }}
            transition={prefersReducedMotion ? { duration: 0.2 } : { type: 'spring', stiffness: 260, damping: 24 }}
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

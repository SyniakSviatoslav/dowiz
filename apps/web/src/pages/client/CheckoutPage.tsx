import { safeStorage } from '../../lib/safeStorage.js';
import React, { useState, useEffect, useRef } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { motion, useReducedMotion } from 'framer-motion';
import { Button, useI18n, StickyActionBar, PriceDisplay, useCurrency, OTPModal, ease, estimateOrderTotal, type OrderTotalConfig } from '@deliveryos/ui';
import { CURRENCIES } from '@deliveryos/shared-types';
import type { LngLatLike } from '@deliveryos/ui';
import { PHONE_E164_REGEX } from '@deliveryos/shared-types';
import { messengerIsPhone } from '../../lib/messenger.js';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';
import { normalizeAlbanianPhone } from './checkout/phone.js';
import { requestPushPermission } from './checkout/push.js';
import { OrderSummaryAccordion } from './checkout/OrderSummaryAccordion.js';
import { ContactInfoSection } from './checkout/ContactInfoSection.js';
import { DeliveryDetailsSection } from './checkout/DeliveryDetailsSection.js';
import { PaymentSection } from './checkout/PaymentSection.js';
import { OrderSummarySection } from './checkout/OrderSummarySection.js';
import { useVenueInfo } from './checkout/useVenueInfo.js';
import { useFallbackPhone } from './checkout/useFallbackPhone.js';
import { useOrderMenuMap } from './checkout/useOrderMenuMap.js';
import type { DeliveryType } from './checkout/types.js';

const OrderCreateResponse = z.object({
  id: z.string(),
  authToken: z.string().optional(),
  // Crypto prepaid (ADR-0017): when present, redirect the customer to the hosted invoice to pay.
  payment: z.object({ method: z.string(), redirectUrl: z.string() }).optional(),
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

  // Crypto prepaid (ADR-0017) — DARK behind VITE_PAYMENTS_CRYPTO_ENABLED. Cash-on-delivery stays the default.
  const CRYPTO_ENABLED = import.meta.env.VITE_PAYMENTS_CRYPTO_ENABLED === 'true';
  const [paymentMethod, setPaymentMethod] = useState<'cash' | 'crypto'>('cash');
  // productId → { image, per-unit nutrition } from the public menu (thumbnails + combined nutrition).
  const orderMenuMap = useOrderMenuMap(slug);

  const [deliveryType, setDeliveryType] = useState<DeliveryType>('delivery');
  const [address, setAddress] = useState('');
  const [phone, setPhone] = useState('');
  const [customerName, setCustomerName] = useState('');
  // UX-2 optional messenger contact (Telegram/WhatsApp/Viber) so the courier can text.
  const [messengerKind, setMessengerKind] = useState('');
  const [messengerHandle, setMessengerHandle] = useState('');
  const [commError, setCommError] = useState('');
  // "Deliver to someone else" — same-receiver checked by default (ships to the customer).
  const [sameReceiver, setSameReceiver] = useState(true);
  const [receiverName, setReceiverName] = useState('');
  const [receiverKind, setReceiverKind] = useState('');
  const [receiverHandle, setReceiverHandle] = useState('');
  // UX-3 optional entry-anchor photo (uploaded to R2 before the order exists).
  const [entryPhotoKey, setEntryPhotoKey] = useState('');
  const [entryPhotoPreview, setEntryPhotoPreview] = useState('');
  const [photoUploading, setPhotoUploading] = useState(false);
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
  // Venue identity + fee inputs from /info (ADR-0005 client MIRROR), including the
  // BUG-1 locationLoadFailed flag that disables the Place-Order button on failure.
  const {
    locationId,
    pickupName,
    pickupAddress,
    locationLoadFailed,
    setLocationLoadFailed,
    locationCenter,
    feeInputs,
    currencyCode,
  } = useVenueInfo(slug);
  const [notes, setNotes] = useState('');
  const [cashAmount, setCashAmount] = useState<number>(0);
  const [tipAmount, setTipAmount] = useState<number>(0); // UX-4 optional courier tip
  const [orderError, setOrderError] = useState('');
  const orderErrorRef = useRef<HTMLDivElement>(null);
  // On a submit failure the form scrolls to top; the error banner sits at the bottom and was
  // off-screen on mobile (read as a "silent" failure). Bring it into view whenever it's set.
  useEffect(() => {
    if (orderError) orderErrorRef.current?.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }, [orderError]);
  // #4 — restaurant phone for the failure fallback (cached on mount by the hook).
  const fallbackPhone = useFallbackPhone(slug);
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
  // Phone-verification (OTP) state — only engaged when the backend signals
  // `requiresOtp` on a soft_confirm. Otherwise checkout behaves exactly as before.
  const [otpOpen, setOtpOpen] = useState(false);
  const [otpToken, setOtpToken] = useState<string | null>(null);

  useEffect(() => {
    if (!slug) return;
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

  // Per-unit nutrition comes from the fetched menu map (cart items don't carry it); fall back to any
  // fields a cart item happens to have. Combined = Σ per-unit × quantity.
  const nutritionOf = (item: any) => orderMenuMap[item.productId] ?? { kcal: (item as any).kcal ?? 0, protein: (item as any).protein ?? 0, fat: (item as any).fat ?? 0, carbs: (item as any).carbs ?? 0 };
  const nutritionTotal = items.reduce((acc, item: any) => {
    const n = nutritionOf(item);
    return { kcal: acc.kcal + (n.kcal ?? 0) * item.quantity, protein: acc.protein + (n.protein ?? 0) * item.quantity, fat: acc.fat + (n.fat ?? 0) * item.quantity, carbs: acc.carbs + (n.carbs ?? 0) * item.quantity };
  }, { kcal: 0, protein: 0, fat: 0, carbs: 0 });
  const hasNutrition = nutritionTotal.kcal > 0;

  const orderItems = items.map(i => ({
    product_id: i.productId,
    quantity: i.quantity,
    modifier_ids: Object.values(i.options || {}).flat() as string[],
  }));

    const handlePlaceOrder = async (e: React.FormEvent) => {
    e.preventDefault();
    setOrderError('');
    if (items.length === 0 || !slug || !locationId) return;
    // Communication is REQUIRED (ADR-0016). Phone-yielding kinds (phone/whatsapp/viber/signal) validate as a
    // phone — that value also flows to `customer.phone` so the per-phone throttle/OTP/dedup keep working.
    // Telegram (username) / SimpleX (text) are phone-less by design.
    if (!messengerKind) {
      setCommError(t('checkout.comm_required', 'Choose how the courier can reach you'));
      return;
    }
    setCommError('');
    if (messengerIsPhone(messengerKind)) {
      const e164 = normalizeAlbanianPhone(phone);
      if (!e164 || !PHONE_E164_REGEX.test(e164)) {
        setPhoneError(t('checkout.phone_invalid', 'Enter a valid phone number (+355...)'));
        return;
      }
      if (e164 !== phone) setPhone(e164);
      setPhoneError('');
    } else {
      if (!messengerHandle.trim()) {
        setCommError(messengerKind === 'telegram'
          ? t('checkout.comm_username_required', 'Enter your @username')
          : t('checkout.comm_handle_required', 'Enter your contact'));
        return;
      }
      setPhoneError('');
    }
    // Receiver ("deliver to someone else") — when not the customer, require name + channel + handle.
    if (!sameReceiver && (!receiverName.trim() || !receiverKind || !receiverHandle.trim())) {
      setCommError(t('checkout.receiver_required', 'Add the receiver’s name and how to reach them'));
      return;
    }

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
            // Phone-yielding kinds carry a real phone (→ per-phone throttle/OTP/dedup). Telegram/SimpleX are
            // phone-less (omit → server makes no customer row, IP-throttle floor) — ADR-0016.
            phone: messengerIsPhone(messengerKind) ? normalizeAlbanianPhone(phone) : undefined,
            name: customerName || undefined,
            messenger_kind: messengerKind,
            messenger_handle: messengerIsPhone(messengerKind) ? normalizeAlbanianPhone(phone) : messengerHandle.trim(),
          },
          ...(sameReceiver ? {} : {
            receiver: { name: receiverName.trim(), messenger_kind: receiverKind, handle: receiverHandle.trim() },
          }),
          ...(entryPhotoKey ? { delivery_photo_key: entryPhotoKey } : {}),
          ...(tipAmount > 0 ? { tip_amount: tipAmount } : {}),
          // Pickup orders carry no delivery pin/address (no delivery fee).
          ...(deliveryType === 'pickup' ? {} : {
            delivery: {
              pin: { lat: pinLat, lng: pinLng },
              address_text: address || undefined,
            },
          }),
          payment: { method: paymentMethod },
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
      // Crypto prepaid: send the customer to the hosted invoice to pay (the order is held until confirmed).
      if (orderRes.payment?.redirectUrl) {
        window.location.href = orderRes.payment.redirectUrl;
        return true;
      }
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
        <OrderSummaryAccordion
          items={items}
          total={total}
          orderMenuMap={orderMenuMap}
          hasNutrition={hasNutrition}
          nutritionTotal={nutritionTotal}
        />

        <ContactInfoSection
          deliveryType={deliveryType}
          customerName={customerName}
          setCustomerName={setCustomerName}
          phone={phone}
          setPhone={setPhone}
          phoneError={phoneError}
          setPhoneError={setPhoneError}
          commError={commError}
          setCommError={setCommError}
          messengerKind={messengerKind}
          setMessengerKind={setMessengerKind}
          messengerHandle={messengerHandle}
          setMessengerHandle={setMessengerHandle}
          sameReceiver={sameReceiver}
          setSameReceiver={setSameReceiver}
          receiverName={receiverName}
          setReceiverName={setReceiverName}
          receiverKind={receiverKind}
          setReceiverKind={setReceiverKind}
          receiverHandle={receiverHandle}
          setReceiverHandle={setReceiverHandle}
          entryPhotoKey={entryPhotoKey}
          entryPhotoPreview={entryPhotoPreview}
          photoUploading={photoUploading}
          uploadEntryPhoto={uploadEntryPhoto}
        />

        <DeliveryDetailsSection
          deliveryType={deliveryType}
          locationCenter={locationCenter}
          pinLocation={pinLocation}
          setPinLocation={setPinLocation}
          address={address}
          setAddress={setAddress}
          entrance={entrance}
          setEntrance={setEntrance}
          entranceError={entranceError}
          apartment={apartment}
          setApartment={setApartment}
          apartmentError={apartmentError}
          notes={notes}
          setNotes={setNotes}
          instructionOption={instructionOption}
          setInstructionOption={setInstructionOption}
          instructionCustom={instructionCustom}
          setInstructionCustom={setInstructionCustom}
          pickupName={pickupName}
          pickupAddress={pickupAddress}
        />

        <PaymentSection
          currencySymbol={currencySymbol}
          total={total}
          cashAmount={cashAmount}
          setCashAmount={setCashAmount}
          tipAmount={tipAmount}
          setTipAmount={setTipAmount}
        />

        <OrderSummarySection
          deliveryType={deliveryType}
          subtotal={subtotal}
          feeKnown={feeKnown}
          deliveryFee={deliveryFee}
          taxTotal={taxTotal}
          tipAmount={tipAmount}
          total={total}
          hasNutrition={hasNutrition}
          nutritionKcal={nutritionTotal.kcal}
        />

        {/* Payment method (ADR-0017) — DARK behind VITE_PAYMENTS_CRYPTO_ENABLED. Cash-on-delivery default. */}
        {CRYPTO_ENABLED && (
          <div className="rounded-[var(--brand-radius)] p-4 border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }} data-testid="checkout-payment">
            <h2 className="text-step-xl font-semibold mb-3" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.payment', 'Payment')}</h2>
            <div className="grid grid-cols-2 gap-2" role="radiogroup" aria-label={t('checkout.payment', 'Payment')}>
              {(['cash', 'crypto'] as const).map(m => {
                const active = paymentMethod === m;
                return (
                  <button key={m} type="button" role="radio" aria-checked={active} data-testid={`pay-method-${m}`}
                    onClick={() => setPaymentMethod(m)}
                    className="flex items-center gap-2 h-[48px] px-3 rounded-[var(--brand-radius-sm)] border text-step-sm font-semibold focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)]"
                    style={{ background: active ? 'var(--brand-primary)' : 'var(--brand-surface-raised)', borderColor: active ? 'var(--brand-primary)' : 'var(--brand-border)', color: active ? 'var(--brand-primary-readable, #fff)' : 'var(--brand-text)' }}>
                    <i className={m === 'cash' ? 'ti ti-cash' : 'ti ti-currency-bitcoin'} aria-hidden="true" />
                    <span>{m === 'cash' ? t('checkout.pay_cash', 'Cash on delivery') : t('checkout.pay_crypto', 'Pay with crypto')}</span>
                  </button>
                );
              })}
            </div>
            {paymentMethod === 'crypto' && (
              <div className="mt-3 p-3 rounded-[var(--brand-radius-sm)] border" data-testid="crypto-disclosure" style={{ background: 'var(--color-danger-light, var(--brand-surface-raised))', borderColor: 'var(--color-danger, var(--brand-border))' }}>
                <p className="text-step-xs font-bold flex items-center gap-1.5" style={{ color: 'var(--color-danger, var(--brand-text))' }}>
                  <i className="ti ti-alert-triangle" aria-hidden="true" />{t('checkout.crypto_irreversible_title', 'Crypto payments are final')}
                </p>
                <p className="text-step-2xs mt-1 leading-relaxed" style={{ color: 'var(--brand-text-muted)' }}>
                  {t('checkout.crypto_irreversible_body', 'Once confirmed on the blockchain, a crypto payment cannot be reversed. If your order is cancelled or undelivered, the venue refunds you manually to your wallet within 3 business days. You’ll pay in USDT or USDC on a secure page.')}
                </p>
              </div>
            )}
          </div>
        )}
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
          what we collect and emphasises that the data is never sold or shared with
          third parties or advertisers (used only to fulfil the order). */}
      <div data-testid="checkout-privacy-notice" className="px-4 py-3 rounded-[var(--brand-radius)] border text-xs leading-relaxed" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}>
        <p className="font-semibold mb-1" style={{ color: 'var(--brand-text)' }}>
          <i className="ti ti-lock mr-1" />{t('checkout.privacy.title', 'Your data')}
        </p>
        <p>{t('checkout.privacy.what', 'To deliver your order we collect your name, phone, address and (if you add it) a door photo.')}</p>
        <p className="mt-1">{t('checkout.privacy.never_sold', 'We never sell or share your information with third parties or advertisers — it\'s used only to fulfil your order.')}</p>
      </div>

      <StickyActionBar>
        <motion.button
          type="submit"
          form="checkout-form"
          data-testid="order-confirm-button"
          disabled={placing || !locationId}
          whileTap={{ scale: (placing || !locationId) ? 1 : 0.97 }}
          className="w-full h-14 rounded-full bg-[var(--brand-primary-strong)] text-[var(--color-on-primary)] font-bold text-base flex items-center justify-center gap-2 transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.97] hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed disabled:translate-y-0"
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

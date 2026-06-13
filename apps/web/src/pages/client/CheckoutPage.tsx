import React, { useState, useEffect } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Button, MapWithPin, useI18n, StickyActionBar, PriceDisplay } from '@deliveryos/ui';
import type { LngLatLike } from '@deliveryos/ui';
import { PHONE_E164_REGEX, PHONE_E164_PATTERN } from '@deliveryos/shared-types';
import { apiClient } from '../../lib/index.js';
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
    const publicKeyRes: any = await apiClient('/api/push/vapid-public-key');
    const publicKey: string | undefined = publicKeyRes?.publicKey;
    if (!publicKey) return;
    const sub = await reg.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: urlBase64ToUint8Array(publicKey) as any,
    });
    const p256dhKey = sub.getKey('p256dh');
    const authKey = sub.getKey('auth');
    if (!p256dhKey || !authKey) return;
    await apiClient('/api/customer/push/subscribe', {
      method: 'POST',
      body: {
        endpoint: sub.endpoint,
        keys: { p256dh: btoa(String.fromCharCode(...new Uint8Array(p256dhKey))), auth: btoa(String.fromCharCode(...new Uint8Array(authKey))) },
        opted_in: true,
      },
    });
  } catch {
    // Push subscription is best-effort
  }
}

function urlBase64ToUint8Array(base64String: string): Uint8Array {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding).replace(/-/g, '+').replace(/_/g, '/');
  const rawData = atob(base64);
  return Uint8Array.from(rawData.split('').map((c) => c.charCodeAt(0)));
}

type DeliveryType = 'delivery' | 'pickup' | 'scheduled';

export function CheckoutPage() {
  const { slug } = useParams<{ slug: string }>();
  const navigate = useNavigate();
  const { items, clearCart } = useSharedCart();
  const { t } = useI18n();

  const [deliveryType, setDeliveryType] = useState<DeliveryType>('delivery');
  const [address, setAddress] = useState('');
  const [phone, setPhone] = useState('');
  const [customerName, setCustomerName] = useState('');
  const [pinLocation, setPinLocation] = useState<LngLatLike | null>(null);
  const [locationId, setLocationId] = useState<string | null>(null);
  const [currency, setCurrency] = useState('ALL');
  const [cashAmount, setCashAmount] = useState<number>(0);
  const [orderError, setOrderError] = useState('');
  const [instructionOption, setInstructionOption] = useState<string>('');
  const [instructionCustom, setInstructionCustom] = useState<string>('');
  const [entrance, setEntrance] = useState('');
  const [apartment, setApartment] = useState('');
  const [phoneError, setPhoneError] = useState('');
  const [entranceError, setEntranceError] = useState('');
  const [apartmentError, setApartmentError] = useState('');

    useEffect(() => {
    if (!slug) return;
    fetch(`/public/locations/${slug}/info`).then(r => r.json())
      .then((info: any) => {
        setLocationId(info.id);
        setCurrency(info.currency_code || 'ALL');
      })
      .catch(() => {
        console.debug('[Checkout] failed to load location info');
      });
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
    } catch { /* ignore corrupt localStorage */ }
  }, [slug]);

  const subtotal = items.reduce((sum, item) => sum + item.price * item.quantity, 0);
  const deliveryFee = deliveryType === 'delivery' ? 200 : 0;
  const total = subtotal + deliveryFee;

  const hasNutrition = items.some((item: any) => (item as any).kcal != null);
  const nutritionTotal = items.reduce((acc, item: any) => ({
    kcal: acc.kcal + ((item as any).kcal ?? 0) * item.quantity,
    protein: acc.protein + ((item as any).protein ?? 0) * item.quantity,
  }), { kcal: 0, protein: 0 });

  const orderItems = items.map(i => ({
    product_id: i.productId,
    quantity: i.quantity,
    modifier_ids: Object.values(i.options || {}).flat() as string[],
  }));

    const handlePlaceOrder = async (e: React.FormEvent) => {
    e.preventDefault();
    setOrderError('');
    if (items.length === 0 || !slug || !locationId) return;
    if (!phone || !PHONE_E164_REGEX.test(phone)) {
      setPhoneError(t('checkout.phone_invalid', 'Enter a valid phone number (+355...)'));
      return;
    }
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
    
    try {
      const idempotencyKey = crypto.randomUUID();
      const orderRes = await apiClient<any>('/orders', {
        method: 'POST',
        body: {
          locationId: locationId,
          type: 'delivery',
          items: orderItems,
          customer: {
            phone: phone,
            name: customerName || undefined,
          },
          delivery: {
            pin: {
              lat: (pinLocation as [number, number])?.[1] || 41.331,
              lng: (pinLocation as [number, number])?.[0] || 19.817,
            },
            address_text: address || undefined,
          },
          payment: { method: 'cash' },
          cash_pay_with: cashAmount > 0 ? cashAmount : undefined,
          idempotency_key: idempotencyKey,
          acknowledged_codes: [],
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
      try {
        localStorage.setItem(`dos_last_delivery_${slug}`, JSON.stringify({
          lat: (pinLocation as [number, number])?.[1] || 41.331,
          lng: (pinLocation as [number, number])?.[0] || 19.817,
          address,
          entrance,
          apartment,
        }));
      } catch { /* localStorage may be full or blocked */ }
      requestPushPermission(slug!);
      clearCart();
      navigate(`/s/${slug}/order/${orderRes.id}`);
    } catch (err: any) {
      if (isDevMode()) { clearCart(); navigate(`/s/${slug}/order/o_mock_123`); return; }
      if (err?.status === 422 && err?.body?.code === 'MIN_ORDER_NOT_MET') {
        setOrderError(t('checkout.min_order_error', 'Minimum order is {{min}} {{currency}}. Your total is {{subtotal}}.', {
          min: err.body.details?.min_order_value,
          currency: 'ALL',
          subtotal: err.body.details?.subtotal,
        }));
        return;
      }
      throw new Error('Failed to place order');
    }
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
        <button onClick={() => navigate(-1)} aria-label={t('common.back', 'Go back')} className="w-10 h-10 rounded-full flex items-center justify-center border transition-colors active:scale-95" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
          <i className="ti ti-arrow-left" aria-hidden="true" />
        </button>
        <h1 className="text-[24px] font-bold" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.title')}</h1>
      </div>

      <form id="checkout-form" onSubmit={handlePlaceOrder} className="space-y-6">
        <div className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
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
                <input required value={phone} onChange={e => { setPhone(e.target.value); setPhoneError(''); }} placeholder="+355 6X XXX XXXX" pattern={PHONE_E164_PATTERN} title="+355 followed by 7-14 digits" type="tel" inputMode="tel" autoComplete="tel" className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: phoneError ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }} />
                {phoneError && <p role="alert" className="text-[12px] mt-1" style={{ color: 'var(--color-danger)' }}>{phoneError}</p>}
              </div>
            </div>
          </div>
        </div>
        <div className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <h2 className="text-[20px] font-semibold mb-6" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.delivery_address')}</h2>
          <div className="flex p-1 rounded-[10px] mb-6 gap-0.5" role="tablist" aria-label={t('checkout.delivery_type', 'Delivery type')} style={{ background: 'var(--brand-surface)' }}>
            <button type="button" role="tab" aria-selected={deliveryType === 'delivery'} onClick={() => setDeliveryType('delivery')} className="flex-1 py-2 text-[13px] rounded-[8px] transition-all" style={btnStyle('delivery')}>{t('courier.deliver')}</button>
            <button type="button" role="tab" aria-selected={deliveryType === 'pickup'} onClick={() => setDeliveryType('pickup')} className="flex-1 py-2 text-[13px] rounded-[8px] transition-all" style={btnStyle('pickup')}>{t('courier.pickup')}</button>
            <button type="button" role="tab" aria-selected={deliveryType === 'scheduled'} onClick={() => setDeliveryType('scheduled')} className="flex-1 py-2 text-[13px] rounded-[8px] transition-all" style={btnStyle('scheduled')}>{t('order.scheduled')}</button>
          </div>

          {deliveryType === 'delivery' && (
            <div className="space-y-4">
              <div>
                <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.delivery_address')}</label>
                <MapWithPin className="h-48 w-full rounded-lg" initialCenter={[19.817, 41.331]} onPinChange={setPinLocation} confirmLabel={t('common.confirm')} placeholder={t('checkout.delivery_address')} />
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
                    <input required value={entrance} onChange={e => setEntrance(e.target.value)} placeholder={t('checkout.entrance_placeholder', 'Entrance number or name')} className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  </div>
                  {entranceError && <p role="alert" className="text-[12px] mt-1" style={{ color: 'var(--color-danger)' }}>{entranceError}</p>}
                </div>
                <div>
                  <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.apartment')}</label>
                  <div className="relative">
                    <i className="ti ti-apartment absolute left-3 top-1/2 -translate-y-1/2 text-lg" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
                    <input required value={apartment} onChange={e => setApartment(e.target.value)} placeholder={t('checkout.apartment_placeholder', 'Apartment or unit number')} className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                  </div>
                  {apartmentError && <p role="alert" className="text-[12px] mt-1" style={{ color: 'var(--color-danger)' }}>{apartmentError}</p>}
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
                    <button
                      key={opt.key}
                      type="button"
                      aria-pressed={instructionOption === opt.val}
                      onClick={() => setInstructionOption(instructionOption === opt.val ? '' : opt.val)}
                      className="px-3 py-1.5 text-[12px] rounded-[20px] border transition-all active:scale-95"
                      style={{
                        background: instructionOption === opt.val ? 'var(--brand-primary)' : 'var(--brand-surface-raised)',
                        borderColor: instructionOption === opt.val ? 'var(--brand-primary)' : 'var(--brand-border)',
                        color: instructionOption === opt.val ? '#fff' : 'var(--brand-text)',
                      }}
                    >{t(opt.key, opt.val)}</button>
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
        </div>

        <div className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
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
                  <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[14px] font-bold" style={{ color: 'var(--brand-text-muted)' }}>{currency}</span>
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
        </div>

        <div className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
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
            {hasNutrition && (
              <div className="flex justify-between text-[12px]">
                <span style={{ color: 'var(--brand-text-muted)' }}>≈ {t('menu.nutrition')}</span>
                <span className="font-medium" style={{ color: 'var(--brand-text-muted)' }}>~{nutritionTotal.kcal} kcal</span>
              </div>
            )}
          </div>
          <div className="pt-4 border-t flex justify-between items-center" style={{ borderColor: 'var(--brand-border)' }}>
            <span className="text-[16px] font-bold" style={{ color: 'var(--brand-text)' }}>{t('cart.total')}</span>
                  <PriceDisplay amount={total} size="lg" />
          </div>
        </div>
      </form>

      <StickyActionBar>
        <button
          type="submit"
          form="checkout-form"
          className="w-full h-14 rounded-full bg-[var(--brand-primary)] text-white font-bold text-base shadow-xl transition-all active:scale-[0.97] flex items-center justify-center gap-2"
          style={{ minHeight: 'var(--tap-critical)' }}
        >
                  {t('checkout.place_order')} &bull; <PriceDisplay amount={total} size="sm" />
        </button>
      </StickyActionBar>

    </div>
  );
}

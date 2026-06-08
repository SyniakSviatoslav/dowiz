import React, { useState, useEffect, useRef } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { Button, OTPModal, MapWithPin, useI18n } from '@deliveryos/ui';
import type { LngLatLike } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { useSharedCart } from '../../lib/CartProvider.js';

const isDevMode = () => typeof window !== 'undefined' && sessionStorage.getItem('dos_dev') === '1';

type DeliveryType = 'delivery' | 'pickup' | 'scheduled';

async function sha256Hex(input: string): Promise<string> {
  const enc = new TextEncoder();
  const buf = await crypto.subtle.digest('SHA-256', enc.encode(input));
  return Array.from(new Uint8Array(buf)).map(b => b.toString(16).padStart(2, '0')).join('');
}

function hashOrderIntent(items: Array<{ product_id: string; quantity: number }>): Promise<string> {
  const canonical = items
    .map(i => `${i.product_id}:${i.quantity}`)
    .sort()
    .join(',');
  return sha256Hex(canonical);
}

export function CheckoutPage() {
  const { slug } = useParams<{ slug: string }>();
  const navigate = useNavigate();
  const { items, clearCart } = useSharedCart();
  const { t } = useI18n();

  const [deliveryType, setDeliveryType] = useState<DeliveryType>('delivery');
  const [address, setAddress] = useState('');
  const [instructions, setInstructions] = useState('');
  const [phone, setPhone] = useState('');
  const [customerName, setCustomerName] = useState('');
  const [isOTPOpen, setOTPOpen] = useState(false);
  const [otpError, setOtpError] = useState('');
  const [pinLocation, setPinLocation] = useState<LngLatLike | null>(null);
  const [locationId, setLocationId] = useState<string | null>(null);
  const [currency, setCurrency] = useState('ALL');

  const otpTokenRef = useRef<string>('');
  const verifiedTokenRef = useRef<string>('');

  useEffect(() => {
    if (!slug) return;
    apiClient<any>(`/public/locations/${slug}/info`)
      .then((info: any) => {
        setLocationId(info.id);
        setCurrency(info.currency_code || 'ALL');
      })
      .catch(() => {
        console.debug('[Checkout] failed to load location info');
      });
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

  const handleStartCheckout = (e: React.FormEvent) => {
    e.preventDefault();
    if (items.length === 0 || !slug) return;
    if (!phone) {
      setOtpError(t('checkout.phone_required', 'Phone number required'));
      return;
    }
    setOTPOpen(true);
  };

  const handleSendOTP = async (verifyPhone: string): Promise<void> => {
    setPhone(verifyPhone);
    setOtpError('');
    if (!slug) return;
    try {
      const res = await apiClient<any>(`/customer/locations/${slug}/otp/send`, {
        method: 'POST',
        body: {
          phone: verifyPhone,
          order_intent: {
            items: items.map(i => ({ product_id: i.productId, quantity: i.quantity })),
            total: total,
            currency: currency,
          },
        },
      });
      otpTokenRef.current = res.otp_token;
    } catch {
      console.debug('[Checkout] OTP send failed');
    }
  };

  const handleVerifyOTP = async (code: string) => {
    if (!slug || !locationId) throw new Error('Location not loaded');
    const otpToken = otpTokenRef.current;
    if (!otpToken) throw new Error('No OTP session');
    const intentHash = await hashOrderIntent(
      items.map(i => ({ product_id: i.productId, quantity: i.quantity }))
    );
    try {
      const verifyRes = await apiClient<any>(`/customer/locations/${slug}/otp/verify`, {
        method: 'POST',
        body: {
          phone,
          code,
          otp_token: otpToken,
          order_intent_hash: intentHash,
        },
      });
      verifiedTokenRef.current = verifyRes.verified_token;
    } catch (e) {
      if (!isDevMode()) throw new Error('Invalid code');
    }
    try {
      const idempotencyKey = crypto.randomUUID();
      const orderRes = await apiClient<any>('/orders', {
        method: 'POST',
        headers: verifiedTokenRef.current ? { 'x-otp-verified': verifiedTokenRef.current } : {},
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
          cash_pay_with: false,
          idempotency_key: idempotencyKey,
          acknowledged_codes: [],
        },
      });
      clearCart();
      navigate(`/s/${slug}/order/${orderRes.id}`);
    } catch {
      if (isDevMode()) { clearCart(); navigate(`/s/${slug}/order/o_mock_123`); return; }
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
    <div className="max-w-xl mx-auto p-4 md:py-8 space-y-6">
      <div className="flex items-center gap-3 mb-6">
        <button onClick={() => navigate(-1)} className="w-10 h-10 rounded-full flex items-center justify-center border transition-colors active:scale-95" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
          <i className="ti ti-arrow-left" />
        </button>
        <h1 className="text-[24px] font-bold" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.title')}</h1>
      </div>

      <form onSubmit={handleStartCheckout} className="space-y-6">
        <div className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <h2 className="text-[20px] font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.contact_info', 'Contact Info')}</h2>
          <div className="space-y-3">
            <div>
              <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.name', 'Name')}</label>
              <div className="relative">
                <i className="ti ti-user absolute left-3 top-1/2 -translate-y-1/2 text-lg" style={{ color: 'var(--brand-text-muted)' }} />
                <input required value={customerName} onChange={e => setCustomerName(e.target.value)} placeholder={t('checkout.name_placeholder', 'Your name')} className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
              </div>
            </div>
            <div>
              <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.phone', 'Phone')}</label>
              <div className="relative">
                <i className="ti ti-phone absolute left-3 top-1/2 -translate-y-1/2 text-lg" style={{ color: 'var(--brand-text-muted)' }} />
                <input required value={phone} onChange={e => setPhone(e.target.value)} placeholder="+355 6X XXX XXXX" className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
              </div>
            </div>
          </div>
        </div>
        <div className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <h2 className="text-[20px] font-semibold mb-6" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.delivery_address')}</h2>
          <div className="flex p-1 rounded-[10px] mb-6 gap-0.5" style={{ background: 'var(--brand-surface)' }}>
            <button type="button" onClick={() => setDeliveryType('delivery')} className="flex-1 py-2 text-[13px] rounded-[8px] transition-all" style={btnStyle('delivery')}>{t('courier.deliver')}</button>
            <button type="button" onClick={() => setDeliveryType('pickup')} className="flex-1 py-2 text-[13px] rounded-[8px] transition-all" style={btnStyle('pickup')}>{t('courier.pickup')}</button>
            <button type="button" onClick={() => setDeliveryType('scheduled')} className="flex-1 py-2 text-[13px] rounded-[8px] transition-all" style={btnStyle('scheduled')}>{t('order.scheduled')}</button>
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
                  <i className="ti ti-map-pin absolute left-3 top-1/2 -translate-y-1/2 text-lg" style={{ color: 'var(--brand-text-muted)' }} />
                  <input required value={address} onChange={e => setAddress(e.target.value)} placeholder={t('checkout.delivery_address')} className="w-full h-[48px] pl-10 pr-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
                </div>
              </div>
              <div>
                <label className="text-[13px] font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.notes')}</label>
                <input value={instructions} onChange={e => setInstructions(e.target.value)} placeholder={t('checkout.notes')} className="w-full h-[48px] px-3 outline-none text-[14px] border rounded-[8px] transition-colors" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
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
                  <i className="ti ti-building-store text-3xl relative z-10" style={{ color: 'var(--brand-text-muted)' }} />
                </div>
              </div>
              <div className="flex items-center gap-3 p-3 rounded-[8px] border" style={{ background: 'var(--color-info-light)', borderColor: 'var(--color-info)', color: 'var(--color-info)' }}>
                <i className="ti ti-info-circle" />
                <p className="text-[13px] font-medium">{t('checkout.phone_hint')}</p>
              </div>
            </div>
          )}
        </div>

        <div className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <h2 className="text-[20px] font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.payment_method')}</h2>
          <div className="border rounded-[8px] p-3 flex items-center justify-between mb-4" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-primary)' }}>
            <div className="flex items-center gap-3">
              <i className="ti ti-cash text-xl" style={{ color: 'var(--brand-primary)' }} />
              <div>
                <div className="text-[14px] font-bold" style={{ color: 'var(--brand-text)' }}>{t('checkout.cash')}</div>
                <div className="text-[12px]" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.place_order')}</div>
              </div>
            </div>
            <i className="ti ti-check" style={{ color: 'var(--brand-primary)' }} />
          </div>
        </div>

        <div className="rounded-[12px] p-4 border shadow-sm" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }}>
          <h2 className="text-[20px] font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('order.title')}</h2>
          <div className="space-y-3 mb-4">
            <div className="flex justify-between text-[14px]">
              <span style={{ color: 'var(--brand-text-muted)' }}>{t('cart.subtotal')}</span>
              <span className="font-medium" style={{ color: 'var(--brand-text)' }}>{subtotal} ALL</span>
            </div>
            {deliveryType === 'delivery' && (
              <div className="flex justify-between text-[14px]">
                <span style={{ color: 'var(--brand-text-muted)' }}>{t('cart.delivery_fee')}</span>
                <span className="font-medium" style={{ color: 'var(--brand-text)' }}>{deliveryFee} ALL</span>
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
            <span className="text-[20px] font-black" style={{ color: 'var(--brand-primary)' }}>{total} ALL</span>
          </div>
        </div>

        <Button type="submit" size="lg" className="w-full font-bold text-[16px] h-[56px] shadow-xl">
          {t('checkout.place_order')} &bull; {total} ALL
        </Button>
      </form>

      <OTPModal isOpen={isOTPOpen} onClose={() => setOTPOpen(false)} phone={phone} onSendOTP={handleSendOTP} onVerifyOTP={handleVerifyOTP} />
    </div>
  );
}

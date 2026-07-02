import React from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { useI18n, ease, MapWithPin, Textarea } from '@deliveryos/ui';
import type { LngLatLike } from '@deliveryos/ui';
import type { DeliveryType } from './types.js';

interface DeliveryDetailsSectionProps {
  deliveryType: DeliveryType;
  locationCenter: LngLatLike;
  pinLocation: LngLatLike | null;
  setPinLocation: React.Dispatch<React.SetStateAction<LngLatLike | null>>;
  address: string;
  setAddress: React.Dispatch<React.SetStateAction<string>>;
  entrance: string;
  setEntrance: React.Dispatch<React.SetStateAction<string>>;
  entranceError: string;
  apartment: string;
  setApartment: React.Dispatch<React.SetStateAction<string>>;
  apartmentError: string;
  notes: string;
  setNotes: React.Dispatch<React.SetStateAction<string>>;
  instructionOption: string;
  setInstructionOption: React.Dispatch<React.SetStateAction<string>>;
  instructionCustom: string;
  setInstructionCustom: React.Dispatch<React.SetStateAction<string>>;
  pickupName: string;
  pickupAddress: string;
}

export function DeliveryDetailsSection({
  deliveryType,
  locationCenter, pinLocation, setPinLocation,
  address, setAddress,
  entrance, setEntrance, entranceError,
  apartment, setApartment, apartmentError,
  notes, setNotes,
  instructionOption, setInstructionOption,
  instructionCustom, setInstructionCustom,
  pickupName, pickupAddress,
}: DeliveryDetailsSectionProps) {
  const { t } = useI18n();
  const prefersReducedMotion = useReducedMotion();

  return (
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
            <label className="text-step-sm font-bold mb-1.5 block" style={{ color: 'var(--brand-text)' }}>{t('checkout.pin_on_map', 'Tap the map to set your delivery location')}</label>
            <MapWithPin className="h-48 w-full rounded-[var(--brand-radius-sm)]" initialCenter={locationCenter} onPinChange={setPinLocation} confirmLabel={t('common.confirm')} placeholder={t('checkout.pin_on_map', 'Tap the map to set your delivery location')} myLocationLabel={t('checkout.my_location', 'My location')} />
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
                <input value={instructionCustom} onChange={e => setInstructionCustom(e.target.value)} placeholder={t('checkout.extra_notes', 'Extra notes...')} className="w-full h-[48px] pl-10 pr-3 outline-none text-step-sm border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
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
  );
}

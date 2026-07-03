import React, { useState, useEffect, useCallback, useRef } from 'react';
import { m, useReducedMotion } from 'framer-motion';
import { useI18n } from '../../lib/I18nProvider.js';
import { ease, duration } from '../../lib/motion.js';
import type { MessageSender, OrderStatusForMsg } from '@deliveryos/shared-types';

interface MessageData {
  id: string;
  order_id: string;
  sender: MessageSender;
  preset_key: string;
  params: Record<string, unknown>;
  body: null;
  read_at: string | null;
  created_at: string;
}

interface PresetOption {
  key: string;
  label: string;
  params?: Record<string, unknown>;
  paramType?: 'minutes' | 'location' | 'action' | 'amount';
  paramOptions?: readonly (string | number)[];
}

interface MessageThreadProps {
  orderId: string;
  role: MessageSender;
  currentStatus: string;
  messages: MessageData[];
  onSend: (presetKey: string, params?: Record<string, unknown>) => void;
  onMarkRead: () => void;
}

const PRESET_BY_ROLE_STATUS: Record<string, PresetOption[]> = {
  courier_IN_DELIVERY: [
    { key: 'cu_on_my_way', label: 'message.preset.cu_on_my_way' },
    { key: 'cu_eta', label: 'message.preset.cu_eta', paramType: 'minutes', paramOptions: [5, 10, 15] as const },
    { key: 'cu_arrived', label: 'message.preset.cu_arrived' },
    { key: 'cu_at_entrance', label: 'message.preset.cu_at_entrance' },
    { key: 'cu_cant_find', label: 'message.preset.cu_cant_find' },
    { key: 'cu_waiting', label: 'message.preset.cu_waiting' },
    { key: 'cu_left_at_door', label: 'message.preset.cu_left_at_door' },
    { key: 'cu_prepare_cash', label: 'message.preset.cu_prepare_cash', paramType: 'amount' },
    { key: 'cu_please_call_me', label: 'message.preset.cu_please_call_me' },
  ],
  customer_IN_DELIVERY: [
    { key: 'cc_coming_out', label: 'message.preset.cc_coming_out' },
    { key: 'cc_wait', label: 'message.preset.cc_wait', paramType: 'minutes', paramOptions: [2, 5] as const },
    { key: 'cc_leave_at_door', label: 'message.preset.cc_leave_at_door' },
    { key: 'cc_im_at', label: 'message.preset.cc_im_at', paramType: 'location', paramOptions: ['entrance', 'gate', 'reception'] as const },
    { key: 'cc_meet_outside', label: 'message.preset.cc_meet_outside' },
  ],
  customer_PENDING: [
    { key: 'co_cancel_request', label: 'message.preset.co_cancel_request' },
  ],
  customer_CONFIRMED: [
    { key: 'co_cancel_request', label: 'message.preset.co_cancel_request' },
    { key: 'co_when_ready', label: 'message.preset.co_when_ready' },
  ],
  customer_PREPARING: [
    { key: 'co_when_ready', label: 'message.preset.co_when_ready' },
  ],
  owner_PENDING: [
    { key: 'ow_accepted_preparing', label: 'message.preset.ow_accepted_preparing' },
    { key: 'ow_substitution', label: 'message.preset.ow_substitution', paramType: 'action', paramOptions: ['replace_similar', 'remove_refund', 'cancel'] as const },
    { key: 'ow_high_load', label: 'message.preset.ow_high_load' },
  ],
  owner_PREPARING: [
    { key: 'ow_accepted_preparing', label: 'message.preset.ow_accepted_preparing' },
    { key: 'ow_delay', label: 'message.preset.ow_delay', paramType: 'minutes', paramOptions: [15, 30] as const },
    { key: 'ow_substitution', label: 'message.preset.ow_substitution', paramType: 'action', paramOptions: ['replace_similar', 'remove_refund', 'cancel'] as const },
    { key: 'ow_high_load', label: 'message.preset.ow_high_load' },
  ],
  owner_CONFIRMED: [
    { key: 'ow_delay', label: 'message.preset.ow_delay', paramType: 'minutes', paramOptions: [15, 30] as const },
    { key: 'ow_high_load', label: 'message.preset.ow_high_load' },
  ],
};

function getPresets(role: string, status: string): PresetOption[] {
  return PRESET_BY_ROLE_STATUS[`${role}_${status}`] || PRESET_BY_ROLE_STATUS[`${role}_${status}`] || [];
}

const TERMINAL: ReadonlySet<string> = new Set(['DELIVERED', 'REJECTED', 'CANCELLED']);

export function MessageThread({ orderId, role, currentStatus, messages, onSend, onMarkRead }: MessageThreadProps) {
  const { t } = useI18n();
  const reduceMotion = useReducedMotion();
  const [selectedPreset, setSelectedPreset] = useState<PresetOption | null>(null);
  const [paramValue, setParamValue] = useState<string | number>('');
  const threadEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    onMarkRead();
  }, []);

  useEffect(() => {
    threadEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = useCallback(() => {
    if (!selectedPreset) return;

    let params: Record<string, unknown> = {};
    if (selectedPreset.paramType === 'minutes' && paramValue) {
      params = { minutes: Number(paramValue) };
    } else if (selectedPreset.paramType === 'location' && paramValue) {
      params = { location: String(paramValue) };
    } else if (selectedPreset.paramType === 'action' && paramValue) {
      params = { action: String(paramValue) };
    } else if (selectedPreset.paramType === 'amount' && paramValue) {
      params = { amount: Number(paramValue) };
    }

    onSend(selectedPreset.key, params);
    setSelectedPreset(null);
    setParamValue('');
  }, [selectedPreset, paramValue, onSend]);

  const isTerminal = TERMINAL.has(currentStatus);
  const myRole = role;
  const availablePresets = getPresets(role, currentStatus);

  return (
    <div className="border-t border-[var(--brand-border)] mt-4">
      <div className="px-4 py-3 font-semibold text-[var(--brand-text)] text-sm">
        {t('message.title', 'Messages')}
      </div>

      {/* Message list */}
      <div className="px-4 space-y-2 max-h-64 overflow-y-auto">
        {messages.length === 0 && (
          <div className="flex flex-col items-center text-center gap-1 py-8">
            <svg width="28" height="28" viewBox="0 0 24 24" fill="none" aria-hidden="true" className="opacity-60" style={{ color: 'var(--brand-text-muted)' }}>
              <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            <p className="text-sm font-medium" style={{ color: 'var(--brand-text)' }}>{t('message.empty', 'No messages yet')}</p>
            <p className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('message.empty_hint', 'Tap a quick reply below to start the conversation.')}</p>
          </div>
        )}
        {messages.map((msg) => {
          const isMine = msg.sender === myRole;
          return (
            <m.div
              key={msg.id}
              className={`flex ${isMine ? 'justify-end' : 'justify-start'}`}
              initial={reduceMotion ? false : { opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: duration.base, ease: ease.out }}
            >
              <div
                className={`max-w-[78%] min-w-0 px-3 py-2 text-sm break-words [overflow-wrap:anywhere] rounded-[var(--brand-radius)] ${
                  isMine
                    ? 'bg-[var(--brand-primary)] text-[var(--brand-bg)] rounded-br-[var(--brand-radius-sm)]'
                    : 'bg-[var(--brand-surface)] text-[var(--brand-text)] border border-[var(--brand-border)] rounded-bl-[var(--brand-radius-sm)]'
                }`}
                style={{ boxShadow: 'var(--elev-1)' }}
              >
                {/* BUGFIX: the stored preset_key is the bare code ('cu_on_my_way'); the catalog keys
                    them under 'message.preset.<key>' (the chips use that). Without the namespace t()
                    fell back to the raw code in the bubble. Prepend it; keep the raw code as fallback. */}
                <p>{t(('message.preset.' + msg.preset_key) as any, msg.preset_key, msg.params as any)}</p>
                <p className={`text-step-2xs mt-1 tabular-nums ${isMine ? 'text-white/80' : 'text-[var(--brand-text-muted)]'}`}>
                  {new Date(msg.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                </p>
              </div>
            </m.div>
          );
        })}
        <div ref={threadEndRef} />
      </div>

      {/* Input area */}
      {!isTerminal && availablePresets.length > 0 && (
        <div className="px-4 py-3 border-t border-[var(--brand-border)]">
          {!selectedPreset ? (
            <div className="flex flex-wrap gap-2">
              {availablePresets.map((preset) => (
                <button
                  key={preset.key}
                  type="button"
                  onClick={() => {
                    // A chip WITHOUT a param sends exactly once and opens no confirm bar.
                    // Only a chip WITH a paramType opens the param-picker/confirm step.
                    if (preset.paramType) {
                      setSelectedPreset(preset);
                    } else {
                      onSend(preset.key, {});
                      setSelectedPreset(null);
                      setParamValue('');
                    }
                  }}
                  className="px-3 py-1.5 min-h-[36px] text-xs rounded-full border border-[var(--brand-border)] bg-[var(--brand-surface)] text-[var(--brand-text)] transition-[background-color,color,border-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:bg-[var(--brand-primary)] hover:text-white hover:border-[var(--brand-primary)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                >
                  {/* Chip is a category trigger shown BEFORE the param is picked, so the raw
                      label still carries `{{minutes}}`/`{{amount}}` placeholders — strip them to
                      an ellipsis (the value is chosen on the next step) instead of leaking mustache. */}
                  {t(preset.label as any, preset.key).replace(/\{\{\w+\}\}/g, '…')}
                </button>
              ))}
            </div>
          ) : (
            <div className="space-y-2">
              {selectedPreset.paramType === 'amount' && (
                <input
                  type="number"
                  inputMode="numeric"
                  min={0}
                  step={1}
                  value={paramValue}
                  onChange={(e) => setParamValue(e.target.value)}
                  placeholder={t('message.amount_placeholder', 'Amount')}
                  className="w-full px-3 py-1.5 min-h-[36px] text-sm rounded-[var(--brand-radius)] border border-[var(--brand-border)] bg-[var(--brand-surface)] text-[var(--brand-text)] transition-[border-color] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                />
              )}
              {selectedPreset.paramType && selectedPreset.paramType !== 'amount' && (
                <div className="flex flex-wrap gap-2">
                  {selectedPreset.paramOptions?.map((opt) => (
                    <button
                      key={String(opt)}
                      type="button"
                      aria-pressed={paramValue === opt}
                      onClick={() => setParamValue(opt)}
                      className={`px-3 py-1.5 min-h-[36px] text-xs rounded-full border transition-[background-color,color,border-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 ${
                        paramValue === opt
                          ? 'bg-[var(--brand-primary)] text-[var(--brand-bg)] border-[var(--brand-primary)]'
                          : 'border-[var(--brand-border)] text-[var(--brand-text)] hover:border-[var(--brand-primary)]'
                      }`}
                    >
                      {selectedPreset.paramType === 'location'
                        ? t(`message.location.${opt}` as any, String(opt))
                        : selectedPreset.paramType === 'action'
                          ? t(`message.action.${opt}` as any, String(opt))
                          : String(opt) + (selectedPreset.paramType === 'minutes' ? ` ${t('common.min', 'min')}` : '')}
                    </button>
                  ))}
                </div>
              )}
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={handleSend}
                  disabled={
                    selectedPreset.paramType === 'amount'
                      ? !(Number(paramValue) > 0)
                      : selectedPreset.paramType
                        ? !paramValue
                        : false
                  }
                  className="flex-1 min-h-[44px] px-3 py-2 text-sm rounded-[var(--brand-radius)] bg-[var(--brand-primary)] text-[var(--brand-bg)] font-medium transition-[opacity,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 disabled:opacity-50 disabled:active:scale-100"
                >
                  {t('message.send', 'Send')}
                </button>
                <button
                  type="button"
                  onClick={() => { setSelectedPreset(null); setParamValue(''); }}
                  className="min-h-[44px] px-3 py-2 text-sm rounded-[var(--brand-radius)] border border-[var(--brand-border)] text-[var(--brand-text)] transition-[background-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:bg-[var(--brand-surface)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                >
                  {t('common.cancel', 'Cancel')}
                </button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

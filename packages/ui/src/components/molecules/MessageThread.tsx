import React, { useState, useEffect, useCallback, useRef } from 'react';
import { useI18n } from '../../lib/I18nProvider.js';
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
    } else if (selectedPreset.paramType === 'amount') {
      params = {};
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
          <p className="text-[var(--brand-text-muted)] text-xs py-2">{t('message.empty', 'No messages')}</p>
        )}
        {messages.map((msg) => {
          const isMine = msg.sender === myRole;
          return (
            <div key={msg.id} className={`flex ${isMine ? 'justify-end' : 'justify-start'}`}>
              <div
                className={`max-w-[75%] px-3 py-2 rounded-lg text-sm ${
                  isMine
                    ? 'bg-[var(--brand-primary)] text-[var(--brand-bg)] rounded-br-sm'
                    : 'bg-[var(--brand-surface)] text-[var(--brand-text)] border border-[var(--brand-border)] rounded-bl-sm'
                }`}
              >
                <p>{t(msg.preset_key as any, msg.preset_key, msg.params as any)}</p>
                <p className={`text-[10px] mt-1 ${isMine ? 'text-white/70' : 'text-[var(--brand-text-muted)]'}`}>
                  {new Date(msg.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                </p>
              </div>
            </div>
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
                  onClick={() => {
                    setSelectedPreset(preset);
                    if (!preset.paramType) {
                      onSend(preset.key, {});
                    }
                  }}
                  className="px-3 py-1.5 text-xs rounded-full border border-[var(--brand-border)] bg-[var(--brand-surface)] text-[var(--brand-text)] hover:bg-[var(--brand-primary)] hover:text-white hover:border-[var(--brand-primary)] transition-colors"
                >
                  {t(preset.label as any, preset.key)}
                </button>
              ))}
            </div>
          ) : (
            <div className="space-y-2">
              {selectedPreset.paramType && (
                <div className="flex flex-wrap gap-2">
                  {selectedPreset.paramOptions?.map((opt) => (
                    <button
                      key={String(opt)}
                      onClick={() => setParamValue(opt)}
                      className={`px-3 py-1.5 text-xs rounded-full border transition-colors ${
                        paramValue === opt
                          ? 'bg-[var(--brand-primary)] text-[var(--brand-bg)] border-[var(--brand-primary)]'
                          : 'border-[var(--brand-border)] text-[var(--brand-text-muted)] hover:border-[var(--brand-primary)]'
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
                  onClick={handleSend}
                  disabled={selectedPreset.paramType ? !paramValue : false}
                  className="flex-1 px-3 py-2 text-sm rounded-lg bg-[var(--brand-primary)] text-[var(--brand-bg)] font-medium disabled:opacity-50"
                >
                  {t('message.send', 'Send')}
                </button>
                <button
                  onClick={() => { setSelectedPreset(null); setParamValue(''); }}
                  className="px-3 py-2 text-sm rounded-lg border border-[var(--brand-border)] text-[var(--brand-text-muted)]"
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

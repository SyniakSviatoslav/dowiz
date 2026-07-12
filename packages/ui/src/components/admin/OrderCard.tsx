/* eslint-disable jsx-a11y/click-events-have-key-events, jsx-a11y/no-static-element-interactions, jsx-a11y/aria-role --
   pre-existing: the card/actions use onClick (incl. stopPropagation) without
   key handlers, and MessageThread takes a `role` *prop* (owner/courier) that the
   linter misreads as a DOM aria-role. Suppressed to keep the diff focused;
   keyboard-a11y for the card is tracked separately. */
import React, { memo, useState } from 'react';
import { Button, useI18n, MessageThread, PriceDisplay } from '../../index.js';
import { type AdminOrder, isOrderDetailsPending } from './types.js';

interface OrderCardMessage {
  name?: string;
  qty?: number;
  quantity?: number;
  price?: number;
}
interface AdminOrderExtras extends AdminOrder {
  rating?: number;
  feedback?: string;
}

interface OrderCardProps {
  order: AdminOrder;
  onUpdateStatus: (id: string, newStatus: string) => Promise<void>;
  isLoading?: boolean;
  showMessages?: boolean;
  onToggleMessages?: (orderId: string) => void;
  messages?: OrderCardMessage[];
  onSendMessage?: (orderId: string, presetKey: string, params?: Record<string, unknown>) => Promise<void>;
  onViewDetail?: (orderId: string) => void;
}
function maskPhone(phone?: string): string {
  if (!phone || phone.length < 4) return phone || '';
  return phone.slice(0, phone.length - 4).replace(/\d/g, '*') + phone.slice(-4);
}

export const OrderCard = memo(function OrderCard({ order, onUpdateStatus, isLoading, showMessages, onToggleMessages, messages, onSendMessage, onViewDetail }: OrderCardProps) {
  const { t } = useI18n();
  const [loadingAction, setLoadingAction] = useState('');

  const handleAction = async (status: string) => {
    setLoadingAction(status);
    await onUpdateStatus(order.id, status);
    setLoadingAction('');
  };

  const getStatusColor = (s: string) => {
    switch (s) {
      case 'PENDING': return `bg-[var(--status-pending-bg)] text-[var(--status-pending)] border-status-pending/30`;
      case 'CONFIRMED': return `bg-[var(--status-confirmed-bg)] text-[var(--status-confirmed)] border-status-confirmed/30`;
      case 'PREPARING': return `bg-[var(--status-preparing-bg)] text-[var(--status-preparing)] border-status-preparing/30`;
      case 'READY': return `bg-[var(--status-ready-bg)] text-[var(--status-ready)] border-status-ready/30`;
      case 'IN_DELIVERY': return `bg-[var(--status-in-delivery-bg)] text-[var(--status-in-delivery)] border-status-in-delivery/30`;
      case 'DELIVERED': return `bg-[var(--status-delivered-bg)] text-[var(--status-delivered)] border-status-delivered/30`;
      case 'REJECTED': return `bg-[var(--status-rejected-bg)] text-[var(--status-rejected)] border-status-rejected/30`;
      case 'CANCELLED': return `bg-[var(--status-cancelled-bg)] text-[var(--status-cancelled)] border-status-cancelled/30`;
      default: return `bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] border-brand-border`;
    }
  };

  const getStatusIcon = (s: string) => {
    switch (s) {
      case 'PENDING': return 'ti ti-clock';
      case 'CONFIRMED': return 'ti ti-circle-check';
      case 'PREPARING': return 'ti ti-chef-hat';
      case 'READY': return 'ti ti-check';
      case 'IN_DELIVERY': return 'ti ti-truck-delivery';
      case 'DELIVERED': return 'ti ti-package';
      case 'REJECTED': return 'ti ti-ban';
      case 'CANCELLED': return 'ti ti-x';
      default: return 'ti ti-help';
    }
  };

  // Localized status label — the raw uppercase enum ("REJECTED") leaked into the
  // otherwise-Albanian owner UI. Reuses the canonical order.* label catalog.
  const STATUS_LABEL_KEYS: Record<string, string> = {
    PENDING: 'order.pending', CONFIRMED: 'order.confirmed', PREPARING: 'order.preparing',
    READY: 'order.ready', IN_DELIVERY: 'order.in_delivery', DELIVERED: 'order.delivered',
    REJECTED: 'order.rejected', CANCELLED: 'order.cancelled',
  };
  const statusLabel = (s: string) => t(STATUS_LABEL_KEYS[s] || '', s.replace(/_/g, ' '));

  const getDeltaMin = (start?: string, end?: string) => {
    if (!start || !end) return null;
    const s = new Date(start).getTime();
    const e = new Date(end).getTime();
    if (isNaN(s) || isNaN(e)) return null;
    return Math.floor((e - s) / 60000);
  };

  const confirmDelta = getDeltaMin(order.createdAt, order.confirmedAt);
  const prepDelta = getDeltaMin(order.confirmedAt, order.readyAt);
  const deliveryDelta = getDeltaMin(order.readyAt, order.deliveredAt);

  // F7: hollow-card guard — see isOrderDetailsPending. Placeholder while the authed
  // backfill (name/items) is still in flight, instead of a nameless / "0 items" card.
  const detailsPending = isOrderDetailsPending(order);

  return (
    <div className={`bg-[var(--brand-surface)] border border-brand-border rounded-lg p-4 flex flex-col gap-4 ${isLoading ? 'opacity-50 pointer-events-none' : `cursor-pointer hover:bg-brand-surface-raised`}`} onClick={() => onViewDetail?.(order.id)}>

      {/* Header */}
      <div className="flex justify-between items-start">
        <div>
          <div className="font-bold text-lg text-brand-text">{order.shortId || '#' + order.id.substring(0, 4).toUpperCase()}</div>
          <div data-dynamic className="text-brand-text-muted text-sm flex items-center gap-2">
            {new Date(order.createdAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
          </div>
        </div>
        <div role="status" className={`px-2.5 py-1 rounded-full text-xs font-bold border flex items-center gap-1.5 transition-colors duration-200 ${getStatusColor(order.status)}`}>
          <i className={getStatusIcon(order.status)} style={{ fontSize: '0.75rem' }} />
          {statusLabel(order.status)}
        </div>
      </div>

      {/* Timeline Deltas */}
      {(confirmDelta != null || prepDelta != null || deliveryDelta != null) && (
        <div className="flex items-center gap-2 text-step-2xs font-medium mt-1">
          {confirmDelta != null && (
            <span className={`flex items-center gap-1 px-2 py-1 rounded-lg bg-status-pending/10 text-status-pending`} title={t('admin.confirm_time', 'Confirmation Time')}>
              <i className="ti ti-clock" style={{ fontSize: '0.7rem' }} />
              {t('admin.confirm_short', 'Confirm')}: {confirmDelta}m
            </span>
          )}
          {prepDelta != null && (
            <span className={`flex items-center gap-1 px-2 py-1 rounded-lg bg-status-scheduled/10 text-status-scheduled`} title={t('admin.prep_time', 'Preparation Time')}>
              <i className="ti ti-chef-hat" style={{ fontSize: '0.7rem' }} />
              {t('admin.prep_short', 'Prep')}: {prepDelta}m
            </span>
          )}
          {deliveryDelta != null && (
            <span className={`flex items-center gap-1 px-2 py-1 rounded-lg bg-status-delivered/10 text-status-delivered`} title={t('admin.delivery_time_hint', 'Delivery Time')}>
              <i className="ti ti-truck-delivery" style={{ fontSize: '0.7rem' }} />
              {t('admin.deliv_short', 'Deliv')}: {deliveryDelta}m
            </span>
          )}
        </div>
      )}

      {/* Signals (Anti-Fake) */}
      <div className="flex gap-2 text-xs">
        {order.signals?.otpVerified ? (
          <span className={`flex items-center gap-1 bg-status-delivered/10 text-status-delivered px-2 py-1 rounded-lg`}>
            <i className="ti ti-shield-check" style={{ fontSize: '0.7rem' }} />
            {t('admin.otp', 'OTP')}
          </span>
        ) : (
          <span className={`flex items-center gap-1 bg-status-pending/10 text-status-pending px-2 py-1 rounded-lg`}>
            <i className="ti ti-shield-x" style={{ fontSize: '0.7rem' }} />
            {t('admin.no_otp', 'No OTP')}
          </span>
        )}
        <span className={`flex items-center gap-1 px-2 py-1 rounded-lg ${order.signals && order.signals.reputationScore < 50 ? `bg-[var(--status-cancelled-light)] text-status-cancelled` : `bg-[var(--status-info-light)] text-semantic-info`}`}>
          <i className="ti ti-star" style={{ fontSize: '0.7rem' }} />
          {t('admin.rep', 'Rep')}: {order.signals?.reputationScore ?? t('admin.new', 'New')}
        </span>
      </div>

      {/* Details */}
      <div className="text-sm space-y-1 text-brand-text">
        {detailsPending ? (
          <div className="space-y-1.5 py-0.5" data-testid="order-details-pending" aria-label={t('admin.loading_details', 'Loading order details…')}>
            <div className="h-4 w-2/3 rounded shimmer" />
            <div className="h-3 w-2/5 rounded shimmer" />
          </div>
        ) : (
          <>
            {order.customerName && order.customerName !== t('common.unknown', 'Unknown') && <div><span className="text-brand-text-muted w-16 inline-block">{t('admin.client', 'Client:')}</span> {order.customerName}</div>}
            {order.customerPhone && <div><span className="text-brand-text-muted w-16 inline-block">{t('common.phone', 'Phone:')}</span> {maskPhone(order.customerPhone)}</div>}
            {order.deliveryAddress && <div><span className="text-brand-text-muted w-16 inline-block">{t('admin.to', 'To:')}</span> {order.deliveryAddress}</div>}
          </>
        )}
        {(() => { const n = order.items?.length || order.itemCount || 0; return (
        <div><span className="text-brand-text-muted w-16 inline-block">{t('admin.items', 'Items:')}</span> {n} {n === 1 ? t('admin.item_lower', 'item') : t('admin.items_lower', 'items')} (<PriceDisplay amount={order.total} />)</div>
        ); })()}
        {order.items && order.items.length > 0 && (
          <div className="ml-16 text-xs space-y-0.5" style={{ color: 'var(--brand-text-muted)' }}>
            {order.items.map((item: any, i: number) => (
              <div key={i} className="flex justify-between">
                <span>{item.name} ×{item.qty || item.quantity}</span>
                <span>{item.price ? <PriceDisplay amount={item.price * (item.qty || item.quantity)} /> : null}</span>
              </div>
            ))}
          </div>
        )}
        {order.courierName && <div><span className="text-brand-text-muted w-16 inline-block">{t('admin.courier', 'Courier:')}</span> {order.courierName}</div>}
        {(order as any).rating != null && (
          <div className="flex items-start gap-1" data-testid="order-rating">
            <span className="text-brand-text-muted w-16 inline-block shrink-0">{t('admin.rating', 'Rating:')}</span>
            <span>
              <span style={{ color: 'var(--brand-primary)' }} aria-label={`${(order as any).rating}/5`}>
                {'★'.repeat((order as any).rating)}{'☆'.repeat(5 - (order as any).rating)}
              </span>
              {(order as any).feedback && <span className="block text-xs italic" style={{ color: 'var(--brand-text-muted)' }}>“{(order as any).feedback}”</span>}
            </span>
          </div>
        )}
        {order.elapsedSeconds !== undefined && order.elapsedSeconds > 1800 && (
        <span data-dynamic className="text-semantic-danger font-bold ml-2">{t('admin.overdue', 'Overdue!')} ({Math.floor(order.elapsedSeconds / 60)} min)</span>
        )}
      </div>

      {/* Messages toggle + inline thread */}
      <div className="border-t border-brand-border pt-3">
        <button
          onClick={() => onToggleMessages?.(order.id)}
          className={`flex items-center gap-2 text-sm font-medium text-brand-primary hover:text-brand-primary-dark transition-colors`}
        >
          <i className="ti ti-message" />
          {showMessages ? t('admin.hide_messages', 'Hide Messages') : t('admin.show_messages', 'Show Messages')}
          {messages && messages.length > 0 && !showMessages && (
            <span className={`ml-auto text-xs bg-brand-primary text-white px-2 py-0.5 rounded-full`}>
              {messages.length}
            </span>
          )}
        </button>
        {showMessages && (
          <div className="mt-2">
            <MessageThread
              orderId={order.id}
              messages={messages ?? []}
              role="owner"
              currentStatus={order.status}
              onSend={(key, params) => onSendMessage?.(order.id, key, params ?? {})}
              onMarkRead={() => {}}
            />
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="pt-3 border-t border-brand-border flex gap-2 overflow-x-auto no-scrollbar" onClick={(e) => e.stopPropagation()}>
        {order.status === 'PENDING' && (
          <>
            <Button size="sm" data-testid="order-confirm" onClick={() => handleAction('CONFIRMED')} isLoading={loadingAction === 'CONFIRMED'}>{t('admin.accept', 'Accept')}</Button>
            <Button size="sm" variant="outline" onClick={() => handleAction('CANCELLED')} isLoading={loadingAction === 'CANCELLED'}>{t('common.reject', 'Reject')}</Button>
          </>
        )}
        {order.status === 'CONFIRMED' && (
          <Button size="sm" data-testid="order-prepare" onClick={() => handleAction('PREPARING')} isLoading={loadingAction === 'PREPARING'}>{t('admin.mark_preparing', 'Mark Preparing')}</Button>
        )}
        {order.status === 'PREPARING' && (
          <Button size="sm" data-testid="order-ready" onClick={() => handleAction('READY')} isLoading={loadingAction === 'READY'}>{t('admin.mark_ready', 'Mark Ready')}</Button>
        )}
        {order.status === 'READY' && (
          <Button size="sm" data-testid="order-assign" onClick={() => handleAction('IN_DELIVERY')} isLoading={loadingAction === 'IN_DELIVERY'}>{t('admin.assign_courier', 'Assign Courier')}</Button>
        )}
      </div>
    </div>
  );
});

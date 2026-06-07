import React, { useState } from 'react';
import { formatALL } from '@deliveryos/shared-types';
import { Button, useI18n } from '../../index.js';

// --- AdminShell ---
interface AdminShellProps {
  children: React.ReactNode;
  currentPath: string;
  onNavigate: (path: string) => void;
  onLogout?: () => void;
}
export function AdminShell({ children, currentPath, onNavigate, onLogout }: AdminShellProps) {
  const { t } = useI18n();
  const navItems = [
    { path: '/admin', label: t('admin.live_orders', 'Live Orders') },
    { path: '/admin/menu', label: t('admin.menu_manager', 'Menu Manager') },
    { path: '/admin/branding', label: t('admin.theme_settings', 'Theme Settings') },
  ];

  return (
    <div className="min-h-screen bg-[var(--brand-bg)] flex flex-col md:flex-row text-[var(--brand-text)]">
      {/* Desktop Sidebar / Mobile Topbar */}
      <div className="w-full md:w-64 bg-[var(--brand-surface)] border-b md:border-b-0 md:border-r border-[var(--brand-border)] p-4 flex flex-col">
        <h1 className="text-xl font-bold mb-6 text-[var(--brand-primary)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>
          DeliveryOS Admin
        </h1>
        <nav className="flex md:flex-col gap-2 overflow-x-auto md:overflow-visible flex-1">
          {navItems.map(item => (
            <button
              key={item.path}
              onClick={() => onNavigate(item.path)}
              className={`w-full text-left px-4 py-2 rounded-lg transition-colors font-medium text-sm ${currentPath === item.path ? 'bg-[var(--brand-primary-light)] text-[var(--brand-primary)]' : 'hover:bg-[var(--brand-surface-raised)]'}`}
            >
              {item.label}
            </button>
          ))}
        </nav>
        {onLogout && (
          <button onClick={onLogout} className="mt-auto px-4 py-2 text-sm text-[var(--color-danger)] hover:bg-[var(--color-danger-light)] rounded-lg transition-colors flex items-center gap-2">
            <i className="ti ti-logout" /> {t('admin.logout', 'Logout')}
          </button>
        )}
      </div>
      
      {/* Main Content */}
      <div className="flex-1 overflow-auto">
        {children}
      </div>
    </div>
  );
}

// --- Toggle ---
interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
}
export function Toggle({ checked, onChange, label }: ToggleProps) {
  return (
    <label className="flex items-center gap-3 cursor-pointer">
      <div className={`relative w-12 h-6 rounded-full transition-colors ${checked ? 'bg-[var(--brand-primary)]' : 'bg-[var(--brand-surface-raised)]'}`}>
        <div className={`absolute top-1 left-1 w-4 h-4 rounded-full bg-[var(--color-on-primary)] transition-transform ${checked ? 'translate-x-6' : 'translate-x-0'}`} />
      </div>
      {label && <span className="font-medium text-[var(--brand-text)]">{label}</span>}
    </label>
  );
}

// --- ColorInput ---
interface ColorInputProps {
  value: string;
  onChange: (value: string) => void;
  label: string;
}
export function ColorInput({ value, onChange, label }: ColorInputProps) {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-sm font-medium text-[var(--brand-text-muted)]">{label}</span>
      <div className="flex items-center gap-2">
        <div className="w-10 h-10 rounded-[var(--brand-radius-sm)] border border-[var(--brand-border)] overflow-hidden shrink-0">
          <input 
            type="color" 
            value={value} 
            onChange={e => onChange(e.target.value)} 
            className="w-full h-full p-0 border-none cursor-pointer scale-150" 
          />
        </div>
        <input 
          type="text" 
          value={value} 
          onChange={e => onChange(e.target.value)} 
          className="flex-1 bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius-sm)] px-3 py-2 text-[var(--brand-text)]"
        />
      </div>
    </div>
  );
}

// --- OrderCard ---
export interface AdminOrder {
  id: string;
  status: 'PENDING' | 'CONFIRMED' | 'PREPARING' | 'READY' | 'IN_DELIVERY' | 'DELIVERED' | 'CANCELLED';
  createdAt: string;
  confirmedAt?: string;
  readyAt?: string;
  deliveredAt?: string;
  items: { name: string; quantity: number }[];
  total: number;
  customerName?: string;
  customerPhone?: string;
  shortId?: string;
  itemCount?: number;
  itemsSummary?: string;
  etaMinutes?: number | null;
  elapsedSeconds?: number;
  courierName?: string | null;
  deliveryAddress?: string;
  signals?: {
    reputationScore: number;
    otpVerified: boolean;
  };
}

interface OrderCardProps {
  order: AdminOrder;
  onUpdateStatus: (id: string, newStatus: string) => Promise<void>;
  isLoading?: boolean;
}
export function OrderCard({ order, onUpdateStatus, isLoading }: OrderCardProps) {
  const { t } = useI18n();
  const [loadingAction, setLoadingAction] = useState('');

  const handleAction = async (status: string) => {
    setLoadingAction(status);
    await onUpdateStatus(order.id, status);
    setLoadingAction('');
  };

  const getStatusColor = (s: string) => {
    switch (s) {
      case 'PENDING': return 'bg-[var(--status-pending-bg)] text-[var(--status-pending)] border-[var(--status-pending-border)]';
      case 'PREPARING': return 'bg-[var(--status-preparing-bg)] text-[var(--status-preparing)] border-[var(--status-preparing-border)]';
      case 'READY': return 'bg-[var(--status-ready-bg)] text-[var(--status-ready)] border-[var(--status-ready-border)]';
      case 'IN_DELIVERY': return 'bg-[var(--status-in-delivery-bg)] text-[var(--status-in-delivery)] border-[var(--status-in-delivery-border)]';
      case 'DELIVERED': return 'bg-[var(--status-delivered-bg)] text-[var(--status-delivered)] border-[var(--status-delivered-border)]';
      case 'CANCELLED': return 'bg-[var(--status-cancelled-bg)] text-[var(--status-cancelled)] border-[var(--status-cancelled-border)]';
      default: return 'bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] border-[var(--brand-border)]';
    }
  };

  const getStatusIcon = (s: string) => {
    switch (s) {
      case 'PENDING': return 'ti ti-clock';
      case 'PREPARING': return 'ti ti-chef-hat';
      case 'READY': return 'ti ti-check';
      case 'IN_DELIVERY': return 'ti ti-truck-delivery';
      case 'DELIVERED': return 'ti ti-package';
      case 'CANCELLED': return 'ti ti-x';
      default: return 'ti ti-help';
    }
  };

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

  return (
    <div className={`bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4 flex flex-col gap-4 ${isLoading ? 'opacity-50 pointer-events-none' : ''}`}>
      
      {/* Header */}
      <div className="flex justify-between items-start">
        <div>
          <div className="font-bold text-lg text-[var(--brand-text)]">#{order.id.slice(-4).toUpperCase()}</div>
          <div className="text-[var(--brand-text-muted)] text-sm flex items-center gap-2">
            {new Date(order.createdAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
          </div>
        </div>
        <div className={`px-2.5 py-1 rounded-full text-xs font-bold border flex items-center gap-1.5 ${getStatusColor(order.status)}`}>
          <i className={getStatusIcon(order.status)} style={{ fontSize: '0.75rem' }} />
          {order.status}
        </div>
      </div>

      {/* Timeline Deltas */}
      {(confirmDelta != null || prepDelta != null || deliveryDelta != null) && (
        <div className="flex items-center gap-2 text-[11px] font-medium mt-1">
          {confirmDelta != null && (
            <span className="flex items-center gap-1 px-2 py-1 rounded-lg bg-[var(--status-pending-light)] text-[var(--status-pending)]" title={t('admin.confirm_time', 'Confirmation Time')}>
              <i className="ti ti-clock" style={{ fontSize: '0.7rem' }} />
              {t('admin.confirm_short', 'Confirm')}: {confirmDelta}m
            </span>
          )}
          {prepDelta != null && (
            <span className="flex items-center gap-1 px-2 py-1 rounded-lg bg-[var(--status-scheduled-light)] text-[var(--status-scheduled)]" title={t('admin.prep_time', 'Preparation Time')}>
              <i className="ti ti-chef-hat" style={{ fontSize: '0.7rem' }} />
              {t('admin.prep_short', 'Prep')}: {prepDelta}m
            </span>
          )}
          {deliveryDelta != null && (
            <span className="flex items-center gap-1 px-2 py-1 rounded-lg bg-[var(--status-delivered-light)] text-[var(--status-delivered)]" title={t('admin.delivery_time_hint', 'Delivery Time')}>
              <i className="ti ti-truck-delivery" style={{ fontSize: '0.7rem' }} />
              {t('admin.deliv_short', 'Deliv')}: {deliveryDelta}m
            </span>
          )}
        </div>
      )}

      {/* Signals (Anti-Fake) */}
      <div className="flex gap-2 text-xs">
        {order.signals?.otpVerified ? (
          <span className="flex items-center gap-1 bg-[var(--status-delivered-light)] text-[var(--status-delivered)] px-2 py-1 rounded-lg">
            <i className="ti ti-shield-check" style={{ fontSize: '0.7rem' }} />
            OTP
          </span>
        ) : (
          <span className="flex items-center gap-1 bg-[var(--status-pending-light)] text-[var(--status-pending)] px-2 py-1 rounded-lg">
            <i className="ti ti-shield-x" style={{ fontSize: '0.7rem' }} />
            {t('admin.no_otp', 'No OTP')}
          </span>
        )}
        <span className={`flex items-center gap-1 px-2 py-1 rounded-lg ${order.signals && order.signals.reputationScore < 50 ? 'bg-[var(--status-cancelled-light)] text-[var(--status-cancelled)]' : 'bg-[var(--status-info-light)] text-[var(--color-info)]'}`}>
          <i className="ti ti-star" style={{ fontSize: '0.7rem' }} />
          {t('admin.rep', 'Rep')}: {order.signals?.reputationScore ?? t('admin.new', 'New')}
        </span>
      </div>

      {/* Details */}
      <div className="text-sm space-y-1 text-[var(--brand-text)]">
        {order.customerName && order.customerName !== 'Unknown' && <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">{t('admin.client', 'Client:')}</span> {order.customerName}</div>}
        {order.customerPhone && <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">{t('common.phone', 'Phone:')}</span> {order.customerPhone}</div>}
        {order.deliveryAddress && <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">{t('admin.to', 'To:')}</span> {order.deliveryAddress}</div>}
        <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">{t('admin.items', 'Items:')}</span> {order.items?.length || 0} {t('admin.items_lower', 'items')} ({formatALL(order.total)})</div>
        {order.items && order.items.length > 0 && (
          <div className="ml-16 text-xs space-y-0.5" style={{ color: 'var(--brand-text-muted)' }}>
            {order.items.map((item: any, i: number) => (
              <div key={i} className="flex justify-between">
                <span>{item.name} ×{item.qty || item.quantity}</span>
                <span>{item.price ? formatALL(item.price * (item.qty || item.quantity)) : ''}</span>
              </div>
            ))}
          </div>
        )}
        {order.courierName && <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">{t('admin.courier', 'Courier:')}</span> {order.courierName}</div>}
        {order.elapsedSeconds !== undefined && order.elapsedSeconds > 1800 && (
        <span className="text-[var(--color-danger)] font-bold ml-2">{t('admin.overdue', 'Overdue!')} ({Math.floor(order.elapsedSeconds / 60)} min)</span>
        )}
      </div>

      {/* Actions */}
      <div className="mt-auto pt-4 border-t border-[var(--brand-border)] flex gap-2 overflow-x-auto no-scrollbar">
        {order.status === 'PENDING' && (
          <>
            <Button size="sm" onClick={() => handleAction('PREPARING')} isLoading={loadingAction === 'PREPARING'}>{t('admin.accept_prepare', 'Accept & Prepare')}</Button>
            <Button size="sm" variant="outline" onClick={() => handleAction('CANCELLED')} isLoading={loadingAction === 'CANCELLED'}>{t('common.reject', 'Reject')}</Button>
          </>
        )}
        {order.status === 'PREPARING' && (
          <Button size="sm" onClick={() => handleAction('READY')} isLoading={loadingAction === 'READY'}>{t('admin.mark_ready', 'Mark Ready')}</Button>
        )}
        {order.status === 'READY' && (
          <Button size="sm" onClick={() => handleAction('IN_DELIVERY')} isLoading={loadingAction === 'IN_DELIVERY'}>{t('admin.assign_courier', 'Assign Courier')}</Button>
        )}
      </div>
    </div>
  );
}

import React, { useState } from 'react';
import { formatALL } from '@deliveryos/shared-types';
import { Button } from '../../index.js';

// --- AdminShell ---
interface AdminShellProps {
  children: React.ReactNode;
  currentPath: string;
  onNavigate: (path: string) => void;
  onLogout?: () => void;
}
export function AdminShell({ children, currentPath, onNavigate, onLogout }: AdminShellProps) {
  const navItems = [
    { path: '/admin', label: 'Live Orders' },
    { path: '/admin/menu', label: 'Menu Manager' },
    { path: '/admin/branding', label: 'Theme Settings' },
  ];

  return (
    <div className="min-h-screen bg-[var(--brand-bg)] flex flex-col md:flex-row text-[var(--brand-text)]">
      {/* Desktop Sidebar / Mobile Topbar */}
      <div className="w-full md:w-64 bg-[var(--brand-surface)] border-b md:border-b-0 md:border-r border-[var(--brand-border)] p-4 flex flex-col">
        <h1 className="text-xl font-bold mb-6 text-[var(--brand-primary)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>
          DeliveryOS Admin
        </h1>
        <nav className="flex md:flex-col gap-2 overflow-x-auto md:overflow-visible flex-1">
          {navItems.map(item => {
            const isActive = currentPath === item.path || (item.path !== '/admin' && currentPath.startsWith(item.path));
            return (
              <button
                key={item.path}
                onClick={() => onNavigate(item.path)}
                className={`text-left px-4 py-2 rounded-[var(--brand-radius-btn)] font-medium transition-colors whitespace-nowrap ${isActive ? 'bg-[var(--brand-primary)] text-white' : 'hover:bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] hover:text-[var(--brand-text)]'}`}
              >
                {item.label}
              </button>
            );
          })}
        </nav>
        {onLogout && (
          <div className="hidden md:block mt-auto pt-4 border-t border-[var(--brand-border)]">
            <button onClick={onLogout} className="text-[var(--brand-text-muted)] hover:text-red-500 font-medium">Log out</button>
          </div>
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
        <div className={`absolute top-1 left-1 w-4 h-4 rounded-full bg-white transition-transform ${checked ? 'translate-x-6' : 'translate-x-0'}`} />
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
  const [loadingAction, setLoadingAction] = useState('');

  const handleAction = async (status: string) => {
    setLoadingAction(status);
    await onUpdateStatus(order.id, status);
    setLoadingAction('');
  };

  const getStatusColor = (s: string) => {
    switch (s) {
      case 'PENDING': return 'bg-yellow-500/20 text-yellow-500 border-yellow-500/30';
      case 'PREPARING': return 'bg-blue-500/20 text-blue-500 border-blue-500/30';
      case 'READY': return 'bg-purple-500/20 text-purple-500 border-purple-500/30';
      case 'IN_DELIVERY': return 'bg-orange-500/20 text-orange-500 border-orange-500/30';
      case 'DELIVERED': return 'bg-green-500/20 text-green-500 border-green-500/30';
      case 'CANCELLED': return 'bg-red-500/20 text-red-500 border-red-500/30';
      default: return 'bg-gray-500/20 text-gray-500 border-gray-500/30';
    }
  };

  return (
    <div className={`bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4 flex flex-col gap-4 ${isLoading ? 'opacity-50 pointer-events-none' : ''}`}>
      
      {/* Header */}
      <div className="flex justify-between items-start">
        <div>
          <div className="font-bold text-lg text-[var(--brand-text)]">#{order.id.slice(-4).toUpperCase()}</div>
          <div className="text-[var(--brand-text-muted)] text-sm">{new Date(order.createdAt).toLocaleTimeString()}</div>
        </div>
        <div className={`px-2 py-1 rounded-full text-xs font-bold border ${getStatusColor(order.status)}`}>
          {order.status}
        </div>
      </div>

      {/* Signals (Anti-Fake) */}
      <div className="flex gap-2 text-xs">
        {order.signals?.otpVerified ? (
          <span className="bg-green-500/10 text-green-500 px-2 py-1 rounded">OTP \u2713</span>
        ) : (
          <span className="bg-yellow-500/10 text-yellow-600 px-2 py-1 rounded">No OTP</span>
        )}
        <span className={`px-2 py-1 rounded ${order.signals && order.signals.reputationScore < 50 ? 'bg-red-500/10 text-red-500' : 'bg-blue-500/10 text-blue-500'}`}>
          Rep: {order.signals?.reputationScore ?? 'New'}
        </span>
      </div>

      {/* Details */}
      <div className="text-sm space-y-1 text-[var(--brand-text)]">
        {order.customerName && order.customerName !== 'Unknown' && <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">Client:</span> {order.customerName}</div>}
        {order.customerPhone && <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">Phone:</span> {order.customerPhone}</div>}
        {order.deliveryAddress && <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">To:</span> {order.deliveryAddress}</div>}
        <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">Items:</span> {order.items?.length || 0} items ({formatALL(order.total)})</div>
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
        {order.courierName && <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">Courier:</span> {order.courierName}</div>}
        {order.elapsedSeconds > 0 && (
          <div><span className="text-[var(--brand-text-muted)] w-16 inline-block">Time:</span> {Math.floor(order.elapsedSeconds / 60)}m ago</div>
        )}
      </div>

      {/* Actions */}
      <div className="mt-auto pt-4 border-t border-[var(--brand-border)] flex gap-2 overflow-x-auto no-scrollbar">
        {order.status === 'PENDING' && (
          <>
            <Button size="sm" onClick={() => handleAction('PREPARING')} isLoading={loadingAction === 'PREPARING'}>Accept & Prepare</Button>
            <Button size="sm" variant="outline" onClick={() => handleAction('CANCELLED')} isLoading={loadingAction === 'CANCELLED'}>Reject</Button>
          </>
        )}
        {order.status === 'PREPARING' && (
          <Button size="sm" onClick={() => handleAction('READY')} isLoading={loadingAction === 'READY'}>Mark Ready</Button>
        )}
        {order.status === 'READY' && (
          <Button size="sm" onClick={() => handleAction('IN_DELIVERY')} isLoading={loadingAction === 'IN_DELIVERY'}>Assign Courier</Button>
        )}
      </div>
    </div>
  );
}

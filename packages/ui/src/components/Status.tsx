import React, { Component } from 'react';
import type { ErrorInfo, ReactNode } from 'react';

// --- SkeletonBase ---
export function SkeletonBase({ className = '' }: { className?: string }) {
  return (
    <div className={`animate-pulse rounded bg-[var(--brand-surface-raised)] ${className}`} />
  );
}

// --- EmptyState ---
export function EmptyState({ title, description, icon, action }: { title: string; description: string; icon?: ReactNode; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center p-8 text-center rounded-[var(--brand-radius)] border border-dashed border-[var(--brand-border)] bg-[var(--brand-surface)]">
      {icon && <div className="mb-4 text-4xl text-[var(--brand-text-muted)]">{icon}</div>}
      <h3 className="mb-2 text-lg font-semibold text-[var(--brand-text)]">{title}</h3>
      <p className="mb-6 text-sm text-[var(--brand-text-muted)] max-w-sm">{description}</p>
      {action}
    </div>
  );
}

// --- OfflineBanner ---
export function OfflineBanner({ fallbackPhone }: { fallbackPhone?: string }) {
  return (
    <div className="fixed top-0 left-0 right-0 z-50 bg-[var(--color-warning)] text-[var(--brand-text)] px-4 py-2 text-sm font-medium text-center shadow-md">
      You are currently offline. Please check your internet connection.
      {fallbackPhone && (
        <span className="block mt-1">Need help? Call us: <a href={`tel:${fallbackPhone}`} className="underline font-bold">{fallbackPhone}</a></span>
      )}
    </div>
  );
}

// --- WSStatusDot ---
export function WSStatusDot({ status }: { status: 'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'error' }) {
  const colors = {
    connected: 'bg-[var(--color-success)]',
    connecting: 'bg-[var(--color-warning)] animate-pulse',
    reconnecting: 'bg-[var(--color-warning)] animate-pulse',
    disconnected: 'bg-[var(--brand-text-muted)]',
    error: 'bg-[var(--color-danger)]',
  };

  return (
    <div className="flex items-center gap-2" title={`WebSocket Status: ${status}`}>
      <span className="relative flex h-3 w-3">
        {(status === 'connecting' || status === 'reconnecting') && (
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--color-warning)] opacity-75"></span>
        )}
        <span className={`relative inline-flex rounded-full h-3 w-3 ${colors[status]}`}></span>
      </span>
    </div>
  );
}

// --- ErrorBoundary ---
interface ErrorBoundaryProps {
  children: ReactNode;
  fallbackPhone?: string;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('ErrorBoundary caught an error:', error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-[50vh] flex flex-col items-center justify-center p-6 text-center">
          <i className="ti ti-alert-triangle text-4xl mb-4" style={{ color: 'var(--color-warning)' }} />
          <h2 className="text-xl font-bold text-[var(--color-danger)] mb-2">Something went wrong</h2>
          <p className="text-[var(--brand-text-muted)] mb-6 max-w-md">
            We encountered an unexpected error. Don't worry, your cart is safe.
          </p>
          {this.props.fallbackPhone && (
            <div className="p-4 bg-[var(--brand-surface-raised)] rounded-[var(--brand-radius-sm)] border border-[var(--brand-border)]">
              <p className="text-sm text-[var(--brand-text)] mb-2">To place an order immediately, please call:</p>
              <a href={`tel:${this.props.fallbackPhone}`} className="text-lg font-bold text-[var(--brand-primary)] hover:underline">
                {this.props.fallbackPhone}
              </a>
            </div>
          )}
          <button 
            onClick={() => window.location.reload()} 
            className="mt-6 px-6 py-2 bg-[var(--brand-primary)] text-white rounded-full font-medium hover:bg-[var(--brand-primary-hover)] transition-colors"
          >
            Reload Page
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}

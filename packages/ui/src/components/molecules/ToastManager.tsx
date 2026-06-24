import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

type ToastVariant = 'success' | 'error' | 'warning' | 'info';

interface Toast {
  id: string;
  message: string;
  variant: ToastVariant;
}

interface ToastContextValue {
  showToast: (message: string, variant?: ToastVariant) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

const variantConfig: Record<ToastVariant, { bg: string; icon: string }> = {
  success: { bg: 'var(--color-success)', icon: 'ti-circle-check' },
  error: { bg: 'var(--color-danger)', icon: 'ti-alert-circle' },
  warning: { bg: 'var(--color-warning)', icon: 'ti-alert-triangle' },
  info: { bg: 'var(--brand-primary)', icon: 'ti-info-circle' },
};

function ToastItem({ toast, onDone }: { toast: Toast; onDone: (id: string) => void }) {
  const config = variantConfig[toast.variant];
  return (
    <div
      className="flex items-center gap-2.5 px-4 py-3 rounded-[var(--brand-radius)] text-sm font-medium text-white max-w-sm animate-toast-in motion-reduce:animate-none"
      role="alert"
      style={{ background: config.bg, boxShadow: 'var(--elev-3)' }}
    >
      <i className={`ti ${config.icon} shrink-0 text-base leading-none`} aria-hidden="true" />
      <span className="flex-1 min-w-0">{toast.message}</span>
      <button
        onClick={() => onDone(toast.id)}
        className="shrink-0 w-6 h-6 flex items-center justify-center rounded-full text-white/80 hover:text-white hover:bg-white/15 transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white/70"
        aria-label="Dismiss"
      >
        <i className="ti ti-x text-sm leading-none" aria-hidden="true" />
      </button>
    </div>
  );
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const showToast = useCallback((message: string, variant: ToastVariant = 'info') => {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
    setToasts((prev) => [...prev, { id, message, variant }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 4000);
  }, []);

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      {typeof document !== 'undefined' &&
        // Top-center, offset below the 56px brand header + notch, so toasts
        // never overlap the header controls (lang/currency switchers).
        createPortal(
          <div
            className="fixed left-1/2 -translate-x-1/2 z-toast flex flex-col items-center gap-2 pointer-events-none w-full max-w-sm px-4"
            style={{ top: 'calc(env(safe-area-inset-top, 0px) + 4rem)' }}
          >
            {toasts.map((t) => (
              <div key={t.id} className="pointer-events-auto">
                <ToastItem toast={t} onDone={removeToast} />
              </div>
            ))}
          </div>,
          document.body,
        )}
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error('useToast must be used within ToastProvider');
  return ctx;
}

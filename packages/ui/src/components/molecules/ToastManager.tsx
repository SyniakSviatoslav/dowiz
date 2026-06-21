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

const variantStyles: Record<ToastVariant, string> = {
  success: 'bg-semantic-success text-white',
  error: 'bg-semantic-danger text-white',
  warning: 'bg-semantic-warning text-white',
  info: 'bg-brand-primary text-white',
};

function ToastItem({ toast, onDone }: { toast: Toast; onDone: (id: string) => void }) {
  return (
    <div
      className={`${variantStyles[toast.variant]} px-4 py-3 rounded-lg shadow-elevation-3 text-sm font-medium animate-toast-in max-w-sm`}
      role="alert"
    >
      <div className="flex items-center gap-2">
        <span className="flex-1">{toast.message}</span>
        <button
          onClick={() => onDone(toast.id)}
          className="text-white/80 hover:text-white transition-colors shrink-0"
          aria-label="Dismiss"
        >
          <i className="ti ti-x" />
        </button>
      </div>
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

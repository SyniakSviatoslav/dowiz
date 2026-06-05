import React, { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';

interface OverlayProps {
  isOpen: boolean;
  onClose: () => void;
  children: React.ReactNode;
  title?: string;
}

function Portal({ children }: { children: React.ReactNode }) {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  if (!mounted) return null;
  return createPortal(children, document.body);
}

function Backdrop({ onClick, children }: { onClick: () => void, children: React.ReactNode }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 transition-opacity">
      <div className="absolute inset-0" onClick={onClick} />
      {children}
    </div>
  );
}

// --- Modal ---
export function Modal({ isOpen, onClose, title, children }: OverlayProps) {
  useEffect(() => {
    if (!isOpen) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };
    document.addEventListener('keydown', onKey);
    return () => {
      document.body.style.overflow = prev;
      document.removeEventListener('keydown', onKey);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;
  return (
    <Portal>
      <Backdrop onClick={onClose}>
        <div role="dialog" aria-modal="true" className="relative z-10 w-full max-w-md rounded-[var(--brand-radius)] bg-[var(--brand-bg)] p-6 shadow-xl ring-1 ring-[var(--brand-border)]">
          {title && <h2 className="mb-4 text-xl font-semibold text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>{title}</h2>}
          <button onClick={onClose} className="absolute right-4 top-4 text-[var(--brand-text-muted)] hover:text-[var(--brand-text)] transition-colors" aria-label="Close">
            <i className="ti ti-x" />
          </button>
          {children}
        </div>
      </Backdrop>
    </Portal>
  );
}

// --- Drawer ---
export function Drawer({ isOpen, onClose, title, children }: OverlayProps) {
  useEffect(() => {
    if (!isOpen) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };
    document.addEventListener('keydown', onKey);
    return () => {
      document.body.style.overflow = prev;
      document.removeEventListener('keydown', onKey);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;
  return (
    <Portal>
      <div className="fixed inset-0 z-50 flex justify-end bg-black/50 transition-opacity">
        <div className="absolute inset-0" onClick={onClose} />
        <div role="dialog" aria-modal="true" className="relative z-10 h-full w-full max-w-md bg-[var(--brand-bg)] shadow-2xl overflow-y-auto transform transition-transform">
          <div className="sticky top-0 flex items-center justify-between border-b border-[var(--brand-border)] bg-[var(--brand-bg)] px-6 py-4">
            {title && <h2 className="text-xl font-semibold text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>{title}</h2>}
            <button onClick={onClose} className="text-[var(--brand-text-muted)] hover:text-[var(--brand-text)] transition-colors w-8 h-8 flex items-center justify-center rounded-full hover:bg-[var(--brand-surface-raised)]" aria-label="Close">
              <i className="ti ti-x" />
            </button>
          </div>
          <div className="p-6">
            {children}
          </div>
        </div>
      </div>
    </Portal>
  );
}

// --- BottomSheet ---
export function BottomSheet({ isOpen, onClose, title, children }: OverlayProps) {
  useEffect(() => {
    if (!isOpen) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose(); };
    document.addEventListener('keydown', onKey);
    return () => {
      document.body.style.overflow = prev;
      document.removeEventListener('keydown', onKey);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;
  return (
    <Portal>
      <div className="fixed inset-0 z-50 flex items-end justify-center bg-black/50 transition-opacity">
        <div className="absolute inset-0" onClick={onClose} />
        <div role="dialog" aria-modal="true" className="relative z-10 w-full max-w-lg rounded-t-[var(--brand-radius)] bg-[var(--brand-bg)] shadow-2xl">
          <div className="flex w-full justify-center pt-3 pb-1" onClick={onClose}>
            <div className="h-1.5 w-12 rounded-full bg-[var(--brand-border)]" />
          </div>
          <div className="px-6 py-4">
            {title && <h2 className="mb-4 text-xl font-semibold text-[var(--brand-text)] text-center" style={{ fontFamily: 'var(--brand-font-heading)' }}>{title}</h2>}
            {children}
          </div>
        </div>
      </div>
    </Portal>
  );
}

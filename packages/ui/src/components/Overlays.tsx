import React, { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useI18n } from '../lib/I18nProvider.js';

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

// Flips true one frame after mount so the panel transitions IN from its start
// state. Exit is instant (the consumer unmounts on isOpen=false) — acceptable,
// and the enter is the part the eye reads. Reduced-motion is honored because the
// transition durations use --motion-* tokens, which the global reduced-motion
// rule zeroes to 0ms.
function useEntered(isOpen: boolean) {
  const [entered, setEntered] = useState(false);
  useEffect(() => {
    if (!isOpen) { setEntered(false); return; }
    const id = requestAnimationFrame(() => setEntered(true));
    return () => cancelAnimationFrame(id);
  }, [isOpen]);
  return entered;
}

// Shared scroll-lock + Esc-to-close. Returns nothing; mirrors prior behavior.
function useOverlayChrome(isOpen: boolean, onClose: () => void) {
  useEffect(() => {
    if (!isOpen) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    const onKey = (e: KeyboardEvent) => { if (e.key === `Escape`) onClose(); };
    document.addEventListener('keydown', onKey);
    return () => {
      document.body.style.overflow = prev;
      document.removeEventListener('keydown', onKey);
    };
  }, [isOpen, onClose]);
}

const CLOSE_BTN =
  `inline-flex items-center justify-center rounded-full text-[var(--brand-text-muted)] hover:text-[var(--brand-text)] hover:bg-[var(--brand-surface-raised)] transition-colors duration-150 ease-[var(--ease-soft)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-brand-bg motion-reduce:transition-none`;

function ScrimClose({ onClose }: { onClose: () => void }) {
  const { t } = useI18n();
  return (
    <div
      className="absolute inset-0"
      role="button"
      tabIndex={0}
      aria-label={t('common.close', 'Close')}
      onClick={onClose}
      onKeyDown={(e) => { if (e.key === `Enter` || e.key === ' ') { e.preventDefault(); onClose(); } }}
    />
  );
}

// --- Modal ---
export function Modal({ isOpen, onClose, title, children }: OverlayProps) {
  useOverlayChrome(isOpen, onClose);
  const entered = useEntered(isOpen);
  const { t } = useI18n();
  if (!isOpen) return null;
  return (
    <Portal>
      <div
        className={`fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4 transition-opacity duration-[var(--motion-base)] ease-[var(--ease-out)] motion-reduce:transition-none`}
        style={{ opacity: entered ? 1 : 0 }}
      >
        <ScrimClose onClose={onClose} />
        <div
          role="dialog"
          aria-modal="true"
          className={`relative z-10 w-full max-w-md rounded-lg bg-brand-bg p-6 ring-1 ring-brand-border transition-[opacity,transform] duration-[var(--motion-base)] ease-[var(--ease-out)] motion-reduce:transition-none`}
          style={{ boxShadow: `boxShadow`, opacity: entered ? 1 : 0, transform: entered ? 'translateY(0) scale(1)' : 'translateY(8px) scale(0.98)' }}
        >
          {title && <h2 className="mb-4 text-xl font-semibold text-brand-text" style={{ fontFamily: `fontFamily` }}>{title}</h2>}
          <button onClick={onClose} className={`absolute right-4 top-4 w-9 h-9 ${CLOSE_BTN}`} aria-label={t('common.close', 'Close')}>
            <i className="ti ti-x" />
          </button>
          {children}
        </div>
      </div>
    </Portal>
  );
}

// --- Drawer ---
export function Drawer({ isOpen, onClose, title, children }: OverlayProps) {
  useOverlayChrome(isOpen, onClose);
  const entered = useEntered(isOpen);
  const { t } = useI18n();
  if (!isOpen) return null;
  return (
    <Portal>
      <div
        className={`fixed inset-0 z-50 flex justify-end bg-black/50 backdrop-blur-sm transition-opacity duration-[var(--motion-base)] ease-[var(--ease-out)] motion-reduce:transition-none`}
        style={{ opacity: entered ? 1 : 0 }}
      >
        <ScrimClose onClose={onClose} />
        <div
          role="dialog"
          aria-modal="true"
          className={`relative z-10 h-full w-full max-w-md bg-brand-bg overflow-y-auto transition-transform duration-[var(--motion-base)] ease-[var(--ease-out)] motion-reduce:transition-none`}
          style={{ boxShadow: `boxShadow`, transform: entered ? 'translateX(0)' : 'translateX(100%)' }}
        >
          <div className={`sticky top-0 z-10 flex items-center justify-between border-b border-brand-border bg-brand-bg px-6 py-4`}>
            {title && <h2 className="text-xl font-semibold text-brand-text" style={{ fontFamily: `fontFamily` }}>{title}</h2>}
            <button onClick={onClose} className={`w-9 h-9 ${CLOSE_BTN}`} aria-label={t('common.close', 'Close')}>
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
  useOverlayChrome(isOpen, onClose);
  const entered = useEntered(isOpen);
  const { t } = useI18n();
  if (!isOpen) return null;
  return (
    <Portal>
      <div
        className={`fixed inset-0 z-50 flex items-end justify-center bg-black/50 backdrop-blur-sm transition-opacity duration-[var(--motion-base)] ease-[var(--ease-out)] motion-reduce:transition-none`}
        style={{ opacity: entered ? 1 : 0 }}
      >
        <ScrimClose onClose={onClose} />
        <div
          role="dialog"
          aria-modal="true"
          className={`relative z-10 w-full max-w-lg rounded-t-lg bg-brand-bg transition-transform duration-[var(--motion-base)] ease-[var(--ease-out)] motion-reduce:transition-none`}
          style={{ boxShadow: `boxShadow`, transform: entered ? 'translateY(0)' : 'translateY(100%)', paddingBottom: 'max(0px, env(safe-area-inset-bottom))' }}
        >
          <div
            className="flex w-full justify-center pt-3 pb-1 cursor-pointer"
            role="button"
            tabIndex={0}
            aria-label={t('common.close', 'Close')}
            onClick={onClose}
            onKeyDown={(e) => { if (e.key === `Enter` || e.key === ' ') { e.preventDefault(); onClose(); } }}
          >
            <div className="h-1.5 w-12 rounded-full bg-brand-border" />
          </div>
          <div className="px-6 py-4">
            {title && <h2 className="mb-4 text-xl font-semibold text-brand-text text-center" style={{ fontFamily: `fontFamily` }}>{title}</h2>}
            {children}
          </div>
        </div>
      </div>
    </Portal>
  );
}

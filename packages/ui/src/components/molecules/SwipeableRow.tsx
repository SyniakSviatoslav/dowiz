import { useRef, useState, type ReactNode, type TouchEvent, type MouseEvent } from 'react';

interface SwipeAction {
  label: string;
  icon?: string;
  onClick: () => void;
  className?: string;
}

interface SwipeableRowProps {
  children: ReactNode;
  actions: SwipeAction[];
  threshold?: number;
  className?: string;
}

export function SwipeableRow({ children, actions, threshold = 80, className = '' }: SwipeableRowProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [offsetX, setOffsetX] = useState(0);
  const [isDragging, setIsDragging] = useState(false);
  const startX = useRef(0);
  const currentX = useRef(0);
  const actionWidth = actions.length * 72;

  const handleTouchStart = (e: TouchEvent) => {
    const touch = e.touches?.[0];
    if (!touch) return;
    startX.current = touch.clientX;
    currentX.current = touch.clientX;
    setIsDragging(true);
  };

  const handleTouchMove = (e: TouchEvent) => {
    if (!isDragging) return;
    const touch = e.touches?.[0];
    if (!touch) return;
    currentX.current = touch.clientX;
    const diff = startX.current - currentX.current;
    const clamped = Math.min(Math.max(0, diff), actionWidth);
    setOffsetX(clamped);
  };

  const handleTouchEnd = () => {
    setIsDragging(false);
    if (offsetX > threshold) {
      setOffsetX(actionWidth);
    } else {
      setOffsetX(0);
    }
  };

  const handleMouseDown = (e: MouseEvent) => {
    startX.current = e.clientX;
    currentX.current = e.clientX;
    setIsDragging(true);
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (!isDragging) return;
    currentX.current = e.clientX;
    const diff = startX.current - currentX.current;
    const clamped = Math.min(Math.max(0, diff), actionWidth);
    setOffsetX(clamped);
  };

  const handleMouseUp = () => {
    setIsDragging(false);
    if (offsetX > threshold) {
      setOffsetX(actionWidth);
    } else {
      setOffsetX(0);
    }
  };

  return (
    <div ref={containerRef} className={`relative overflow-hidden ${className}`}>
      <div
        className="absolute inset-y-0 right-0 flex items-stretch"
        style={{ width: `${actionWidth}px` }}
      >
        {actions.map((action, i) => (
          <button
            key={i}
            onClick={() => { setOffsetX(0); action.onClick(); }}
            className={`flex flex-col items-center justify-center gap-1 px-3 text-xs font-semibold text-white transition-colors ${action.className || 'bg-[var(--brand-primary)]'}`}
            style={{ minWidth: '64px', minHeight: '44px' }}
          >
            {action.icon && <i className={action.icon} />}
            {action.label}
          </button>
        ))}
      </div>
      <div
        className="relative bg-[var(--brand-surface)] transition-transform duration-200 select-none"
        style={{ transform: `translateX(-${offsetX}px)`, touchAction: 'pan-y' }}
        role="presentation"
        onTouchStart={handleTouchStart}
        onTouchMove={handleTouchMove}
        onTouchEnd={handleTouchEnd}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={() => { if (isDragging) { setIsDragging(false); setOffsetX(0); }}}
      >
        {children}
      </div>
    </div>
  );
}

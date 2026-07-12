import { useState, useRef, useCallback, type ReactNode } from 'react';

interface TooltipProps {
  content: string;
  position?: Position;
  children: ReactNode;
}

type Position = 'top' | 'bottom' | 'left' | 'right';

const positionMap: Record<Position, { container: string; arrow: string }> = {
top: {
  container: `bottom-full left-1/2 -translate-x-1/2 mb-2`,
  arrow: `-bottom-1 left-1/2 -translate-x-1/2`,
},
bottom: {
  container: `top-full left-1/2 -translate-x-1/2 mt-2`,
  arrow: `-top-1 left-1/2 -translate-x-1/2`,
},
left: {
  container: `right-full top-1/2 -translate-y-1/2 mr-2`,
  arrow: `-right-1 top-1/2 -translate-y-1/2`,
},
right: {
  container: `left-full top-1/2 -translate-y-1/2 ml-2`,
  arrow: `-left-1 top-1/2 -translate-y-1/2`,
},
};

export function Tooltip({ content, position = 'top', children }: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const timerRef = useRef<number>();

  const show = useCallback(() => {
    clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(() => setVisible(true), 300);
  }, []);

  const hide = useCallback(() => {
    clearTimeout(timerRef.current);
    setVisible(false);
  }, []);

  const pos = positionMap[position];

  return (
    <div className="relative inline-flex" onMouseEnter={show} onMouseLeave={hide}>
      {children}
      <div
        className={`absolute z-dropdown ${pos.container} transition-all duration-200 pointer-events-none ${
          visible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-1'
        }`}
      >
        <div className={`relative bg-brand-surface-raised border border-brand-border text-brand-text px-3 py-1.5 rounded-md text-xs max-w-[200px] whitespace-normal shadow-elevation-2`}>
          {content}
        </div>
        <div
          className={`absolute w-2.5 h-2.5 bg-brand-surface-raised border border-brand-border rotate-45 ${pos.arrow}`}
          style={{ clipPath: `clipPath` }}
        />
      </div>
    </div>
  );
}

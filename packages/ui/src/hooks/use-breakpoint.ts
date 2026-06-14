import { useState, useEffect } from 'react';

const BP_TABLET = 768;
const BP_DESKTOP = 1280;

export function useBreakpoint(): { isMobile: boolean; isTablet: boolean; isDesktop: boolean; width: number } {
  const [width, setWidth] = useState(() => {
    if (typeof window !== 'undefined') return window.innerWidth;
    return BP_DESKTOP;
  });

  useEffect(() => {
    let timeout: ReturnType<typeof setTimeout>;
    const handleResize = () => {
      clearTimeout(timeout);
      timeout = setTimeout(() => setWidth(window.innerWidth), 100);
    };
    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('resize', handleResize);
      clearTimeout(timeout);
    };
  }, []);

  return {
    isMobile: width < BP_TABLET,
    isTablet: width >= BP_TABLET && width < BP_DESKTOP,
    isDesktop: width >= BP_DESKTOP,
    width,
  };
}

export function useIsMobile(): boolean {
  return useBreakpoint().isMobile;
}

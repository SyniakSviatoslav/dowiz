import { createContext, useContext, useState, useCallback, type ReactNode } from 'react';

interface SoundPrefsContextValue {
  alertSoundEnabled: boolean;
  toggleAlertSound: () => void;
}

const SoundPrefsContext = createContext<SoundPrefsContextValue | null>(null);

export function SoundPrefsProvider({ children }: { children: ReactNode }) {
  const [alertSoundEnabled, setAlertSoundEnabled] = useState(() => {
    if (typeof window === 'undefined') return true;
    return localStorage.getItem('dos_alert_sound') !== 'off';
  });

  const toggleAlertSound = useCallback(() => {
    setAlertSoundEnabled(prev => {
      const next = !prev;
      localStorage.setItem('dos_alert_sound', next ? 'on' : 'off');
      return next;
    });
  }, []);

  return (
    <SoundPrefsContext.Provider value={{ alertSoundEnabled, toggleAlertSound }}>
      {children}
    </SoundPrefsContext.Provider>
  );
}

export function useSoundPrefs(): SoundPrefsContextValue {
  const ctx = useContext(SoundPrefsContext);
  if (!ctx) return { alertSoundEnabled: true, toggleAlertSound: () => {} };
  return ctx;
}

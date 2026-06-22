import { useEffect, useState } from 'react';
import { useI18n } from '../../lib/I18nProvider.js';
import { isSunlightOn, setSunlight } from '../../utils/sunlight.js';

// Header button to toggle Sunlight Mode (high-contrast outdoor theme). Reflects the
// persisted/OS-derived state and flips html[data-sunlight] live.
export function SunlightToggle() {
  const { t } = useI18n();
  const [on, setOn] = useState(false);

  useEffect(() => { setOn(isSunlightOn()); }, []);

  const toggle = () => {
    const next = !on;
    setOn(next);
    setSunlight(next);
  };

  return (
    <button
      type="button"
      onClick={toggle}
      aria-pressed={on}
      aria-label={t('common.sunlight_mode', 'Sunlight mode — high contrast for bright sun')}
      title={t('common.sunlight_mode', 'Sunlight mode — high contrast for bright sun')}
      data-testid="sunlight-toggle"
      data-sunlight-on={on}
      className="flex items-center justify-center w-9 h-9 rounded-lg transition-colors hover:bg-[var(--brand-surface-raised)]"
      style={{ color: on ? 'var(--brand-primary)' : 'var(--brand-text-muted)', border: '1px solid var(--brand-border)', background: 'var(--brand-surface)' }}
    >
      <i className={on ? 'ti ti-sun-filled' : 'ti ti-sun'} style={{ fontSize: '1rem' }} aria-hidden="true" />
    </button>
  );
}

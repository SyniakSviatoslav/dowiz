// The persistent "Voice" on/off preference toggle (docs/design/voice-control/ui-spec.md §5) — a
// small control for the storefront menu controls/preferences row. Rendered only when the render
// predicate passes (flag on + not killed + WebGPU-capable) — otherwise there is nothing to toggle
// (consistent with "absent, not greyed"); that gating happens at the mount site, same as MicFab.

import { useI18n } from '../lib/I18nProvider.js';

export interface VoiceSettingToggleProps {
  enabled: boolean;
  onChange: (enabled: boolean) => void;
  className?: string;
}

export function VoiceSettingToggle({ enabled, onChange, className = '' }: VoiceSettingToggleProps) {
  const { t } = useI18n();
  return (
    <button
      type="button"
      role="switch"
      aria-checked={enabled}
      data-testid="voice-setting-toggle"
      onClick={() => onChange(!enabled)}
      className={`inline-flex items-center gap-2 rounded-full px-3 text-sm font-medium transition-[background-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 ${className}`}
      style={{
        minHeight: 'var(--tap-min)',
        background: enabled ? 'var(--brand-primary)' : 'var(--brand-surface-raised)',
        color: enabled ? 'var(--color-on-primary)' : 'var(--brand-text)',
        border: enabled ? 'none' : '1px solid var(--brand-border)',
      }}
    >
      <i className="ti ti-microphone" aria-hidden="true" />
      {t('voice.setting_label')}
    </button>
  );
}

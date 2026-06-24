import type { Config } from 'tailwindcss';

const config: Config = {
  content: ['./src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        brand: {
          primary: 'var(--brand-primary)',
          'primary-hover': 'var(--brand-primary-hover)',
          'primary-light': 'var(--brand-primary-light)',
          'primary-strong': 'var(--brand-primary-strong)',
          accent: 'var(--brand-accent)',
          bg: 'var(--brand-bg)',
          surface: 'var(--brand-surface)',
          'surface-raised': 'var(--brand-surface-raised)',
          text: 'var(--brand-text)',
          'text-muted': 'var(--brand-text-muted)',
          border: 'var(--brand-border)',
        },
        status: {
          pending: 'var(--status-pending)',
          confirmed: 'var(--status-confirmed)',
          preparing: 'var(--status-preparing)',
          ready: 'var(--status-ready)',
          'in-delivery': 'var(--status-in-delivery)',
          delivered: 'var(--status-delivered)',
          rejected: 'var(--status-rejected)',
          cancelled: 'var(--status-cancelled)',
          scheduled: 'var(--status-scheduled)',
          'picked-up': 'var(--status-picked-up)',
        },
        semantic: {
          success: 'var(--color-success)',
          warning: 'var(--color-warning)',
          danger: 'var(--color-danger)',
          info: 'var(--color-info)',
          'success-light': 'var(--color-success-light)',
          'warning-light': 'var(--color-warning-light)',
          'danger-light': 'var(--color-danger-light)',
          'info-light': 'var(--color-info-light)',
        },
      },
      fontFamily: {
        heading: 'var(--brand-font-heading)',
        body: 'var(--brand-font-body)',
        // Editorial display face (paper skin maps --font-display → Fraunces).
        display: 'var(--font-display, var(--brand-font-heading))',
      },
      // Namespaced modular type scale. Intentionally NOT keyed xs/sm/base/... —
      // that would override Tailwind's stock text-* and resize 500+ existing
      // usages. `text-step-*` is additive; migrate incrementally.
      // See docs/operating-model/typography-scale.md.
      fontSize: {
        'step-xs': ['var(--text-xs)', { lineHeight: 'var(--leading-normal)' }],
        'step-sm': ['var(--text-sm)', { lineHeight: 'var(--leading-normal)' }],
        'step-base': ['var(--text-base)', { lineHeight: 'var(--leading-normal)' }],
        'step-lg': ['var(--text-lg)', { lineHeight: 'var(--leading-snug)' }],
        'step-xl': ['var(--text-xl)', { lineHeight: 'var(--leading-snug)' }],
        'step-2xl': ['var(--text-2xl)', { lineHeight: 'var(--leading-tight)' }],
        'step-3xl': ['var(--text-3xl)', { lineHeight: 'var(--leading-tight)' }],
      },
      // Namespaced too: Tailwind's stock leading-tight/snug/relaxed have
      // different values (1.25/1.375/1.625) and are used in ~15 places —
      // overriding them would shift those. `leading-step-*` is additive.
      lineHeight: {
        'step-tight': 'var(--leading-tight)',
        'step-snug': 'var(--leading-snug)',
        'step-normal': 'var(--leading-normal)',
        'step-relaxed': 'var(--leading-relaxed)',
      },
      fontWeight: {
        normal: 'var(--weight-normal)',
        medium: 'var(--weight-medium)',
        semibold: 'var(--weight-semibold)',
        bold: 'var(--weight-bold)',
      },
      borderRadius: {
        none: 'var(--radius-none)',
        sm: 'var(--radius-sm)',
        md: 'var(--radius-md)',
        lg: 'var(--radius-lg)',
        xl: 'var(--radius-xl)',
        '2xl': 'var(--radius-2xl)',
        full: 'var(--radius-full)',
      },
      spacing: {
        '0': 'var(--space-0)',
        '1': 'var(--space-1)',
        '2': 'var(--space-2)',
        '3': 'var(--space-3)',
        '4': 'var(--space-4)',
        '5': 'var(--space-5)',
        '6': 'var(--space-6)',
        '8': 'var(--space-8)',
        '10': 'var(--space-10)',
        '12': 'var(--space-12)',
        '16': 'var(--space-16)',
      },
      boxShadow: {
        'elevation-1': 'var(--elevation-1)',
        'elevation-2': 'var(--elevation-2)',
        'elevation-3': 'var(--elevation-3)',
        'elevation-4': 'var(--elevation-4)',
      },
      zIndex: {
        dropdown: 'var(--z-dropdown)',
        sticky: 'var(--z-sticky)',
        'modal-backdrop': 'var(--z-modal-backdrop)',
        modal: 'var(--z-modal)',
        toast: 'var(--z-toast)',
      },
      keyframes: {
        'slide-up': {
          '0%': { transform: 'translateY(100%)' },
          '100%': { transform: 'translateY(0)' },
        },
        'toast-in': {
          '0%': { transform: 'translateX(100%)', opacity: '0' },
          '100%': { transform: 'translateX(0)', opacity: '1' },
        },
      },
      animation: {
        'slide-up': 'slide-up 0.3s ease-out',
        'toast-in': 'toast-in 0.3s ease-out',
      },
    },
  },
  plugins: [],
};

export default config;

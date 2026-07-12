interface IconProps {
  name: string;
  size?: number | string;
  className?: string;
  stroke?: number;
}

const SIZE_MAP: Record<string, string> = {
  'sm': '1rem',
  'md': '1.25rem',
  'lg': '1.5rem',
  'xl': '2rem',
};

export function Icon({ name, size = 'md', className = '', stroke }: IconProps) {
  const resolvedSize = SIZE_MAP[size] ?? size;
  return (
    <i
      className={`ti ti-${name} ${className}`}
      style={{ fontSize: resolvedSize, ...(stroke !== undefined ? { '--ti-stroke': stroke } as React.CSSProperties : {}) }}
    />
  );
}

export const ICONS = {
  HOME: 'home',
  SETTINGS: 'settings',
  USER: 'user',
  SEARCH: 'search',
  PLUS: 'plus',
  MINUS: 'minus',
  X: 'x',
  CHECK: 'check',
  CHEVRON_LEFT: 'chevron-left',
  CHEVRON_RIGHT: 'chevron-right',
  CHEVRON_DOWN: 'chevron-down',
  CHEVRON_UP: 'chevron-up',
  MENU: 'menu-2',
  BELL: 'bell',
  STAR: 'star',
  STAR_FILLED: 'star-filled',
  HEART: 'heart',
  SHOPPING_CART: 'shopping-cart',
  MAP_PIN: 'map-pin',
  PHONE: 'phone',
  MAIL: 'mail',
  CLOCK: 'clock',
  TRASH: 'trash',
  EDIT: 'edit',
  LOGOUT: 'logout',
  INFO_CIRCLE: 'info-circle',
  ALERT_TRIANGLE: 'alert-triangle',
  FILTER: 'filter',
  DOTS_VERTICAL: 'dots-vertical',
  DOTS: 'dots',
  ARROW_LEFT: 'arrow-left',
  ARROW_RIGHT: 'arrow-right',
  RELOAD: 'reload',
  COPY: 'copy',
  LINK: 'link',
  EXTERNAL_LINK: 'external-link',
  EYE: 'eye',
  EYE_OFF: 'eye-off',
  LOCK: 'lock',
  UNLOCK: 'unlock',
  UPLOAD: 'upload',
  DOWNLOAD: 'download',
  CALENDAR: 'calendar',
  TAG: 'tag',
  GLOBE: 'world',
  MOON: 'moon',
  SUN: 'sun',
} as const;

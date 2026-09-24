// Buttons. One geometry; the VARIANT carries meaning, the SIZE carries context.
//
//   variant  primary   the one main action on a screen (there is only one)
//            success   an action that completes something (deliver, confirm)
//            secondary a real action that is not the main one
//            ghost     low emphasis: back, cancel, secondary navigation
//            danger    irreversible or destructive
//   size     md (44px, the --tap floor) · lg (56px, a courier's thumb) · sm is
//            refused on purpose: nothing tappable goes under --tap.
//
// `href` renders an <a> with the same look (a link that looks like a button is
// still a link to assistive technology). `busy` renders the spinner and sets
// aria-busy + disabled, so a double tap cannot send twice.
import { esc, cx, attrs, merge, icon, label, i18nAttr } from './core.js';

export const VARIANTS = ['primary', 'success', 'secondary', 'ghost', 'danger'];
const SIZES = ['md', 'lg'];

function check(variant, size){
  if (!VARIANTS.includes(variant)) throw new Error(`ui.button: unknown variant ${variant}`);
  if (!SIZES.includes(size)) throw new Error(`ui.button: unknown size ${size} (nothing under --tap)`);
}

/// @param {object} o
///   label, icon, iconEnd, variant='secondary', size='md', block=false,
///   id, type='button', disabled, busy, busyLabel, pressed, ariaLabel, href,
///   target, cls, attrs
export function button(o = {}){
  const { variant = 'secondary', size = 'md', block = false, busy = false } = o;
  check(variant, size);
  const cls = cx('ui-btn', `ui-btn--${variant}`, size !== 'md' && `ui-btn--${size}`,
                 block && 'ui-btn--block', !o.label && 'ui-btn--icon', o.cls);
  const inner = busy
    ? `${icon('loader-2', 'ui-spin')}${label(o.busyLabel ?? o.label)}`
    : `${icon(o.icon)}${label(o.label)}${icon(o.iconEnd)}`;
  const common = merge(
    { id: o.id, class: cls },
    i18nAttr('aria-label', o.ariaLabel),
    i18nAttr('title', o.title),
    o.pressed == null ? null : { 'aria-pressed': String(!!o.pressed) },
    o.attrs,
  );
  if (o.href) {
    const a = merge(common, { href: o.href, target: o.target,
      rel: o.target === '_blank' ? 'noopener' : o.rel,
      'aria-disabled': o.disabled ? 'true' : null });
    return `<a${attrs(a)}>${inner}</a>`;
  }
  const b = merge(common, { type: o.type || 'button', disabled: !!(o.disabled || busy),
                            'aria-busy': busy ? 'true' : null });
  return `<button${attrs(b)}>${inner}</button>`;
}

/// A round, icon-only control. `ariaLabel` is REQUIRED: an icon with no name is
/// a button a screen reader announces as "button".
export function iconButton(o = {}){
  if (!o.ariaLabel) throw new Error('ui.iconButton: ariaLabel is required');
  const a = merge(
    { id: o.id, type: o.type || 'button', class: cx('ui-iconbtn', o.variant === 'plain' && 'ui-iconbtn--plain', o.cls),
      disabled: !!o.disabled },
    i18nAttr('aria-label', o.ariaLabel),
    i18nAttr('title', o.title ?? o.ariaLabel),
    o.pressed == null ? null : { 'aria-pressed': String(!!o.pressed) },
    o.attrs,
  );
  return `<button${attrs(a)}>${o.text ? esc(o.text) : icon(o.icon)}</button>`;
}

/// Put a live button into its busy state and return the function that undoes
/// it. For the tap that is in flight: the label says what is happening, the
/// control cannot be pressed twice, and a failure restores it exactly.
export function setBusy(btn, busyText){
  if (!btn) return () => {};
  const had = btn.innerHTML, was = btn.disabled;
  btn.disabled = true;
  btn.setAttribute('aria-busy', 'true');
  btn.innerHTML = `${icon('loader-2', 'ui-spin')}<span>${esc(busyText ?? '')}</span>`;
  return () => {
    btn.disabled = was;
    btn.removeAttribute('aria-busy');
    btn.innerHTML = had;
  };
}

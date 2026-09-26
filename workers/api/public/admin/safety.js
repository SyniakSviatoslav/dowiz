// DATA & SAFETY (W-WIRE row 8): the owner's own copy of the venue, and the
// four repairs that used to be a curl in a runbook. Each is idempotent on the
// hub's side, each asks first, and each shows the hub's own answer.
//   Download a copy      GET  /api/owner/backup
//   Restore a copy       POST /api/owner/restore        (into an EMPTY venue)
//   Re-apply erasures    POST /api/owner/customers/reforget
//   Repair customer keys POST /api/owner/customers/rekey
//   Archive old orders   POST /api/owner/hub/rotate     (the nightly job, now)
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $, esc, icon, t, api, post, busy, confirm, store } from '/admin/core.js';
import { btn, input, rowDiv } from '/admin/parts.js';
import { q, fail, open, paint } from '/admin/wire-core.js';

/// The five actions: id, icon, words, and whether it may lose anything.
export const ACTIONS = [
  { id: 'backup', icon: 'download', key: 'w_backup', hint: 'w_backupHint' },
  { id: 'restore', icon: 'upload', key: 'w_restore', hint: 'w_restoreHint', danger: true },
  { id: 'reforget', icon: 'refresh', key: 'w_reforget', hint: 'w_reforgetHint', path: '/owner/customers/reforget' },
  { id: 'rekey', icon: 'key', key: 'w_rekey', hint: 'w_rekeyHint', path: '/owner/customers/rekey' },
  { id: 'rotate', icon: 'history', key: 'w_rotate', hint: 'w_rotateHint', path: '/owner/hub/rotate' },
];

/// The hub's answer, said in one line.
export const said = r => (r == null ? '' : typeof r === 'string' ? r : JSON.stringify(r).slice(0, 400));

async function download(){
  const r = await fetch('/api/owner/backup' + q(), { headers: { authorization: 'Bearer ' + store.t } });
  if (!r.ok) throw new Error('HTTP ' + r.status);
  const a = document.createElement('a');
  a.href = URL.createObjectURL(await r.blob());
  a.download = `dowiz-backup-${store.loc}.json`;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 10_000);
}

export function mount(host, last = null){
  if (!host) return;
  host.innerHTML = ACTIONS.map(a => rowDiv({ leading: icon(a.icon), title: { t: a.key }, sub: `<span data-t="${a.hint}"></span>`,
    trailing: a.id === 'restore' ? input({ type: 'file', id: 'w-restore', accept: 'application/json,.json', key: 'w_chooseFile' })
      : btn({ id: 'w-' + a.id, variant: a.danger ? 'danger' : 'secondary', icon: a.icon, key: 'w_run' }) })).join('')
    + (last ? `<p class="eyebrow mt-3" data-t="w_answer"></p><pre class="mono small" id="wSaid">${esc(last)}</pre>` : '');
  paint(host);
  $('#w-backup').onclick = () => busy($('#w-backup'), download).catch(fail);
  for (const a of ACTIONS.filter(x => x.path)) $('#w-' + a.id).onclick = async () => {
    const ok = await confirm(t(a.key), t(a.hint));
    if (!ok) return openSafety();
    let r = null;
    try { r = await post(a.path + q(), {}); } catch (e) { r = String(e.message || e); }
    openSafety(said(r));
  };
  $('#w-restore').onchange = async e => {
    const f = e.target.files && e.target.files[0]; if (!f) return;
    let bundle;
    try { bundle = JSON.parse(await f.text()); } catch { return fail(t('w_notABackup')); }
    const ok = await confirm(t('w_restore'), t('w_restoreQ'), { danger: true });
    if (!ok) return openSafety();
    let r = null;
    try { r = await post('/owner/restore' + q(), bundle); } catch (err) { r = String(err.message || err); }
    openSafety(said(r));
  };
}

export const openSafety = (last = null) => mount(open('w_safety', 'w_safetyHint', 'safety'), last);

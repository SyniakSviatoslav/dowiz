// The Published-menu card (W-PUBUI, over BN2): what the venue's object has put
// on the CDN for the storefront, and a button to publish now. Drawing is
// publish-view.js (pure, tested); this file is the fetch and the tap.
//
// The venue travels as ?location_id= (the hub refuses unknown body fields),
// and the answer is the object's own record, relayed by /api/owner/publish
// (services/venue/publish.rs). A 503 from "publish now" is the object's own
// sentence -- publishing is off until the CDN bucket is attached -- and is
// shown as it comes.
//
// ASCII QUOTES ONLY in this file.
import { $, t, api, post, toast, sheet, store, busy, retranslate } from '/admin/core.js';
import { page, cdnOrigin } from '/admin/publish-view.js';
import '/admin/publish-i18n.js';

const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const fail = e => toast(String((e && e.message) || e));

const load = () => api('/owner/publish' + q());

export async function open(){
  let d;
  try { d = await load(); } catch (e) { return fail(e); }
  draw(d);
}

function draw(d){
  sheet(page(d, cdnOrigin(location)), { name: 'publish', keepScroll: true });
  retranslate();
  const b = $('#pubNow');
  if (!b) return;
  b.onclick = async () => {
    try {
      const r = await busy(b, () => post('/owner/publish' + q(), {}));
      toast(`${t('pub_written')}: ${r.written}`);
      draw(await load());
    } catch (e) {
      // The object's 503 is English; the owner reads the same fact in their language.
      if (e && e.status === 503) toast(t('pub_offHint')); else fail(e);
    }
  };
}

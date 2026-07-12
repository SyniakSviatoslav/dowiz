import { chromium } from '@playwright/test';
const BASE='https://dowiz-staging.fly.dev', OWNER=process.env.OWNER_TOKEN, OUT='audit/iter2-validate';
const b=await chromium.launch(); const c=await b.newContext({viewport:{width:1280,height:900}});
await c.addInitScript(t=>localStorage.setItem('dos_access_token',t),OWNER);
const p=await c.newPage();
const errs=[]; p.on('console',m=>{if(m.type()==='error')errs.push(m.text())}); p.on('pageerror',e=>errs.push('pe '+e.message));
await p.goto(BASE+'/admin/menu',{waitUntil:'networkidle',timeout:60000}); await p.waitForTimeout(2500);

// 1) RESTORE: if Cola shows "I stopuar", toggle it back to available
const colaState = await p.evaluate(()=>{
  for(const e of document.querySelectorAll('div')){const tx=e.textContent||'';if(/Cola/.test(tx)&&/200 ALL/.test(tx)&&tx.length<300){
    return {stopped:/stopuar/i.test(tx)};}} return {stopped:null};
});
if(colaState.stopped===true){
  await p.evaluate(()=>{for(const e of document.querySelectorAll('div')){const tx=e.textContent||'';if(/Cola/.test(tx)&&/200 ALL/.test(tx)&&tx.length<300){const sw=e.querySelector('button.rounded-full');if(sw)sw.click();break;}}});
  await p.waitForTimeout(1200);
}
const restored = await p.evaluate(()=>{for(const e of document.querySelectorAll('div')){const tx=e.textContent||'';if(/Cola/.test(tx)&&/200 ALL/.test(tx)&&tx.length<300)return /disponuesh/i.test(tx)&&!/stopuar/i.test(tx);}return null;});

// 2) OPEN editor modal via the pencil (svg button that is NOT the toggle and NOT delete)
const opened = await p.evaluate(()=>{
  for(const e of document.querySelectorAll('div')){const tx=e.textContent||'';if(/Cola/.test(tx)&&/200 ALL/.test(tx)&&tx.length<300){
    const bs=[...e.querySelectorAll('button')];
    // toggle is button.rounded-full; the other svg buttons are [edit, delete] in order -> click edit (first non-toggle svg btn)
    const acts=bs.filter(x=>!/rounded-full/.test((x.className||'').toString()) && x.querySelector('svg'));
    if(acts.length){acts[0].click();return 'clicked-edit n='+acts.length;}
    return 'no-edit';
  }}return 'no-card';
});
await p.waitForTimeout(1500);

// detect modal: role=dialog OR any fixed full-screen overlay
const modal = await p.evaluate(()=>{
  let d=document.querySelector('[role="dialog"]');
  if(!d){const fx=[...document.querySelectorAll('*')].filter(e=>{const cs=getComputedStyle(e);return cs.position==='fixed'&&e.offsetHeight>300&&e.offsetWidth>300&&parseInt(cs.zIndex||0)>=20;});d=fx[0];}
  if(!d)return {present:false};
  const cs=getComputedStyle(d);
  return {present:true, role:d.getAttribute('role'), anim:cs.animationName, transDur:cs.transitionDuration, cls:(d.className||'').toString().slice(0,80)};
});
await p.screenshot({path:`${OUT}/05b-admin-modal.png`});
// focus first button in modal for focus-visible check
let closeInfo='none';
if(modal.present){
  await p.evaluate(()=>{const d=document.querySelector('[role="dialog"]')||[...document.querySelectorAll('*')].find(e=>getComputedStyle(e).position==='fixed'&&e.offsetHeight>300);const btn=d&&d.querySelector('button');if(btn)btn.focus();});
  closeInfo=await p.evaluate(()=>{const el=document.activeElement;if(!el)return'na';const cs=getComputedStyle(el);return`tag=${el.tagName} aria=${el.getAttribute('aria-label')} outline=${cs.outlineWidth}/${cs.outlineStyle} ring=${(cs.boxShadow||'').slice(0,60)}`;});
  await p.screenshot({path:`${OUT}/05c-modal-close-focus.png`});
}
console.log(JSON.stringify({colaWasStopped:colaState.stopped,restored,opened,modal,closeInfo,errs:errs.slice(0,8)},null,2));
await b.close();
